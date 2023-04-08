use std::collections::{HashMap};
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::future::{Future, IntoFuture};
use std::ops::{DerefMut, FromResidual, Mul};
use std::path::{Path, PathBuf};
use std::pin::{Pin};
use std::str::FromStr;
use std::sync::{Arc, PoisonError, Weak};
use std::task::{Context, Poll};
use anyhow::Error;
use cached::lazy_static::lazy_static;
use cached::once_cell::sync::Lazy;
use fn_graph::{DataAccessDyn, FnGraphBuilder, FnId, TypeIds};
use futures::{FutureExt};
use futures::future::{BoxFuture};
use ordered_float::OrderedFloat;
use tiny_skia::Pixmap;
use weak_table::{WeakKeyHashMap};

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::from_svg::from_svg;
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::png_output::png_output;
use crate::image_tasks::repaint::{AlphaChannel, to_alpha_channel};
use crate::image_tasks::repaint::paint;
use crate::image_tasks::stack::{stack_layer_on_background, stack_layer_on_layer};
use crate::image_tasks::task_spec::TaskSpec::{FromSvg, PngOutput};
use crate::TILE_SIZE;

/// Specification of a task that produces and/or consumes at least one [Pixmap]. Created
/// to de-duplicate copies of the same task, since function closures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum TaskSpec {
    None {},
    Animate {background: Box<TaskSpec>, frames: Vec<TaskSpec>},
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

#[derive(Clone, Debug)]
pub enum TaskResult {
    Err {value: CloneableError},
    Empty {},
    Pixmap {value: Pixmap},
    AlphaChannel {value: AlphaChannel}
}

#[macro_export]
macro_rules! anyhoo {
    ($($args:expr),+) => {
        crate::image_tasks::task_spec::CloneableError::from(anyhow::anyhow!($($args),+))
    }
}

impl FromResidual<Result<Pixmap, CloneableError>> for TaskResult {
    fn from_residual(residual: Result<Pixmap, CloneableError>) -> Self {
        match residual {
            Err(e) => TaskResult::Err {value: e},
            Ok(pixmap) => TaskResult::Pixmap {value: pixmap}
        }
    }
}

impl TryInto<Pixmap> for TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<Pixmap, CloneableError> {
        match self {
            TaskResult::Pixmap { value } => Ok(value),
            TaskResult::Err { value } => Err(value),
            TaskResult::Empty {} => Err(anyhoo!("Tried to cast an empty result to Pixmap")),
            TaskResult::AlphaChannel { .. } => Err(anyhoo!("Tried to cast an AlphaChannel result to Pixmap")),
        }
    }
}

impl TryInto<Pixmap> for &TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<Pixmap, CloneableError> {
        match self {
            TaskResult::Pixmap { value } => Ok(value.to_owned()),
            TaskResult::Err { value } => Err(value.to_owned()),
            TaskResult::Empty {} => Err(anyhoo!("Tried to cast an empty result to Pixmap")),
            TaskResult::AlphaChannel { .. } => Err(anyhoo!("Tried to cast an AlphaChannel result to Pixmap")),
        }
    }
}

impl FromResidual<Result<AlphaChannel, CloneableError>> for TaskResult {
    fn from_residual(residual: Result<AlphaChannel, CloneableError>) -> Self {
        match residual {
            Err(e) => TaskResult::Err {value: e},
            Ok(alpha_channel) => TaskResult::AlphaChannel {value: alpha_channel}
        }
    }
}

impl TryInto<AlphaChannel> for TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<AlphaChannel, CloneableError> {
        match self {
            TaskResult::Pixmap { .. } => Err(anyhoo!("Tried to cast an empty result to AlphaChannel")),
            TaskResult::Err { value } => Err(value),
            TaskResult::Empty {} => Err(anyhoo!("Tried to cast an empty result to AlphaChannel")),
            TaskResult::AlphaChannel { value } => Ok(value)
        }
    }
}

impl TryInto<AlphaChannel> for &TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<AlphaChannel, CloneableError> {
        match self {
            TaskResult::Pixmap { .. } => Err(anyhoo!("Tried to cast an empty result to AlphaChannel")),
            TaskResult::Err { value } => Err(value.to_owned()),
            TaskResult::Empty {} => Err(anyhoo!("Tried to cast an empty result to AlphaChannel")),
            TaskResult::AlphaChannel { value } => Ok(value.to_owned())
        }
    }
}

impl FromResidual<Result<Infallible, CloneableError>> for TaskResult {
    fn from_residual(residual: Result<Infallible, CloneableError>) -> Self {
        match residual {
            Err(e) => TaskResult::Err {value: e},
            Ok(..) => TaskResult::Empty {}
        }
    }
}

impl TryInto<()> for TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<(), CloneableError> {
        match self {
            TaskResult::Err { value } => Err(value),
            _ => Ok(())
        }
    }
}

impl TryInto<()> for &TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<(), CloneableError> {
        match self {
            TaskResult::Err { value } => Err(value.to_owned()),
            _ => Ok(())
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

trait TaskSpecFnMetadata: DataAccessDyn {}

impl DataAccessDyn for &TaskSpec {
    fn borrows(&self) -> TypeIds {
        TypeIds::new()
    }

    fn borrow_muts(&self) -> TypeIds {
        TypeIds::new()
    }
}

type SyncMutex<T> = std::sync::Mutex<T>;
type ResultMap = SyncMutex<WeakKeyHashMap<Weak<TaskSpec>, CloneableFutureWrapper<'static, TaskResult>>>;

#[derive(Clone)]
pub struct CloneableFutureWrapper<'a, T> where T: Clone + Send {
    result: Arc<SyncMutex<Option<T>>>,
    future: Arc<SyncMutex<BoxFuture<'a, T>>>,
}

impl <'a, T> CloneableFutureWrapper<'a, T> where T: Clone + Send {
    pub fn new<U>(base: U) -> CloneableFutureWrapper<'a, T>
            where U : Future<Output=T> + Send + 'a {
        return CloneableFutureWrapper {
            result: Arc::new(SyncMutex::new(None)),
            future: Arc::new(SyncMutex::new(Box::pin(base.into_future()))),
        }
    }
}

impl <'a, T> Future for CloneableFutureWrapper<'a, T> where T: Clone + Send {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        return match self.result.lock() {
            Ok(mut locked_result) => {
                let locked_result = locked_result.deref_mut();
                match locked_result {
                    Some(result) => Poll::Ready(result.to_owned()),
                    None => {
                        let mut future = self.future.lock().unwrap();
                        let new_result = future.poll_unpin(cx);
                        match new_result {
                            Poll::Ready(new_result) => {
                                *locked_result = Some(new_result.to_owned());
                                Poll::Ready(new_result)
                            },
                            Poll::Pending => Poll::Pending
                        }
                    }
                }
            }
            Err(PoisonError {..}) => {
                panic!("Got a PoisonError while polling self")
            }
        }
    }

    /*
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        return match self.result.try_lock() {
            Ok(mut locked_result) => {
                let mut locked_result = locked_result.deref_mut();
                match locked_result {
                    Some(result) => Poll::Ready(result.to_owned()),
                    None => {
                        let mut future = self.future.get_mut().unwrap();
                        let new_result = future.poll_unpin(cx);
                        match new_result {
                            Poll::Ready(new_result) => {
                                *locked_result = Some(new_result.to_owned());
                                Poll::Ready(new_result)
                            },
                            Poll::Pending => Poll::Pending
                        }
                    }
                }
            }
            Err(TryLockError::WouldBlock) => {
                let weak_self = Arc::downgrade(&mut Arc::new(self.deref()));
                self.waker = Arc::downgrade(&Arc::new(cx.waker()));
                tokio::spawn(async move {
                    let maybe_self = weak_self.upgrade();
                    match maybe_self {
                        Some(live_self) => {
                            let &mut mut locked_result = live_self.result.get_mut().unwrap();
                            if locked_result.is_none() {
                                let &mut locked_future = live_self.future.get_mut().unwrap();
                                locked_result = Some(locked_future.await);
                                let maybe_waker = live_self.waker.upgrade();
                                match maybe_waker {
                                    Some(live_waker) => {
                                        live_waker.wake_by_ref();
                                    }
                                    None => {}
                                }
                            }
                        }
                        None => {}
                    }
                });
                Poll::Pending
            }
            Err(TryLockError::Poisoned(e)) => {
                panic!("{}", e)
            }
        }
    }*/
}

impl IntoFuture for TaskSpec {
    type Output = TaskResult;
    type IntoFuture = CloneableFutureWrapper<'static, TaskResult>;
    fn into_future(self) -> CloneableFutureWrapper<'static, TaskResult> {
        let mut results_map = RESULTS.lock().unwrap();
        let entry = results_map.entry(Arc::new(self.clone()));
        let owned_self = self.to_owned();
        return entry.or_insert_with(|| CloneableFutureWrapper::new(Box::pin(async {
            match owned_self {
                TaskSpec::None { .. } => {
                    TaskResult::Err { value: anyhoo!("Call to into_future() on a None task") }
                },
                TaskSpec::Animate { background, frames } => {
                    let background: Pixmap = background.to_owned().into_future().await.try_into()?;
                    let frames: Vec<CloneableFutureWrapper<TaskResult>>
                        = frames.iter().map(|task_spec| task_spec.to_owned().into_future()).collect();
                    animate(background, frames).await
                },
                FromSvg { source } => {
                    from_svg(source.to_owned(), *TILE_SIZE)
                },
                TaskSpec::MakeSemitransparent { base, alpha } => {
                    let base: Pixmap = base.into_future().await.try_into()?;
                    make_semitransparent(base, alpha.0)
                },
                PngOutput { base, destinations } => {
                    let base: Pixmap = base.into_future().await.try_into()?;
                    png_output(base, &destinations)
                },
                TaskSpec::Repaint { base, color } => {
                    let base: AlphaChannel = base.into_future().await.try_into()?;
                    paint(base, &color)
                },
                TaskSpec::StackLayerOnColor { background, foreground } => {
                    let foreground: Pixmap = foreground.into_future().await.try_into()?;
                    stack_layer_on_background(&background, foreground)
                },
                TaskSpec::StackLayerOnLayer { background, foreground } => {
                    let background: Pixmap = background.into_future().await.try_into()?;
                    let foreground: Pixmap = foreground.into_future().await.try_into()?;
                    stack_layer_on_layer(background, foreground)
                },
                TaskSpec::ToAlphaChannel { base } => {
                    let base: Pixmap = base.into_future().await.try_into()?;
                    to_alpha_channel(base)
                }
            }
        }))).clone();
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
    pub static ref OUT_DIR: &'static Path = Path::new("./out/");
    pub static ref SVG_DIR: &'static Path = Path::new("./svg/");
}

        pub fn name_to_out_path(name: &str) -> PathBuf {
            let mut out_file_path = OUT_DIR.to_path_buf();
            out_file_path.push(format!("{}.png", name));
            return out_file_path;
        }

        pub fn name_to_svg_path(name: &str) -> PathBuf {
            let mut svg_file_path = SVG_DIR.to_path_buf();
            svg_file_path.push(format!("{}.svg", name));
            return svg_file_path;
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

static RESULTS: Lazy<ResultMap> = Lazy::new(|| SyncMutex::new(WeakKeyHashMap::new()));
