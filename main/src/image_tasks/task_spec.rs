use std::any::TypeId;
use fn_graph::{DataAccessDyn, FnGraphBuilder, FnId, TypeIds};
use resman::{FnRes, IntoFnRes, IntoFnResource, Resources};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::future::Future;
use futures::future::{BoxFuture, Shared, WeakShared};
use std::ops::{Deref, Mul};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, RwLock, Weak};
use std::any::Any;
use std::pin::pin;
use std::ptr::DynMetadata;
use anyhow::Error;
use anyhow::anyhow;
use async_std::stream;
use chashmap_next::CHashMap;
use cached::lazy_static::lazy_static;
use cached::once_cell::sync::Lazy;
use fn_meta::{FnMetadata, FnMetadataExt};
use futures::{FutureExt, TryStreamExt};
use ordered_float::OrderedFloat;
use tiny_skia::Pixmap;
use weak_table::WeakKeyHashMap;
use async_std::stream::ExactSizeStream;
use async_std::stream::from_iter;
use async_std::stream::StreamExt;
use crate::image_tasks::from_svg::from_svg;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::png_output::png_output;
use crate::image_tasks::repaint::{AlphaChannel, to_alpha_channel};
use crate::image_tasks::animate::animate;
use crate::image_tasks::repaint::paint;
use crate::image_tasks::stack::{stack_layer_on_background, stack_layer_on_layer};
use crate::image_tasks::task_spec::TaskSpec::{FromSvg, PngOutput};

/// Specification of a task that produces and/or consumes at least one [Pixmap]. Created
/// to de-duplicate copies of the same task, since function closures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum TaskSpec {
    None {},
    Animate {background: Box<TaskSpec>, frames: Vec<Box<TaskSpec>>},
    FromSvg {source: PathBuf},
    MakeSemitransparent {base: Box<TaskSpec>, alpha: OrderedFloat<f32>},
    PngOutput {base: Box<TaskSpec>, destinations: Vec<PathBuf>},
    Repaint {base: Box<TaskSpec>, color: ComparableColor},
    StackLayerOnColor {background: ComparableColor, foreground: Box<TaskSpec>},
    StackLayerOnLayer {background: Box<TaskSpec>, foreground: Box<TaskSpec>},
    ToAlphaChannel {base: Box<TaskSpec>}
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CloneableError {
    message: String
}

impl From<Error> for CloneableError {
    fn from(value: Error) -> Self {
        return CloneableError {message: value.to_string()}
    }
}

impl Display for CloneableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.message)
    }
}

pub type CloneableResult<T> = Result<T, CloneableError>;
pub type SharedResultFuture<T> = Shared<BoxFuture<'static, dyn Future<Output=CloneableResult<T>>>>;
type FutureMap<T> = WeakKeyHashMap<Weak<TaskSpec>,SharedResultFuture<T>>;


static PIXMAP_FUTURES: Lazy<FutureMap<Pixmap>> = Lazy::new(FutureMap::new);
static ALPHA_FUTURES: Lazy<FutureMap<AlphaChannel>> = Lazy::new(FutureMap::new);
static EMPTY_FUTURES: Lazy<FutureMap<()>> = Lazy::new(FutureMap::new);

const PIXMAP: TypeId = TypeId::of::<Pixmap>();

macro_rules! checked_downcast {
    ($arg:expr, $type:ty) => {
        $arg.ok_or(anyhow::anyhow!("Missing input"))?.downcast_ref::<dyn std::future::Future<Output = $type>>().ok_or(anyhow::anyhow!("Wrong type of input"))?
    }
}

macro_rules! checked_get {
    ($map:expr, $key:expr) => {
        ($map).get($key.as_ref()).ok_or(crate::image_tasks::task_spec::anyhoo!("Missing future for prerequisite task"))?
    }
}

#[macro_export]
macro_rules! anyhoo {
    ($($args:expr),+) => {
        crate::image_tasks::task_spec::CloneableError::from(anyhow::anyhow!($($args),+))
    }
}

impl TaskSpec {
    fn get_pixmap_future(&self) -> CloneableResult<&SharedResultFuture<Pixmap>> {
        match self {
            TaskSpec::None { .. } => {
                Err(anyhoo!("Call to get_pixmap_future() on a None task {}", self))
            },
            TaskSpec::ToAlphaChannel { .. } => {
                Err(anyhoo!("Call to get_pixmap_future() on a ToAlphaChannel task {}", self))
            },
            PngOutput { .. } => {
                Err(anyhoo!("Call to get_pixmap_future() on a PngOutput task {}", self))
            },
            _ => {
                PIXMAP_FUTURES.deref().get(self).ok_or(anyhoo!("Missing future for {}", self))
            }
        }
    }
    fn get_alpha_channel_future(&self) -> CloneableResult<&SharedResultFuture<AlphaChannel>> {
        match self {
            TaskSpec::ToAlphaChannel { .. } => {
                ALPHA_FUTURES.deref().get(self).ok_or(anyhoo!("Missing future for {}", self))
            },
            _ => {
                Err(anyhoo!("Call to get_alpha_channel_future() on a non-ToAlphaChannel task {}", self))
            }
        }
    }
    fn get_empty_future(&self) -> CloneableResult<&SharedResultFuture<()>> {
        match self {
            PngOutput { .. } => {
                EMPTY_FUTURES.deref().get(self).ok_or(anyhoo!("Missing future for {}", self))
            },
            _ => {
                Err(anyhoo!("Call to get_empty_future() on a non-PngOutput task {}", self))
            }
        }
    }

    fn get_future(&self) -> CloneableResult<&SharedResultFuture<()>> {
        match self {
            PngOutput { .. } => {
                self.get_empty_future()
            }
            TaskSpec::ToAlphaChannel { .. } => {
                let future = self.get_alpha_channel_future()?;
                async { future.await }
            }
            _ => {
                let future = self.get_pixmap_future()?;
                async { future.await }
            }
        }
    }

    fn register(&self, width: u32) -> CloneableResult<()> {
        match self {
            TaskSpec::None { .. } => {
                Err(anyhoo!("Call to register() on a None task"))
            }
            TaskSpec::Animate { background, frames } => {
                if PIXMAP_FUTURES.contains_key(self) {
                    Ok(())
                }
                background.register(width)?;
                let background = background.get_pixmap_future()?;
                for frame in frames {
                    frame.register(width)?;
                }
                let mut frame_futures = vec![];
                for frame in frames {
                    frame_futures.push(frame.get_pixmap_future()?);
                }
                let pixmap_future: SharedResultFuture<Pixmap> = async {
                    animate(background, frame_futures)
                }.shared();
                PIXMAP_FUTURES.deref().insert(Arc::new(self.to_owned()), pixmap_future);
                Ok(())
            }
            FromSvg { source } => {
                if PIXMAP_FUTURES.contains_key(self) {
                    Ok(())
                }
                let pixmap_future: SharedResultFuture<Pixmap> = async {
                    from_svg(source, width)
                }.shared();
                PIXMAP_FUTURES.deref().insert(Arc::new(self.to_owned()), pixmap_future);
                Ok(())
            }
            TaskSpec::MakeSemitransparent { base, alpha } => {
                if PIXMAP_FUTURES.contains_key(self) {
                    Ok(())
                }
                let base = base.get_pixmap_future()?;
                let pixmap_future: SharedResultFuture<Pixmap> = async {
                    make_semitransparent(base.await?, alpha.0)
                }.shared();
                PIXMAP_FUTURES.deref().insert(Arc::new(self.to_owned()), pixmap_future);
                Ok(())
            }
            PngOutput { base, destinations } => {
                if EMPTY_FUTURES.contains_key(self) {
                    Ok(())
                }
                let base = base.get_pixmap_future()?;
                let output_future: SharedResultFuture<()> = async {
                    png_output(base.await?, destinations)
                }.shared();
                EMPTY_FUTURES.deref().insert(Arc::new(self.to_owned()), output_future);
                Ok(())
            }
            TaskSpec::Repaint { base, color } => {
                if PIXMAP_FUTURES.contains_key(self) {
                    Ok(())
                }
                let base = base.get_alpha_channel_future()?;
                let pixmap_future: SharedResultFuture<Pixmap> = async {
                    paint(base.await?, color.0)
                }.shared();
                PIXMAP_FUTURES.deref().insert(Arc::new(self.to_owned()), pixmap_future);
                Ok(())
            }
            TaskSpec::StackLayerOnColor { background, foreground } => {
                if PIXMAP_FUTURES.contains_key(self) {
                    Ok(())
                }
                let foreground = foreground.get_pixmap_future()?;
                let pixmap_future: SharedResultFuture<Pixmap> = async {
                    stack_layer_on_background(background, foreground.await)
                }.shared();
                PIXMAP_FUTURES.deref().insert(Arc::new(self.to_owned()), pixmap_future);
                Ok(())
            }
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                if PIXMAP_FUTURES.contains_key(self) {
                    Ok(())
                }
                let background = background.get_pixmap_future()?;
                let foreground = foreground.get_pixmap_future()?;
                let pixmap_future: SharedResultFuture<Pixmap> = async {
                    stack_layer_on_layer(background.await, foreground.await)
                }.shared();
                PIXMAP_FUTURES.deref().insert(Arc::new(self.to_owned()), pixmap_future);
                Ok(())
            }
            TaskSpec::ToAlphaChannel { base } => {
                if ALPHA_FUTURES.contains_key(self) {
                    Ok(())
                }
                let base = base.get_pixmap_future()?;
                let alpha_future: SharedResultFuture<AlphaChannel> = async {
                    to_alpha_channel(base.await)
                }.shared();
                ALPHA_FUTURES.deref().insert(Arc::new(self.to_owned()), alpha_future);
                Ok(())
            }
        }
    }
}

impl Display for TaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskSpec::Animate { background, frames: _frames } => {
                write!(f, "Animate({};", background)
            }
            FromSvg { source } => {
                write!(f, "{}", source.to_string_lossy())
            }
            TaskSpec::MakeSemitransparent { base, alpha } => {
                write!(f, "{}@{}", base, alpha)
            }
            PngOutput { base: _base, destinations } => {
                write!(f, "{}", destinations.iter().map({ |dest|
                    match dest.file_name() {
                        None => "Unknown PNG file",
                        Some(name) => name.to_str().unwrap()
                    }
                }).collect::<Vec<&str>>().as_slice().join(","))
            }
            TaskSpec::Repaint { base, color } => {
                write!(f, "{}@{}", base, color)
            }
            TaskSpec::StackLayerOnColor { background, foreground } => {
                write!(f, "{{{};{}}}", background, foreground.to_string())
            }
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                write!(f, "{{{};{}}}", background.to_string(), foreground.to_string())
            }
            TaskSpec::ToAlphaChannel { base } => {
                write!(f, "Alpha({})", base)
            }
            TaskSpec::None {} => {
                write!(f, "None")
            }
        }
    }
}

const MAP_TYPE: TypeId = TypeId::of::<CHashMap<&TaskSpec, Box<dyn Future<Output = dyn Any + Send + Sync>>>>();

trait TaskSpecFnMetadata: DataAccessDyn {}

impl DataAccessDyn for &TaskSpec {
    fn borrows(&self) -> TypeIds {
        TypeIds::new()
    }

    fn borrow_muts(&self) -> TypeIds {
        TypeIds::from_vec(vec![MAP_TYPE])
    }
}

impl TaskSpec {
    pub fn add_to(&'static self,
                  graph: &mut FnGraphBuilder<&TaskSpec>,
                  existing_nodes: &mut HashMap<&TaskSpec, FnId>,
                  tile_width: u32) -> FnId
    {
        if existing_nodes.contains_key(self) {
            return *existing_nodes.get(self).unwrap();
        }
        let self_id: FnId = match self {
            TaskSpec::Animate { background, frames } => {
                let background_id = background.add_to(graph, existing_nodes, tile_width);
                let mut frame_ids: Vec<FnId> = vec![];
                for frame in frames {
                    frame_ids.push(frame.add_to(graph, existing_nodes, tile_width));
                }
                let animate_id = graph.add_fn(self);
                graph.add_edge(background_id, animate_id).expect("Failed to add background edge");
                frame_ids.into_iter().for_each(|frame_id| {
                    graph.add_edge(frame_id, animate_id).expect("Failed to add frame edge");
                });
                animate_id
            },
            FromSvg { .. } => {
                graph.add_fn(self)
            },
            TaskSpec::MakeSemitransparent { base, .. } => {
                let base_id = base.add_to(graph, existing_nodes, tile_width);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            PngOutput { base, .. } => {
                let base_id = base.add_to(graph, existing_nodes, tile_width);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::Repaint { base, .. } => {
                let base_id = base.add_to(graph, existing_nodes, tile_width);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::StackLayerOnColor { foreground, .. } => {
                let base_id = foreground.add_to(graph, existing_nodes, tile_width);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                let background_id = background.add_to(graph, existing_nodes, tile_width);
                let foreground_id = foreground.add_to(graph, existing_nodes, tile_width);
                let self_id = graph.add_fn(self);
                graph.add_edge(background_id, self_id).expect("Failed to add background edge");
                graph.add_edge(foreground_id, self_id).expect("Failed to add foreground edge");
                self_id
            },
            TaskSpec::ToAlphaChannel { base } => {
                let base_id = base.add_to(graph, existing_nodes, tile_width);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::None {} => {
                panic!("Attempted to add a None task to graph");
            }
        };
        existing_nodes.insert(self, self_id);
        return self_id;
    }
}

lazy_static! {
    static ref OUT_DIR: &'static Path = Path::new("./out/");
    static ref SVG_DIR: &'static Path = Path::new("./svg/");
}

pub fn name_to_out_path(name: &str) -> PathBuf {
    return OUT_DIR.with_file_name(format!("{}.png", name)).as_path().into();
}

pub fn name_to_svg_path(name: &str) -> PathBuf {
    return SVG_DIR.with_file_name(format!("{}.svg", name)).as_path().into();
}

pub fn from_svg_task(name: &str) -> TaskSpec {
    return FromSvg {source: name_to_svg_path(name)};
}

pub fn repaint_task(base: TaskSpec, color: ComparableColor) -> TaskSpec {
    return TaskSpec::Repaint {base: Box::from(TaskSpec::ToAlphaChannel { base: Box::new(base) }), color};
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> TaskSpec {
    return repaint_task(from_svg_task(name), color);
}

pub fn semitrans_svg_task(name: &str, alpha: f32) -> TaskSpec {
    return TaskSpec::MakeSemitransparent {base: Box::new(from_svg_task(name)),
            alpha: alpha.into()};
}

pub fn path(name: &str) -> Vec<PathBuf> {
    return vec![name_to_out_path(name)];
}

pub fn out_task(name: &str, base: TaskSpec) -> TaskSpec {
    return PngOutput {base: Box::new(base), destinations: path(name)};
}

#[macro_export]
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        crate::image_tasks::task_spec::TaskSpec::StackLayerOnLayer {
            background: Box::new($first_layer.to_owned()),
            foreground: Box::new($second_layer.to_owned())
        }
    };
    ( $first_layer:expr, $second_layer:expr, $( $more_layers:expr ),+ ) => {{
        let mut layers_so_far = crate::stack!($first_layer, $second_layer);
        $( layers_so_far = crate::stack!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

#[macro_export]
macro_rules! stack_on {
    ( $background:expr, $foreground:expr ) => {
        crate::image_tasks::task_spec::TaskSpec::StackLayerOnColor {
            background: $background,
            foreground: Box::new($foreground)
        }
    };
    ( $background:expr, $first_layer:expr, $( $more_layers:expr ),+ ) => {{
        let mut layers_so_far = crate::stack_on!($background, $first_layer);
        $( layers_so_far = crate::stack!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

#[macro_export]
macro_rules! paint_stack {
    ( $color:expr, $( $layers:expr ),* ) => {
        crate::image_tasks::task_spec::repaint_task(
            crate::stack!($(crate::image_tasks::task_spec::from_svg_task($layers)),*),
            $color.to_owned())
    }
}

impl FromStr for TaskSpec {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(FromSvg {
            source: name_to_svg_path(s)
        })
    }
}

impl Mul<f32> for TaskSpec {
    type Output = TaskSpec;

    fn mul(self, rhs: f32) -> Self::Output {
        TaskSpec::MakeSemitransparent {
            base: self.into(),
            alpha: OrderedFloat::from(rhs)
        }
    }
}

impl Mul<ComparableColor> for TaskSpec {
    type Output = TaskSpec;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        let clone = self.to_owned();
        return match self {
            TaskSpec::ToAlphaChannel { base: _base } => {
                TaskSpec::Repaint {
                    base: Box::new(clone),
                    color: rhs
                }
            },
            _ => TaskSpec::Repaint {
                base: Box::new(TaskSpec::ToAlphaChannel { base: self.into() }),
                color: rhs
            }
        };
    }
}