use std::cell::RefCell;
use std::collections::{HashMap};
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};
use std::future::{Future};
use std::ops::{Deref, DerefMut, FromResidual, Mul};
use std::path::{Path, PathBuf};
use std::pin::{Pin};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use anyhow::{Error};

use cached::lazy_static::lazy_static;
use cooked_waker::{IntoWaker, ViaRawPointer, WakeRef};
use fn_graph::{DataAccessDyn, TypeIds};
use fn_graph::daggy::Dag;
use futures::{FutureExt};


use log::{info};
use ordered_float::OrderedFloat;
use petgraph::graph::{IndexType, NodeIndex};
use tiny_skia::Pixmap;

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

/// Specification of a task that produces and/or consumes at least one [Pixmap]. Created so that
/// copies of the same task created for different [Material] instances can be deduplicated, since
/// function closures and futures don't implement [Eq] or [Hash].
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
        CloneableError {message: value.to_string()}
    }
}

impl Display for CloneableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Tagged union of the possible results of [TaskSpec] execution.
#[derive(Clone)]
pub enum TaskResult {
    Err {value: CloneableError},
    Empty {},
    Pixmap {value: Arc<Pixmap>},
    AlphaChannel {value: Arc<AlphaChannel>}
}

impl Debug for TaskResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskResult::Err {value } => f.debug_struct("Err").field("value", value).finish(),
            TaskResult::Empty {} => f.write_str("Empty"),
            TaskResult::Pixmap { .. } => f.write_str("Pixmap"),
            TaskResult::AlphaChannel { .. } => f.write_str("AlphaChannel")
        }
    }
}

#[macro_export]
macro_rules! anyhoo {
    ($($args:expr),+) => {
        $crate::image_tasks::task_spec::CloneableError::from(anyhow::anyhow!($($args),+))
    }
}

impl FromResidual<Result<Pixmap, CloneableError>> for TaskResult {
    fn from_residual(residual: Result<Pixmap, CloneableError>) -> Self {
        match residual {
            Err(e) => TaskResult::Err {value: e},
            Ok(pixmap) => TaskResult::Pixmap {value: Arc::new(pixmap)}
        }
    }
}

impl <'a> TryInto<Arc<Pixmap>> for &'a TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<Arc<Pixmap>, CloneableError> {
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
            Ok(alpha_channel) => TaskResult::AlphaChannel {value: Arc::new(alpha_channel)}
        }
    }
}

impl <'a> TryInto<Arc<AlphaChannel>> for &TaskResult {
    type Error = CloneableError;
    fn try_into(self) -> Result<Arc<AlphaChannel>, CloneableError> {
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
                write!(f, "{{{};{}}}", background, foreground)
            }
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                write!(f, "{{{};{}}}", background, foreground)
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

/// Stores the wakers for all clones of a given [CloneableFutureWrapper] and wakes them when the
/// wrapped future wakes the MultiWaker or is found to be ready.
#[derive(Debug)]
pub struct MultiWaker {
    wakers: Mutex<Vec<Waker>>
}

unsafe impl ViaRawPointer for &MultiWaker {
    type Target = MultiWaker;

    fn into_raw(self) -> *mut Self::Target {
        (unsafe { Pin::into_inner_unchecked(Pin::new_unchecked(self)) }
        as *const MultiWaker)
        as *mut MultiWaker
    }

    unsafe fn from_raw(ptr: *mut Self::Target) -> Self {
        Pin::into_inner_unchecked(Pin::new_unchecked(&*ptr))
    }
}

impl MultiWaker {
    fn new() -> MultiWaker {
        MultiWaker {wakers: Mutex::new(Vec::with_capacity(16))}
    }

    fn add_waker(&self, waker: Waker) {
        self.wakers.lock().unwrap().push(waker);
    }
}

impl WakeRef for MultiWaker {
    fn wake_by_ref(&self) {
        for waker in self.wakers.lock().unwrap().drain(..) {
            waker.wake_by_ref();
        }
    }
}

pub enum CloneableFutureWrapperState<'a, T> {
    Upcoming {
        future: SyncBoxFuture<'a, T>,
        multiwaker: Arc<MultiWaker>,
        waker: Arc<Waker>
    },
    Finished {
        result: Arc<T>
    }
}

/// Wraps a future so it can be consumed by multiple dependencies. Implemented because I was told
/// [futures::future::Shared] would probably have a memory leak. Unlike Shared, this doesn't
/// directly use a [std::cell::UnsafeCell].
#[derive(Clone,Debug)]
pub struct CloneableFutureWrapper<'a, T> where T: Clone + Sync + Send {
    state: Arc<Mutex<CloneableFutureWrapperState<'a, T>>>
}

impl <'a, T> Debug for CloneableFutureWrapperState<'a, T> where T: Clone + Send + Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneableFutureWrapperState::Upcoming { waker, .. } =>
                f.debug_struct("Upcoming").field("waker", waker).finish(),
            CloneableFutureWrapperState::Finished { result } => f.debug_struct("Finished").field("result", result).finish()
        }
    }
}

impl <'a, T> CloneableFutureWrapper<'a, T> where T: Clone + Sync + Send + Debug + 'a {
    pub fn new(name: String, base: SyncBoxFuture<'a, T>) -> CloneableFutureWrapper<'a, T> {
        let waker = Arc::new(MultiWaker::new());
        CloneableFutureWrapper {
            state: Arc::new(Mutex::new(CloneableFutureWrapperState::Upcoming {
                future: Box::pin(async move {
                    info!("Starting {}", name);
                    let result = base.await;
                    info!("Finishing {} with result of {:?}", name, result);
                    result
                }),
                multiwaker: waker.to_owned(),
                waker: Arc::new(waker.into_waker())
            }))
        }
    }
}

impl <'a, T> Future for CloneableFutureWrapper<'a, T> where T: Clone + Send + Sync + Debug + 'a {
    type Output = Arc<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match state.deref_mut() {
            CloneableFutureWrapperState::Finished { result } => Poll::Ready(result.to_owned()),
            CloneableFutureWrapperState::Upcoming { future, multiwaker, waker} => {
                let new_result = future.poll_unpin(
                    &mut Context::from_waker(waker.deref()));
                if let Poll::Ready(finished_result) = new_result {
                    let result_arc = Arc::new(finished_result);
                    waker.wake_by_ref();
                    *state = CloneableFutureWrapperState::Finished {result: result_arc.to_owned()};
                    Poll::Ready(result_arc)
                } else {
                    multiwaker.add_waker(cx.waker().to_owned());
                    Poll::Pending
                }
            }
        }
    }
}

impl TaskSpec {
    /// Used in [TaskSpec::add_to] to deduplicate certain tasks that are redundant.
    fn is_all_black(&self) -> bool {
        match self {
            TaskSpec::None { .. } => false,
            TaskSpec::Animate { background, frames } =>
                background.is_all_black() && frames.iter().all(|frame| frame.is_all_black()),
            FromSvg { source } => !(COLOR_SVGS.contains(&&*source.to_string_lossy())),
            TaskSpec::MakeSemitransparent { .. } => true,
            PngOutput { base, .. } => base.is_all_black(),
            TaskSpec::Repaint { color, .. } => color.is_black_or_transparent(),
            TaskSpec::StackLayerOnColor { background, foreground } =>
                background.is_black_or_transparent() && foreground.is_all_black(),
            TaskSpec::StackLayerOnLayer { background, foreground } => background.is_all_black() && foreground.is_all_black(),
            TaskSpec::ToAlphaChannel { .. } => true
        }
    }
}

pub type TaskResultFuture<'b> = CloneableFutureWrapper<'b, TaskResult>;
pub type TaskToFutureGraphNodeMap<'a, Ix> = HashMap<&'a TaskSpec,NodeIndex<Ix>>;

impl TaskSpec {
    /// Converts this task to a [TaskResultFuture] if it's not already present, also does so
    /// recursively for this task's dependencies, and adds dependency->consumer edges. Returns the
    /// added or existing future.
    /// [existing_nodes] is used to track tasks already added to the graph so that they are reused
    /// if this task also consumes them. This task is added in case other tasks that depend on it
    /// are added later.
    pub fn add_to<'a, 'until_graph_built, E, Ix>(&'a self,
                                                 graph: &mut Dag<RefCell<TaskResultFuture>, E, Ix>,
                                                 existing_nodes: &mut
                                                    TaskToFutureGraphNodeMap<'until_graph_built, Ix>)
                                                 -> NodeIndex<Ix>
    where Ix: IndexType, E: Default, 'a: 'until_graph_built
    {
        let name: String = (&self).to_string();
        if let Some(existing_index) = existing_nodes.get(&self) {
            info!("Matched an existing node: {}", name);
            return *existing_index;
        }

        // Simplify redundant tasks first
        match self {
            TaskSpec::MakeSemitransparent {base, alpha} => {
                if *alpha == 1.0 {
                    return base.add_to(graph, existing_nodes);
                }
            },
            TaskSpec::Repaint {base, color} => {
                if *color == ComparableColor::BLACK
                        && let TaskSpec::ToAlphaChannel{base: base_of_base} = base.deref()
                        && base_of_base.is_all_black() {
                    return base_of_base.add_to(graph, existing_nodes);
                }
            },
            TaskSpec::StackLayerOnColor {background, foreground} => {
                if *background == ComparableColor::TRANSPARENT {
                    return foreground.add_to(graph, existing_nodes);
                }
            }
            _ => {}
        }

        info!("No existing node found for: {}", name);
        let mut dependencies: Vec<NodeIndex<Ix>> = Vec::with_capacity(16);
        let as_future: TaskResultFuture = match self {
            TaskSpec::None { .. } =>
                CloneableFutureWrapper::new(name, Box::pin(
                async { TaskResult::Err{value: anyhoo!("Call to into_future() on a None task") } }
                )),
            TaskSpec::Animate { background, frames } => {
                let background_index = background.add_to(graph, existing_nodes);
                let background_future = graph[background_index].get_mut().to_owned();
                dependencies.push(background_index);
                let mut frame_futures: Vec<TaskResultFuture> = Vec::with_capacity(frames.len());
                for frame in frames {
                    let frame_index = frame.add_to(graph, existing_nodes);
                    frame_futures.push(graph[frame_index].borrow().to_owned());
                    dependencies.push(frame_index);
                }
                CloneableFutureWrapper::new(name, Box::pin(async move {
                   animate(background_future, frame_futures).await
                }))
            },
            FromSvg { source } => {
                let source = source.to_owned();
                CloneableFutureWrapper::new(name, Box::pin(
                    async move { from_svg(&source, *TILE_SIZE) }))
            },
            TaskSpec::MakeSemitransparent { base, alpha } => {
                let alpha: f32 = (*alpha).into();
                let base_index = base.add_to(graph, existing_nodes);
                let base_future = graph[base_index].get_mut().to_owned();
                dependencies.push(base_index);
                CloneableFutureWrapper::new(name, Box::pin(
                    async move {
                        let base_result: Arc<AlphaChannel> = (&*base_future.await).try_into()?;
                        let mut channel = Arc::unwrap_or_clone(base_result);
                        make_semitransparent(&mut channel, alpha);
                        TaskResult::AlphaChannel {value: Arc::new(channel)}
                    }))
            },
            PngOutput { base, destinations } => {
                let destinations = destinations.to_owned();
                let base_index = base.add_to(graph, existing_nodes);
                let base_future = graph[base_index].get_mut().to_owned();
                dependencies.push(base_index);
                CloneableFutureWrapper::new(name, Box::pin(
                    async move {
                        let image: Arc<Pixmap> = (&*base_future.await).try_into()?;
                        png_output(image.deref(), &destinations)
                    }
                ))
            },
            TaskSpec::Repaint { base, color } => {
                let color = color.to_owned();
                let base_index = base.add_to(graph, existing_nodes);
                let base_future = graph[base_index].get_mut().to_owned();
                dependencies.push(base_index);
                CloneableFutureWrapper::new(name, Box::pin(async move {
                    let base_image: Arc<AlphaChannel> = (&*base_future.await).try_into()?;
                    TaskResult::Pixmap {value: Arc::new(paint(base_image.deref(), &color))}
                }))
            },
            TaskSpec::StackLayerOnColor { background, foreground } => {
                let background = background.to_owned();
                let fg_index = foreground.add_to(graph, existing_nodes);
                let fg_future = graph[fg_index].get_mut().to_owned();
                dependencies.push(fg_index);
                CloneableFutureWrapper::new(name, Box::pin(async move {
                    let fg_image: Arc<Pixmap> = (&*fg_future.await).try_into()?;
                    stack_layer_on_background(&background, &fg_image)
                }))
            },
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                let bg_index = background.add_to(graph, existing_nodes);
                dependencies.push(bg_index);
                let fg_index = foreground.add_to(graph, existing_nodes);
                dependencies.push(fg_index);
                let bg_future = graph[bg_index].get_mut().to_owned();
                let fg_future = graph[fg_index].get_mut().to_owned();
                CloneableFutureWrapper::new(name, Box::pin(async move {
                    let bg_image: Arc<Pixmap> = (&*bg_future.await).try_into()?;
                    let mut out_image = Arc::unwrap_or_clone(bg_image);
                    let fg_image: Arc<Pixmap> = (&*fg_future.await).try_into()?;
                    stack_layer_on_layer(&mut out_image, fg_image.deref());
                    let out_image_arc = Arc::new(out_image);
                    TaskResult::Pixmap {value: out_image_arc}
                }))
            },
            TaskSpec::ToAlphaChannel { base } => {
                let base_index = base.add_to(graph, existing_nodes);
                let base_future = graph[base_index].get_mut().to_owned();
                dependencies.push(base_index);
                CloneableFutureWrapper::new(name, Box::pin(async move {
                    let base_image: Arc<Pixmap> = (&*base_future.await).try_into()?;
                    to_alpha_channel(base_image.deref())
                }))
            }
        };
        let self_id = graph.add_node(RefCell::new(as_future));
        for dependency in dependencies {
            graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        existing_nodes.insert(self, self_id);
        self_id
    }
}

lazy_static! {
    pub static ref OUT_DIR: &'static Path = Path::new("./out/");
    pub static ref SVG_DIR: &'static Path = Path::new("./svg/");
}

pub fn name_to_out_path(name: &str) -> PathBuf {
    let mut out_file_path = OUT_DIR.to_path_buf();
    out_file_path.push(format!("{}.png", name));
    out_file_path
}

pub fn name_to_svg_path(name: &str) -> PathBuf {
    let mut svg_file_path = SVG_DIR.to_path_buf();
    svg_file_path.push(format!("{}.svg", name));
    svg_file_path
}

pub fn from_svg_task(name: &str) -> Box<TaskSpec> {
    Box::new(FromSvg {source: name_to_svg_path(name)})
}

pub fn paint_task(base: Box<TaskSpec>, color: ComparableColor) -> Box<TaskSpec> {
    Box::new(
        TaskSpec::Repaint {base: Box::new(TaskSpec::ToAlphaChannel { base }), color})
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> Box<TaskSpec> {
    paint_task(from_svg_task(name), color)
}

pub fn semitrans_svg_task(name: &str, alpha: f32) -> Box<TaskSpec> {
    Box::new(TaskSpec::MakeSemitransparent {base: Box::from(TaskSpec::ToAlphaChannel { base: from_svg_task(name) }),
        alpha: alpha.into()})
}

pub fn path(name: &str) -> Vec<PathBuf> {
    vec![name_to_out_path(name)]
}

pub fn out_task(name: &str, base: Box<TaskSpec>) -> Box<TaskSpec> {
    Box::new(PngOutput {base, destinations: path(name)})
}

#[macro_export]
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        Box::new($crate::image_tasks::task_spec::TaskSpec::StackLayerOnLayer {
            background: $first_layer.into(),
            foreground: $second_layer.into()
        })
    };
    ( $first_layer:expr, $second_layer:expr, $( $more_layers:expr ),+ ) => {{
        let mut layers_so_far = $crate::stack!($first_layer, $second_layer);
        $( layers_so_far = crate::stack!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

        #[macro_export]
        macro_rules! stack_on {
    ( $background:expr, $foreground:expr ) => {
        Box::new($crate::image_tasks::task_spec::TaskSpec::StackLayerOnColor {
            background: $background,
            foreground: $foreground.into()
        })
    };
    ( $background:expr, $first_layer:expr, $( $more_layers:expr ),+ ) => {{
        let mut layers_so_far = $crate::stack_on!($background, $first_layer);
        $( layers_so_far = crate::stack!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

        #[macro_export]
        macro_rules! paint_stack {
    ( $color:expr, $( $layers:expr ),* ) => {
        $crate::image_tasks::task_spec::paint_task(
            $crate::stack!($(crate::image_tasks::task_spec::from_svg_task($layers)),*).into(),
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
            base: match self {
                TaskSpec::ToAlphaChannel { .. } => {
                    Box::new(self)
                },
                _ => Box::new(TaskSpec::ToAlphaChannel { base: Box::new(self) })},
            alpha: OrderedFloat::from(rhs)
        }
    }
}

impl Mul<ComparableColor> for TaskSpec {
    type Output = TaskSpec;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        match &self {
            TaskSpec::ToAlphaChannel { .. } => {
                TaskSpec::Repaint {
                    base: Box::new(self),
                    color: rhs
                }
            },
            TaskSpec::MakeSemitransparent { .. } => {
                TaskSpec::Repaint {
                    base: Box::new(self),
                    color: rhs
                }
            },
            TaskSpec::Repaint { base, .. } => {
                TaskSpec::Repaint {
                    base: Box::new(*base.to_owned()),
                    color: rhs
                }
            },
            _ => TaskSpec::Repaint {
                base: Box::new(TaskSpec::ToAlphaChannel { base: Box::new(self) }),
                color: rhs
            }
        }
    }
}
