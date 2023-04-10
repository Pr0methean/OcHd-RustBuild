use std::collections::{HashMap};
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::future::{Future, IntoFuture};
use std::ops::{Deref, DerefMut, FromResidual, Index, Mul};
use std::panic::panic_any;
use std::path::{Path, PathBuf};
use std::pin::{Pin};
use std::str::FromStr;
use std::sync::{Arc, PoisonError, Mutex, Weak, OnceLock, TryLockError, TryLockResult, LockResult, MutexGuard};
use std::task::{Context, Poll, Waker};
use anyhow::{anyhow, Error};
use async_std::task::block_on;
use cached::lazy_static::lazy_static;
use chashmap_next::CHashMap;
use fn_graph::{DataAccessDyn, FnGraphBuilder, FnId, TypeIds};
use fn_graph::daggy::Dag;
use futures::{FutureExt};
use futures::channel::oneshot::{channel, Receiver};
use futures::future::{BoxFuture, ready};
use ordered_float::OrderedFloat;
use petgraph::graph::{IndexType, NodeIndex};
use tiny_skia::Pixmap;
use tokio::sync::OnceCell;

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg};
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

pub type SyncBoxFuture<'a, T> = Pin<Box<dyn Future<Output=T> + Send + Sync + 'a>>;

#[derive(Clone)]
pub struct CloneableFutureWrapper<'a, T> where T: Clone + Send {
    name: String,
    result: Arc<Mutex<OnceCell<T>>>,
    future: Arc<Mutex<SyncBoxFuture<'a, T>>>,
    wakers: Arc<Mutex<Vec<Arc<Waker>>>>
}

impl <'a, T> Debug for CloneableFutureWrapper<'a, T> where T: Clone + Send + Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloneableFutureWrapper")
            .field("name", &self.name)
            .field("result", &self.result)
            .field("wakers", &self.wakers)
            .finish()
    }
}

impl <'a, T> CloneableFutureWrapper<'a, T> where T: Clone + Send + Debug + 'a {
    pub fn new(name: &str, base: SyncBoxFuture<'a, T>) -> CloneableFutureWrapper<'a, T> {
        let name_string = name.to_string();
        return CloneableFutureWrapper {
            name: name_string.to_owned(),
            result: Arc::new(Mutex::new(OnceCell::new())),
            future: Arc::new(Mutex::new(Box::pin(async move {
                println!("Starting {}", name_string);
                let result = base.await;
                println!("Finishing {}", name_string);
                result
            }))),
            wakers: Arc::new(Mutex::new(vec![]))
        };
    }
}

impl <'a, T> Future for CloneableFutureWrapper<'a, T> where T: Clone + Send + Sync + Debug + 'a {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result_lock = self.result.lock().unwrap();
        let result = result_lock.get();
        return match result {
            Some(t) => Poll::Ready(t.to_owned()),
            None => {
                let mut future_lock = self.future.lock().unwrap();
                let future = future_lock.deref_mut();
                let new_result = future.poll_unpin(cx);
                let mut wakers_lock = self.wakers.lock().unwrap();
                match new_result.to_owned() {
                    Poll::Ready(result_value) => {
                        let wakers = wakers_lock.deref_mut();
                        for waker in wakers.drain(..) {
                            waker.wake_by_ref();
                        }
                        result_lock.set(result_value.to_owned()).expect("Failed to set result_lock"); // Ignore error for idempotence
                    },
                    Poll::Pending => {
                        wakers_lock.deref_mut().push(Arc::new(cx.waker().to_owned()));
                    }
                }
                new_result
            }
        }
    }
}

impl TaskSpec {
    fn is_all_black(&self) -> bool {
        match self {
            TaskSpec::None { .. } => false,
            TaskSpec::Animate { background, frames } =>
                background.is_all_black() && frames.iter().all(|frame| frame.is_all_black()),
            FromSvg { source } => !(COLOR_SVGS.contains(&&*source.to_string_lossy())),
            TaskSpec::MakeSemitransparent { base, .. } => base.is_all_black(),
            PngOutput { base, .. } => base.is_all_black(),
            TaskSpec::Repaint { color, .. } => color.is_black_or_transparent(),
            TaskSpec::StackLayerOnColor { background, foreground } =>
                background.is_black_or_transparent() && foreground.is_all_black(),
            TaskSpec::StackLayerOnLayer { background, foreground } => background.is_all_black() && foreground.is_all_black(),
            TaskSpec::ToAlphaChannel { .. } => false
        }
    }
}

pub type TaskResultFuture<'b> = CloneableFutureWrapper<'b, TaskResult>;
pub type TaskToFutureGraphNodeMap<'a,'b,Ix>
    = HashMap<TaskSpec, (NodeIndex<Ix>, TaskResultFuture<'b>)>;

impl TaskSpec {
    pub fn add_to<'a,'b,E, Ix>(self,
                               graph: &mut Dag<TaskResultFuture<'b>, E, Ix>,
                               existing_nodes: &mut TaskToFutureGraphNodeMap<'a,'b,Ix>)
                               -> (NodeIndex<Ix>, TaskResultFuture<'b>)
    where Ix: IndexType, E: Default, 'b : 'a
    {
        let name: String = (&self).to_string();
        if existing_nodes.contains_key(&self) {
            println!("Matched an existing node: {}", self);
            let (index, future) = existing_nodes.get(&self).unwrap();
            return (index.to_owned(), future.to_owned());
        }

        // Simplify redundant tasks first
        match &self {
            TaskSpec::MakeSemitransparent {base, alpha} => {
                if *alpha == 1.0 {
                    return base.to_owned().add_to(graph, existing_nodes);
                }
            },
            TaskSpec::Repaint {base, color} => {
                if *color == ComparableColor::BLACK
                        && let TaskSpec::ToAlphaChannel{base: base_of_base} = base.deref()
                        && base_of_base.is_all_black() {
                    return base.to_owned().add_to(graph, existing_nodes);
                }
            },
            TaskSpec::StackLayerOnColor {background, foreground} => {
                if *background == ComparableColor::TRANSPARENT {
                    return foreground.to_owned().add_to(graph, existing_nodes);
                }
            }
            _ => {}
        }

        println!("No existing node found for: {}", name);
        let new_self = self.to_owned();
        let mut dependencies: Vec<NodeIndex<Ix>> = vec![];
        let as_future: TaskResultFuture<'b> = match new_self.to_owned() {
            TaskSpec::None { .. } =>
                CloneableFutureWrapper::new(&*name, Box::pin(
                async { TaskResult::Err{value: anyhoo!("Call to into_future() on a None task") } }
                )),
            TaskSpec::Animate { background, frames } => {
                let background_name = background.to_string();
                let (background_index, background_future)
                    = background.to_owned().add_to(graph, existing_nodes);
                dependencies.reserve(frames.len() + 1);
                dependencies.push(background_index);
                let mut frame_futures = Vec::with_capacity(frames.len());
                for frame in frames {
                    let (frame_index, frame_future) = frame.to_owned().add_to(graph, existing_nodes);
                    frame_futures.push(frame_future.to_owned());
                    dependencies.push(frame_index);
                }
                let background_future = background_future.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(async move {
                    match background_future.await {
                        TaskResult::Pixmap {value} => animate(value, frame_futures).await,
                        _ => TaskResult::Err {value: anyhoo!("Got {:?} instead of Pixmap for background", background_name)}
                    }
                }))
            },
            FromSvg { source } => {
                let source = source.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(
                    async move { from_svg(source, *TILE_SIZE) }))
            },
            TaskSpec::MakeSemitransparent { base, alpha } => {
                let alpha = alpha.0.to_owned();
                let (base_index, base_future) = base.to_owned().add_to(graph, existing_nodes);
                dependencies.push(base_index);
                CloneableFutureWrapper::new(&*name, Box::pin(
                    async move {
                        make_semitransparent(base_future.await.try_into()?, alpha)
                    }))
            },
            PngOutput { base, destinations } => {
                let (base_index, base_future) = base.to_owned().add_to(graph, existing_nodes);
                dependencies.push(base_index);
                let destinations = destinations.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(
                    async move {
                        png_output(base_future.await.try_into()?, &destinations)
                    }
                ))
            },
            TaskSpec::Repaint { base, color } => {
                let (base_index, base_future) = base.to_owned().add_to(graph, existing_nodes);
                dependencies.push(base_index);
                let color = color.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(async move {
                    paint(base_future.to_owned().await.try_into()?, &color)
                }))
            },
            TaskSpec::StackLayerOnColor { background, foreground } => {
                let (fg_index, fg_future) = foreground.to_owned().add_to(graph, existing_nodes);
                dependencies.push(fg_index);
                let fg_future = fg_future.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(async move {
                    stack_layer_on_background(&background, &(fg_future.to_owned().await.try_into()?))
                }))
            },
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                let (bg_index, bg_future) = background.to_owned().add_to(graph, existing_nodes);
                dependencies.reserve(2);
                dependencies.push(bg_index);
                let bg_future = bg_future.to_owned();
                let (fg_index, fg_future) = foreground.to_owned().add_to(graph, existing_nodes);
                dependencies.push(fg_index);
                let fg_future = fg_future.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(async move {
                    stack_layer_on_layer(&(bg_future.to_owned().await.try_into()?), &(fg_future.to_owned().await.try_into()?))
                }))
            },
            TaskSpec::ToAlphaChannel { base } => {
                let (base_index, base_future) = base.to_owned().add_to(graph, existing_nodes);
                dependencies.push(base_index);
                let base_future = base_future.to_owned();
                CloneableFutureWrapper::new(&*name, Box::pin(async move {
                    to_alpha_channel(&(base_future.to_owned().await.try_into()?))
                }))
            }
        };
        let self_id = graph.add_node(as_future.to_owned()).to_owned();
        for dependency in dependencies {
            graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        existing_nodes.insert(new_self, (self_id, as_future.to_owned()));
        return (self_id.to_owned(), as_future.to_owned());
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

pub fn from_svg_task(name: &str) -> Box<TaskSpec> {
    return Box::new(FromSvg {source: name_to_svg_path(name)});
}

pub fn paint_task(base: Box<TaskSpec>, color: ComparableColor) -> Box<TaskSpec> {
    return Box::new(
        TaskSpec::Repaint {base: Box::new(TaskSpec::ToAlphaChannel { base }), color});
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> Box<TaskSpec> {
    return paint_task(from_svg_task(name), color);
}

pub fn semitrans_svg_task(name: &str, alpha: f32) -> Box<TaskSpec> {
    return Box::new(TaskSpec::MakeSemitransparent {base: from_svg_task(name),
        alpha: alpha.into()});
}

pub fn path(name: &str) -> Vec<PathBuf> {
    return vec![name_to_out_path(name)];
}

pub fn out_task(name: &str, base: Box<TaskSpec>) -> Box<TaskSpec> {
    return Box::new(PngOutput {base, destinations: path(name)});
}

#[macro_export]
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        Box::new(crate::image_tasks::task_spec::TaskSpec::StackLayerOnLayer {
            background: $first_layer.into(),
            foreground: $second_layer.into()
        })
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
        Box::new(crate::image_tasks::task_spec::TaskSpec::StackLayerOnColor {
            background: $background,
            foreground: $foreground.into()
        })
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
        crate::image_tasks::task_spec::paint_task(
            crate::stack!($(crate::image_tasks::task_spec::from_svg_task($layers)),*).into(),
            $color)
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
            base: Box::new(self),
            alpha: OrderedFloat::from(rhs)
        }
    }
}

impl Mul<ComparableColor> for TaskSpec {
    type Output = TaskSpec;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        let owned_self = self.to_owned();
        return match self {
            TaskSpec::ToAlphaChannel { base: _base } => {
                TaskSpec::Repaint {
                    base: Box::new(owned_self),
                    color: rhs
                }
            },
            _ => TaskSpec::Repaint {
                base: Box::new(TaskSpec::ToAlphaChannel { base: Box::new(self) }),
                color: rhs
            }
        };
    }
}
