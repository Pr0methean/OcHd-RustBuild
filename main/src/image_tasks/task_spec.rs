use std::collections::{HashMap};

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

use std::ops::{Deref, DerefMut, Mul};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use anyhow::{Error};

use cached::lazy_static::lazy_static;
use crate::anyhoo;
use fn_graph::{DataAccessDyn, TypeIds};
use fn_graph::daggy::Dag;
use itertools::{Itertools};


use log::{error, info};
use ordered_float::OrderedFloat;
use petgraph::graph::{IndexType, NodeIndex};
use replace_with::replace_with_and_return;

use tiny_skia::{Mask, MaskType, Pixmap};

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg};
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::png_output::{copy_out_to_out, png_output};
use crate::image_tasks::repaint::paint;
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_alpha_on_background, stack_layer_on_background, stack_layer_on_layer};
use crate::TILE_SIZE;

pub trait TaskSpecTraits <T>: Clone + Debug + Display + Ord + Eq + Hash {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<T>)
        where Ix : IndexType, E: Default, 'b: 'a;
}

impl TaskSpecTraits<Pixmap> for ToPixmapTaskSpec {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<Pixmap>)
                         where Ix : IndexType, E: Default, 'b: 'a {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future)) = ctx.pixmap_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        let (dependencies, function): (Vec<NodeIndex<Ix>>, LazyTaskFunction<Pixmap>) = match self {
            ToPixmapTaskSpec::None { .. } => panic!("Tried to add None task to graph"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let (background_index, background_future) = background.add_to(ctx);
                let mut dependencies = Vec::with_capacity(frames.len() + 1);
                dependencies.push(background_index);
                let mut frame_futures: Vec<CloneableLazyTask<Pixmap>> = Vec::with_capacity(frames.len());
                for frame in frames {
                    let (frame_index, frame_future) = frame.add_to(ctx);
                    frame_futures.push(frame_future);
                    dependencies.push(frame_index);
                }
                (dependencies, Box::new(move || {
                    let background: Arc<Box<Pixmap>> = background_future.into_result()?;
                    animate(&background, frame_futures)
                }))
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source = source.to_owned();
                (vec![], Box::new(move || {
                    Ok(Box::new(from_svg(&source, *TILE_SIZE)?))
                }))
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                if *background == ComparableColor::TRANSPARENT {
                    return foreground.add_to(ctx);
                }
                let background = background.to_owned();
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![fg_index],
                Box::new(move || {
                    let fg_image: Arc<Box<Pixmap>> = fg_future.into_result()?;
                    let mut fg_image = Arc::unwrap_or_clone(fg_image);
                    stack_layer_on_background(background, &mut fg_image);
                    Ok(fg_image)
                }))
            },
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                if let ToPixmapTaskSpec::PaintAlphaChannel {base: base_of_bg, color: color_of_bg} = background.deref()
                        && let ToPixmapTaskSpec::PaintAlphaChannel {base: base_of_fg, color: color_of_fg} = foreground.deref()
                        && color_of_bg == color_of_fg {
                    error!("Wanted to rebuild {} by merging {} and {}, but the borrow checker \
                    doesn't allow this!", self, base_of_bg, base_of_fg);
                    /*
                    FIXME: Fails borrow checker:
                    let simplified = ToPixmapTaskSpec::PaintAlphaChannel {
                        base: Box::new(ToAlphaChannelTaskSpec::StackAlphaOnAlpha {
                            background: base_of_bg.to_owned(),
                            foreground: base_of_fg.to_owned()
                        }),
                        color: color_of_fg.to_owned()
                    };
                    return simplified.add_to(ctx);
                     */
                }
                let (bg_index, bg_future) = background.add_to(ctx);
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![bg_index, fg_index], Box::new(move || {
                    let bg_image: Arc<Box<Pixmap>> = bg_future.into_result()?;
                    let mut out_image = Arc::unwrap_or_clone(bg_image);
                    let fg_image: Arc<Box<Pixmap>> = fg_future.into_result()?;
                    stack_layer_on_layer(&mut out_image, fg_image.deref());
                    Ok(out_image)
                }))
            },
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                if *color == ComparableColor::BLACK
                        && let ToAlphaChannelTaskSpec::FromPixmap {base: base_of_base} = base.deref()
                        && base_of_base.is_all_black() {
                    return base_of_base.add_to(ctx);
                }
                let color = color.to_owned();
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index],
                Box::new(move || {
                    let base_image: Arc<Box<Mask>> = base_future.into_result()?;
                    Ok(paint(Arc::unwrap_or_clone(base_image).as_ref(), color)?)
                }))
            },
        };
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        let task = CloneableLazyTask::new(function);
        ctx.pixmap_task_to_future_map.insert(self, (self_id, task.to_owned()));
        (self_id, task)
    }
}

impl TaskSpecTraits<Mask> for ToAlphaChannelTaskSpec {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<Mask>)
                         where Ix : IndexType, E: Default, 'b: 'a {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future))
                = ctx.alpha_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        let (dependencies, function): (Vec<NodeIndex<Ix>>, LazyTaskFunction<Mask>)
                = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                if *alpha == 1.0 {
                    return base.add_to(ctx);
                }
                let alpha: f32 = (*alpha).into();
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index],
                Box::new(move || {
                    let base_result: Arc<Box<Mask>> = base_future.into_result()?;
                    let mut channel = Arc::unwrap_or_clone(base_result);
                    make_semitransparent(&mut channel, alpha);
                    Ok(channel)
                }))
            },
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                if let ToPixmapTaskSpec::PaintAlphaChannel {base: base_of_base, ..} = base.deref() {
                    return base_of_base.add_to(ctx);
                }
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index],
                Box::new(move || {
                    let base_image: Arc<Box<Pixmap>> = base_future.into_result()?;
                    Ok(Box::new(Mask::from_pixmap((**base_image).as_ref(),
                                                  MaskType::Alpha)))
                }))
            },
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { layers } => {
                let mut layer_indices = Vec::with_capacity(layers.len());
                let mut layer_tasks: Vec<CloneableLazyTask<Mask>> = Vec::with_capacity(layers.len());
                for layer in layers {
                    let (layer_index, layer_task) = layer.add_to(ctx);
                    layer_indices.push(layer_index);
                    layer_tasks.push(layer_task);
                }
                (layer_indices,
                Box::new(move || {
                    let mut iter = layer_tasks.into_iter();
                    let bg = iter.next().unwrap();
                    let bg_arc: Arc<Box<Mask>> = bg.into_result()?;
                    let mut out_image = Arc::unwrap_or_clone(bg_arc);
                    for layer in iter {
                        stack_alpha_on_alpha(&mut out_image, &*(layer.into_result()?));
                    }
                    Ok(out_image)
                }))
            },
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background, foreground } => {
                if *background == 0.0 {
                    return foreground.add_to(ctx);
                }
                let background = background.0;
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![fg_index],
                 Box::new(move || {
                     let fg_arc: Arc<Box<Mask>> = fg_future.into_result()?;
                     let mut fg_image = Arc::unwrap_or_clone(fg_arc);
                     stack_alpha_on_background(background, &mut fg_image);
                     Ok(fg_image)
                 }))
            }
        };
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        let task = CloneableLazyTask::new(function);
        ctx.alpha_task_to_future_map.insert(self, (self_id, task.to_owned()));
        (self_id, task)
    }
}

impl TaskSpecTraits<()> for FileOutputTaskSpec {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<()>)
                         where Ix : IndexType, E: Default, 'b: 'a {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future))
                = ctx.output_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        let (dependencies, function): (Vec<NodeIndex<Ix>>, LazyTaskFunction<()>) = match self {
            FileOutputTaskSpec::PngOutput {base, destination } => {
                let destination = destination.to_owned();
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index], Box::new(move || {
                    Ok(Box::new(png_output(*Arc::unwrap_or_clone(base_future.into_result()?),
                                           destination)?))
                }))
            }
            FileOutputTaskSpec::Copy {original, link} => {
                let link = link.to_owned();
                let original_path = original.get_path();
                let (base_index, base_future) = original.add_to(ctx);
                (vec![base_index], Box::new(move || {
                    base_future.into_result()?;
                    Ok(Box::new(copy_out_to_out(original_path, link)?))
                }))
            }
        };
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        let wrapped_future = CloneableLazyTask::new(function);
        ctx.output_task_to_future_map.insert(self, (self_id, wrapped_future.to_owned()));
        (self_id, wrapped_future)
    }
}

pub type CloneableResult<T> = Result<Arc<Box<T>>, CloneableError>;

/// [TaskSpec] for a task that produces a [Pixmap].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToPixmapTaskSpec {
    Animate {background: Box<ToPixmapTaskSpec>, frames: Vec<ToPixmapTaskSpec>},
    FromSvg {source: PathBuf},
    PaintAlphaChannel {base: Box<ToAlphaChannelTaskSpec>, color: ComparableColor},
    StackLayerOnColor {background: ComparableColor, foreground: Box<ToPixmapTaskSpec>},
    StackLayerOnLayer {background: Box<ToPixmapTaskSpec>, foreground: Box<ToPixmapTaskSpec>},
    None {},
}

/// [TaskSpec] for a task that produces an [AlphaChannel].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToAlphaChannelTaskSpec {
    MakeSemitransparent {base: Box<ToAlphaChannelTaskSpec>, alpha: OrderedFloat<f32>},
    FromPixmap {base: Box<ToPixmapTaskSpec>},
    StackAlphaOnAlpha {layers: Vec<ToAlphaChannelTaskSpec>},
    StackAlphaOnBackground {background: OrderedFloat<f32>, foreground: Box<ToAlphaChannelTaskSpec>}
}

/// [TaskSpec] for a task that doesn't produce a heap object as output.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FileOutputTaskSpec {
    PngOutput {base: ToPixmapTaskSpec, destination: PathBuf},
    Copy {original: Box<FileOutputTaskSpec>, link: PathBuf}
}

impl FileOutputTaskSpec {
    pub(crate) fn get_path(&self) -> PathBuf {
        match self {
            FileOutputTaskSpec::PngOutput { destination, .. } => destination.to_owned(),
            FileOutputTaskSpec::Copy { link, .. } => link.to_owned()
        }
    }
}

/// Specification of a task that produces one of several output types. Created so that
/// copies of the same task created for different [Material] instances can be deduplicated, since
/// function closures and futures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum TaskSpec {
    ToPixmap(ToPixmapTaskSpec),
    ToAlphaChannel(ToAlphaChannelTaskSpec),
    FileOutput(FileOutputTaskSpec)
}

impl Display for TaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let inner: Box<&dyn Display> = Box::new(match self {
            TaskSpec::ToPixmap(inner) => inner,
            TaskSpec::ToAlphaChannel(inner) => inner,
            TaskSpec::FileOutput(inner) => inner
        });
        <Box<&dyn Display> as Display>::fmt(&inner, f)
    }
}

impl From<&ToPixmapTaskSpec> for TaskSpec {
    fn from(value: &ToPixmapTaskSpec) -> Self {
        TaskSpec::ToPixmap(value.to_owned())
    }
}

impl From<&ToAlphaChannelTaskSpec> for TaskSpec {
    fn from(value: &ToAlphaChannelTaskSpec) -> Self {
        TaskSpec::ToAlphaChannel(value.to_owned())
    }
}

impl From<&FileOutputTaskSpec> for TaskSpec {
    fn from(value: &FileOutputTaskSpec) -> Self {
        TaskSpec::FileOutput(value.to_owned())
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CloneableError {
    message: String
}

impl From<Error> for CloneableError {
    fn from(value: Error) -> Self {
        CloneableError {message: value.to_string()}
    }
}

#[macro_export]
macro_rules! anyhoo {
    ($($args:expr),+) => {
        $crate::image_tasks::task_spec::CloneableError::from(anyhow::anyhow!($($args),+))
    }
}

impl Display for ToPixmapTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToPixmapTaskSpec::Animate { background, frames } => {
                write!(f, "Animate({};{})", background, frames.iter().join(";"))
            }
            ToPixmapTaskSpec::FromSvg { source } => {
                write!(f, "{}", source.to_string_lossy())
            }
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                write!(f, "{}@{}", *base, color)
            }
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                write!(f, "{},{}", background, foreground)
            }
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                write!(f, "{},{}", background, foreground)
            }
            ToPixmapTaskSpec::None {} => {
                write!(f, "None")
            },
        }
    }
}

impl Display for FileOutputTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            FileOutputTaskSpec::PngOutput { destination, .. } => {
                destination.to_string_lossy().to_string()
            },
            FileOutputTaskSpec::Copy { original, link } => {
                format!("Symlink {} -> {}", link.to_string_lossy(), original.to_string())
            }
        })
    }
}

impl Display for ToAlphaChannelTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                write!(f, "{:?}@{:?}", base, alpha)
            }
            ToAlphaChannelTaskSpec::FromPixmap {base} => {
                write!(f, "Alpha({:?})", base)
            }
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha {layers} => {
                f.write_str(&layers.iter().join(","))
            }
            ToAlphaChannelTaskSpec::StackAlphaOnBackground {background, foreground} => {
                write!(f, "{},{}", background, foreground)
            }
        }
    }
}


trait TaskSpecFnMetadata: DataAccessDyn {}

impl DataAccessDyn for &TaskSpec {
    fn borrows(&self) -> TypeIds {
        TypeIds::new()
    }

    fn borrow_muts(&self) -> TypeIds {
        TypeIds::new()
    }
}

pub type LazyTaskFunction<T> = Box<dyn FnOnce() -> Result<Box<T>, CloneableError> + Send>;

pub enum CloneableLazyTaskState<T> where T: ?Sized {
    Upcoming {
        function: LazyTaskFunction<T>,
    },
    Finished {
        result: CloneableResult<T>
    }
}

#[derive(Clone,Debug)]
pub struct CloneableLazyTask<T> where T: ?Sized {
    state: Arc<Mutex<CloneableLazyTaskState<T>>>
}

impl <T> Debug for CloneableLazyTaskState<T> where T: ?Sized {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneableLazyTaskState::Upcoming { .. } => {
                f.write_str("Upcoming")
            },
            CloneableLazyTaskState::Finished { result } => {
                match result {
                    Ok(..) => f.write_str("Finished(Ok)"),
                    Err(error) => f.write_fmt(
                        format_args!("Finished(Error({}))", error.message))
                }
            }
        }
    }
}

impl <T> CloneableLazyTask<T> where T: ?Sized {
    pub fn new(base: LazyTaskFunction<T>) -> CloneableLazyTask<T> {
        CloneableLazyTask {
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Upcoming {
                function: base
            }))
        }
    }

    /// Consumes this particular copy of the task and returns the result. Trades off readability and
    /// maintainability to maximize the chance of avoiding unnecessary copies.
    pub fn into_result(self) -> CloneableResult<T> {
        let state = self.state;
        let result = Arc::try_unwrap(state);
        match result {
            Ok(mutex) => match mutex.into_inner() {
                Ok(state) => {
                    // We're the last referent to this Lazy, so we don't need to clone anything.
                    match state {
                        CloneableLazyTaskState::Upcoming { function } => {
                            function().map(Arc::new)
                        },
                        CloneableLazyTaskState::Finished { result } => {
                            result
                        },
                    }
                },
                Err(e) => {
                    Err(anyhoo!(e.to_string()))
                }
            },
            Err(arc) => {
                // We're not the last referent to this Lazy, so we need to make at least a shallow
                // copy, which will become deep (via Arc::clone_or_unwrap) if it needs to be
                // mutable.
                let lock_result = arc.lock();
                match lock_result {
                    Ok(mut guard) => replace_with_and_return(
                        guard.deref_mut(),
                        || CloneableLazyTaskState::Finished {result: Err(anyhoo!("replace_with failed")) },
                        |state| -> (CloneableResult<T>, CloneableLazyTaskState<T>) {
                            match state {
                                CloneableLazyTaskState::Upcoming { function } => {
                                    let result = function().map(Arc::new);
                                    (result.to_owned(), CloneableLazyTaskState::Finished { result })
                                },
                                CloneableLazyTaskState::Finished { result } => {
                                    (result.to_owned(), CloneableLazyTaskState::Finished { result })
                                }
                            }
                        }
                    ),
                    Err(e) => {
                        Err(anyhoo!(e.to_string()))
                    }
                }
            }
        }
    }
}

impl ToPixmapTaskSpec {
    /// Used in [TaskSpec::add_to] to deduplicate certain tasks that are redundant.
    fn is_all_black(&self) -> bool {
        match self {
            ToPixmapTaskSpec::None { .. } => panic!("is_all_black() called on None task"),
            ToPixmapTaskSpec::Animate { background, frames } =>
                background.is_all_black() && frames.iter().all(|frame| frame.is_all_black()),
            ToPixmapTaskSpec::FromSvg { source } => !(COLOR_SVGS.contains(&&*source.to_string_lossy())),
            ToPixmapTaskSpec::PaintAlphaChannel { color, .. } => color.is_black_or_transparent(),
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } =>
                background.is_black_or_transparent() && foreground.is_all_black(),
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => background.is_all_black() && foreground.is_all_black(),
        }
    }
}

impl From<ToPixmapTaskSpec> for ToAlphaChannelTaskSpec {
    fn from(value: ToPixmapTaskSpec) -> Self {
        ToAlphaChannelTaskSpec::FromPixmap {base: Box::new(value)}
    }
}

pub type TaskGraph<E, Ix> = Dag<TaskSpec, E, Ix>;
pub struct TaskGraphBuildingContext<'a, E, Ix> where Ix: IndexType {
    pub graph: TaskGraph<E, Ix>,
    pixmap_task_to_future_map: HashMap<&'a ToPixmapTaskSpec, (NodeIndex<Ix>, CloneableLazyTask<Pixmap>)>,
    alpha_task_to_future_map: HashMap<&'a ToAlphaChannelTaskSpec, (NodeIndex<Ix>, CloneableLazyTask<Mask>)>,
    pub output_task_to_future_map: HashMap<&'a FileOutputTaskSpec, (NodeIndex<Ix>, CloneableLazyTask<()>)>
}

impl <'a,E,Ix> TaskGraphBuildingContext<'a,E,Ix> where Ix: IndexType {
    pub(crate) fn new() -> Self {
        TaskGraphBuildingContext {
            graph: TaskGraph::new(),
            pixmap_task_to_future_map: HashMap::new(),
            alpha_task_to_future_map: HashMap::new(),
            output_task_to_future_map: HashMap::new()
        }
    }
}

lazy_static! {
    pub static ref ASSET_DIR: &'static Path = Path::new("assets/minecraft/textures");
    pub static ref SVG_DIR: &'static Path = Path::new("./svg/");
    pub static ref METADATA_DIR: &'static Path = Path::new("./metadata");
}

pub fn name_to_out_path(name: &str) -> PathBuf {
    let mut out_file_path = ASSET_DIR.to_path_buf();
    out_file_path.push(format!("{}.png", name));
    out_file_path
}

pub fn name_to_svg_path(name: &str) -> PathBuf {
    let mut svg_file_path = SVG_DIR.to_path_buf();
    svg_file_path.push(format!("{}.svg", name));
    svg_file_path
}

pub fn from_svg_task(name: &str) -> ToPixmapTaskSpec {
    ToPixmapTaskSpec::FromSvg {source: name_to_svg_path(name)}
}

pub fn svg_alpha_task(name: &str) -> ToAlphaChannelTaskSpec {
    ToAlphaChannelTaskSpec::from(from_svg_task(name))
}


pub fn paint_task(base: ToAlphaChannelTaskSpec, color: ComparableColor) -> ToPixmapTaskSpec {
        ToPixmapTaskSpec::PaintAlphaChannel {base: Box::new(base), color}
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> ToPixmapTaskSpec {
    paint_task(ToAlphaChannelTaskSpec::FromPixmap { base: Box::new(from_svg_task(name)) },
               color)
}

pub fn semitrans_svg_task(name: &str, alpha: f32) -> ToAlphaChannelTaskSpec {
    ToAlphaChannelTaskSpec::MakeSemitransparent {
        base: Box::new(ToAlphaChannelTaskSpec::FromPixmap {
            base: Box::new(from_svg_task(name))
        }),
        alpha: alpha.into()}
}

pub fn out_task(name: &str, base: ToPixmapTaskSpec) -> FileOutputTaskSpec {
    FileOutputTaskSpec::PngOutput {base, destination: name_to_out_path(name)}
}

#[macro_export]
macro_rules! stack_alpha {
    ( $( $layers:expr ),+ ) => {
        {
            let layers: Vec<$crate::image_tasks::task_spec::ToAlphaChannelTaskSpec> = vec![$($layers.into()),+];
            let mut layers: Vec<$crate::image_tasks::task_spec::ToAlphaChannelTaskSpec> = layers
                    .into_iter()
                    .flat_map(|layer|
                        if let $crate::image_tasks::task_spec::ToAlphaChannelTaskSpec::StackAlphaOnAlpha {layers: sub_layers } = layer {
                            sub_layers.to_owned()
                        } else {
                            vec![layer]
                        }
                    )
                    .collect();
            layers.sort();
            $crate::image_tasks::task_spec::ToAlphaChannelTaskSpec::StackAlphaOnAlpha {layers}
        }
    }
}

pub fn stack(background: ToPixmapTaskSpec, foreground: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
    if let ToPixmapTaskSpec::PaintAlphaChannel {base: fg_base, color: fg_color} = &foreground {
        if let ToPixmapTaskSpec::PaintAlphaChannel {base: bg_base, color: bg_color} = &background
                && fg_color == bg_color {
            return ToPixmapTaskSpec::PaintAlphaChannel {
                base: Box::new(stack_alpha!(*bg_base.to_owned(), *fg_base.to_owned())),
                color: fg_color.to_owned()
            };
        } else if let ToPixmapTaskSpec::StackLayerOnLayer {background: bg_bg, foreground: bg_fg} = &background
                && let ToPixmapTaskSpec::PaintAlphaChannel {base: bg_fg_base, color: bg_fg_color} = &**bg_fg
                && fg_color == bg_fg_color {
            return ToPixmapTaskSpec::StackLayerOnLayer {
                background: bg_bg.to_owned(),
                foreground: Box::new(ToPixmapTaskSpec::PaintAlphaChannel {
                    base: Box::new(stack_alpha!(*bg_fg_base.to_owned(), *fg_base.to_owned())),
                    color: fg_color.to_owned()
                })
            };
        } else if let ToPixmapTaskSpec::StackLayerOnColor {background: bg_bg, foreground: bg_fg} = &background
                && let ToPixmapTaskSpec::PaintAlphaChannel {base: bg_fg_base, color: bg_fg_color} = &**bg_fg
                && fg_color == bg_fg_color {
            return ToPixmapTaskSpec::StackLayerOnColor {
                background: bg_bg.to_owned(),
                foreground: Box::new(ToPixmapTaskSpec::PaintAlphaChannel {
                    base: Box::new(stack_alpha!(*bg_fg_base.to_owned(), *fg_base.to_owned())),
                    color: fg_color.to_owned()
                })
            };
        }
    }
    ToPixmapTaskSpec::StackLayerOnLayer {
        background: Box::new(background), foreground: Box::new(foreground)
    }
}

#[macro_export]
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        $crate::image_tasks::task_spec::stack($first_layer.into(), $second_layer.into())
    };
    ( $first_layer:expr, $second_layer:expr, $( $more_layers:expr ),+ ) => {{
        let mut layers_so_far = $crate::stack!($first_layer, $second_layer);
        $( layers_so_far = $crate::stack!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

#[macro_export]
macro_rules! stack_on {
    ( $background:expr, $foreground:expr ) => {
        if $background == $crate::image_tasks::color::ComparableColor::TRANSPARENT {
            $foreground
        } else {
            $crate::image_tasks::task_spec::ToPixmapTaskSpec::StackLayerOnColor {
                background: $background,
                foreground: Box::new($foreground.into())
            }
        }
    };
    ( $background:expr, $first_layer:expr, $( $more_layers:expr ),+ ) => {{
        if $background == $crate::image_tasks::color::ComparableColor::TRANSPARENT {
            $crate::stack!($first_layer, $($more_layers),+)
        } else {
            let mut layers_so_far = $crate::stack_on!($background, $first_layer);
            $( layers_so_far = $crate::stack!(layers_so_far, $more_layers); )+
            layers_so_far
        }
    }};
}

#[macro_export]
macro_rules! paint_stack {
    ( $color:expr, $( $layers:expr ),* ) => {
        $crate::image_tasks::task_spec::paint_task(
            $crate::stack_alpha!(
                $(
                    $crate::image_tasks::task_spec::svg_alpha_task($layers)
                ),*).into(),
            $color)
    }
}

impl FromStr for ToPixmapTaskSpec {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ToPixmapTaskSpec::FromSvg {
            source: name_to_svg_path(s)
        })
    }
}

impl Mul<f32> for ToAlphaChannelTaskSpec {
    type Output = ToAlphaChannelTaskSpec;

    fn mul(self, rhs: f32) -> Self::Output {
        ToAlphaChannelTaskSpec::MakeSemitransparent {
            base: Box::new(self),
            alpha: OrderedFloat::from(rhs)
        }
    }
}

impl Mul<ComparableColor> for ToAlphaChannelTaskSpec {
    type Output = ToPixmapTaskSpec;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        paint_task(self, rhs)
    }
}

impl Mul<ComparableColor> for ToPixmapTaskSpec {
    type Output = ToPixmapTaskSpec;
    fn mul(self, rhs: ComparableColor) -> Self::Output {
        match &self {
            ToPixmapTaskSpec::PaintAlphaChannel { base, .. } => {
                ToPixmapTaskSpec::PaintAlphaChannel {
                    base: Box::new(*base.to_owned()),
                    color: rhs
                }
            },
            _ => ToPixmapTaskSpec::PaintAlphaChannel {
                base: Box::new(ToAlphaChannelTaskSpec::FromPixmap { base: Box::new(self) }),
                color: rhs
            }
        }
    }
}
