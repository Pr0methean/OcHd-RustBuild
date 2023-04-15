use std::collections::{HashMap};

use std::fmt::{Debug, Display, Formatter};
use std::future::{Future};
use std::hash::Hash;
use std::marker::Destruct;

use std::ops::{Deref, DerefMut, Mul};
use std::path::{Path, PathBuf};
use std::pin::{Pin};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use anyhow::{Error};

use cached::lazy_static::lazy_static;
use cooked_waker::{IntoWaker, ViaRawPointer, WakeRef};
use crate::anyhoo;
use fn_graph::{DataAccessDyn, TypeIds};
use fn_graph::daggy::Dag;
use futures::{FutureExt};
use itertools::Itertools;


use log::{error, info};
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
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_layer_on_background, stack_layer_on_layer};
use crate::TILE_SIZE;

pub trait TaskSpecTraits <T>: Clone + Debug + Display + Ord + Eq + Hash {
    fn add_to<'a, 'until_graph_built, E, Ix>(&'until_graph_built self,
                                             ctx: &mut
                                             TaskGraphBuildingContext<'until_graph_built, 'a, E, Ix>)
                                             -> (NodeIndex<Ix>, TaskResultFuture<'a, T>)
        where Ix : IndexType, E: Default, 'a : 'until_graph_built;
}

impl TaskSpecTraits<Pixmap> for ToPixmapTaskSpec {
    fn add_to<'a, 'until_graph_built, E, Ix>(&'until_graph_built self,
                                             ctx: &mut
                                             TaskGraphBuildingContext<'until_graph_built, 'a, E, Ix>)
                                             -> (NodeIndex<Ix>, TaskResultFuture<'a, Pixmap>)
                                            where Ix : IndexType, E: Default, 'a : 'until_graph_built {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future)) = ctx.pixmap_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        let mut dependencies: Vec<NodeIndex<Ix>> = Vec::with_capacity(16);
        let future: SyncBoxFuture<Result<Box<Pixmap>,CloneableError>> = match self {
            ToPixmapTaskSpec::None { .. } => panic!("Tried to add None task to graph"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let (background_index, background_future) = background.add_to(ctx);
                dependencies.push(background_index);
                let mut frame_futures: Vec<TaskResultFuture<Pixmap>> = Vec::with_capacity(frames.len());
                for frame in frames {
                    let (frame_index, frame_future) = frame.add_to(ctx);
                    frame_futures.push(frame_future);
                    dependencies.push(frame_index);
                }
                Box::pin(async move {
                    let background: Arc<Box<Pixmap>> = background_future.await?.into();
                    Ok(Box::new(animate(&background, frame_futures).await?))
                })
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source = source.to_owned();
                Box::pin(
                    async move { Ok(Box::new(from_svg(&source, *TILE_SIZE)?)) }
                )
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                if *background == ComparableColor::TRANSPARENT {
                    return foreground.add_to(ctx);
                }
                let background = background.to_owned();
                let (fg_index, fg_future) = foreground.add_to(ctx);
                dependencies.push(fg_index);
                Box::pin(async move {
                    let fg_image: Arc<Box<Pixmap>> = fg_future.await?.into();
                    Ok(Box::new(stack_layer_on_background(&background, &fg_image)?))
                })
            },
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                let (bg_index, bg_future) = background.add_to(ctx);
                dependencies.push(bg_index);
                let (fg_index, fg_future) = foreground.add_to(ctx);
                dependencies.push(fg_index);
                Box::pin(async move {
                    let bg_image: Arc<Box<Pixmap>> = bg_future.await?.into();
                    let mut out_image = Arc::unwrap_or_clone(bg_image);
                    let fg_image: Arc<Box<Pixmap>> = fg_future.await?.into();
                    stack_layer_on_layer(&mut out_image, fg_image.deref());
                    Ok(out_image)
                })
            },
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                if *color == ComparableColor::BLACK
                        && let ToAlphaChannelTaskSpec::FromPixmap {base: base_of_base} = base.deref()
                        && base_of_base.is_all_black() {
                    return base_of_base.add_to(ctx);
                }
                let color = color.to_owned();
                let (base_index, base_future) = base.add_to(ctx);
                dependencies.push(base_index);
                Box::pin(async move {
                    let base_image: Arc<Box<AlphaChannel>> = base_future.await?;
                    Ok(Box::new(paint(&*base_image, &color)))
                })
            },
        };
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        let wrapped_future = CloneableFutureWrapper::new(name, future);
        ctx.pixmap_task_to_future_map.insert(self, (self_id, wrapped_future.to_owned()));
        (self_id, wrapped_future)
    }
}

impl TaskSpecTraits<AlphaChannel> for ToAlphaChannelTaskSpec {
    fn add_to<'a, 'until_graph_built, E, Ix>(&'until_graph_built self,
                                             ctx: &mut
                                             TaskGraphBuildingContext<'until_graph_built, 'a, E, Ix>)
                                             -> (NodeIndex<Ix>, TaskResultFuture<'a, AlphaChannel>)
        where Ix : IndexType, E: Default, 'a : 'until_graph_built {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future))
                = ctx.alpha_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        let mut dependencies: Vec<NodeIndex<Ix>> = Vec::with_capacity(16);
        let future: SyncBoxFuture<Result<Box<AlphaChannel>,CloneableError>> = match self {
                ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                if *alpha == 1.0 {
                    return base.add_to(ctx);
                }
                let alpha: f32 = (*alpha).into();
                let (base_index, base_future) = base.add_to(ctx);
                dependencies.push(base_index);
                Box::pin(
                    async move {
                        let base_result: Arc<Box<AlphaChannel>> = base_future.await?.into();
                        let mut channel = Arc::unwrap_or_clone(base_result);
                        make_semitransparent(&mut channel, alpha);
                        Ok(channel)
                    }
                )
            },
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                let (base_index, base_future) = base.add_to(ctx);
                dependencies.push(base_index);
                Box::pin(async move {
                    let base_image: Arc<Box<Pixmap>> = base_future.await?.into();
                    Ok(Box::new(to_alpha_channel(base_image.deref())?))
                })
            },
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { background, foreground } => {
                let (bg_index, bg_future) = background.add_to(ctx);
                let (fg_index, fg_future) = foreground.add_to(ctx);
                dependencies.push(bg_index);
                dependencies.push(fg_index);
                Box::pin(async move {
                    let mut bg_image = Arc::unwrap_or_clone(bg_future.await?);
                    stack_alpha_on_alpha(&mut bg_image, &*fg_future.await?);
                    Ok(bg_image)
                })
            }
        };
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        let wrapped_future = CloneableFutureWrapper::new(name, future);
        ctx.alpha_task_to_future_map.insert(self, (self_id, wrapped_future.to_owned()));
        (self_id, wrapped_future)
    }
}

impl TaskSpecTraits<()> for SinkTaskSpec {
    fn add_to<'a, 'until_graph_built, E, Ix>(&'until_graph_built self,
                                             ctx: &mut
                                                 TaskGraphBuildingContext<'until_graph_built, 'a, E, Ix>)
                                             -> (NodeIndex<Ix>, TaskResultFuture<'a, ()>)
        where Ix : IndexType, E: Default, 'a : 'until_graph_built {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future))
                = ctx.output_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        let mut dependencies: Vec<NodeIndex<Ix>> = Vec::with_capacity(16);
        let future: SyncBoxFuture<Result<Box<()>,CloneableError>> = match self {
            SinkTaskSpec::PngOutput {base, destinations} => {
                let destinations = destinations.to_owned();
                let (base_index, base_future) = base.add_to(ctx);
                dependencies.push(base_index);
                Box::pin(async move {
                    Ok(Box::new(png_output(&*base_future.await?, &destinations)?))
                })
            }
        };
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        let wrapped_future = CloneableFutureWrapper::new(name, future);
        ctx.output_task_to_future_map.insert(self, (self_id, wrapped_future.to_owned()));
        (self_id, wrapped_future)
    }
}

pub type TaskResultFuture<'a, T> = CloneableFutureWrapper<'a, T>;
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
    StackAlphaOnAlpha {background: Box<ToAlphaChannelTaskSpec>, foreground: Box<ToAlphaChannelTaskSpec>},
}

/// [TaskSpec] for a task that doesn't produce a heap object as output.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum SinkTaskSpec {
    PngOutput {base: Box<ToPixmapTaskSpec>, destinations: Vec<PathBuf>},
}

/// Specification of a task that produces one of several output types. Created so that
/// copies of the same task created for different [Material] instances can be deduplicated, since
/// function closures and futures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum TaskSpec {
    ToPixmapTaskSpec(ToPixmapTaskSpec),
    ToAlphaChannelTaskSpec(ToAlphaChannelTaskSpec),
    SinkTaskSpec(SinkTaskSpec)
}

impl Display for TaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let inner: Box<&dyn Display> = Box::new(match self {
            TaskSpec::ToPixmapTaskSpec(inner) => inner,
            TaskSpec::ToAlphaChannelTaskSpec(inner) => inner,
            TaskSpec::SinkTaskSpec(inner) => inner
        });
        inner.fmt(f)
    }
}

impl From<&ToPixmapTaskSpec> for TaskSpec {
    fn from(value: &ToPixmapTaskSpec) -> Self {
        TaskSpec::ToPixmapTaskSpec(value.to_owned())
    }
}

impl From<&ToAlphaChannelTaskSpec> for TaskSpec {
    fn from(value: &ToAlphaChannelTaskSpec) -> Self {
        TaskSpec::ToAlphaChannelTaskSpec(value.to_owned())
    }
}

impl From<&SinkTaskSpec> for TaskSpec {
    fn from(value: &SinkTaskSpec) -> Self {
        TaskSpec::SinkTaskSpec(value.to_owned())
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

impl Display for SinkTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*match self {
            SinkTaskSpec::PngOutput { base: _base, destinations } => {
                destinations.iter().map(|dest|
                    match dest.file_name() {
                        None => "Unknown PNG file",
                        Some(name) => name.to_str().unwrap()
                    }
                ).join(",")
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
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha {background, foreground} => {
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

pub enum CloneableFutureWrapperState<'a, T> where T: ?Sized {
    Upcoming {
        future: SyncBoxFuture<'a, Result<Box<T>,CloneableError>>,
        multiwaker: Arc<MultiWaker>,
        waker: Arc<Waker>
    },
    Finished {
        result: Arc<Box<T>>
    },
    Err {
        error: Arc<CloneableError>
    }
}

/// Wraps a future so it can be consumed by multiple dependencies. Implemented because I was told
/// [futures::future::Shared] would probably have a memory leak. Unlike Shared, this doesn't
/// directly use a [std::cell::UnsafeCell].
#[derive(Debug)]
pub struct CloneableFutureWrapper<'a, T> where T: ?Sized + 'static {
    state: Arc<Mutex<CloneableFutureWrapperState<'a, T>>>
}

impl <'a, T> Clone for CloneableFutureWrapper<'a, T> where T: ?Sized {
    fn clone(&self) -> Self {
        CloneableFutureWrapper {state: self.state.to_owned()}
    }

    fn clone_from(&mut self, source: &Self) where Self: Destruct {
        self.state = source.state.to_owned();
    }
}

impl <'a, T: 'a> Debug for CloneableFutureWrapperState<'a, T> where T: ?Sized + 'static {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneableFutureWrapperState::Upcoming { waker, .. } =>
                f.debug_struct("Upcoming").field("waker", waker).finish(),
            CloneableFutureWrapperState::Finished { .. } => {
                f.write_str("Finished")
            },
            CloneableFutureWrapperState::Err { error } => f.debug_struct("Err").field("error", error).finish()
        }
    }
}

impl <'a, T> CloneableFutureWrapper<'a, T> where T: Debug + Sync + Send + ?Sized + 'a {
    pub fn new(name: String, base: SyncBoxFuture<'a, Result<Box<T>, CloneableError>>) -> CloneableFutureWrapper<'a, T> {
        let waker = Arc::new(MultiWaker::new());
        CloneableFutureWrapper {
            state: Arc::new(Mutex::new(CloneableFutureWrapperState::Upcoming {
                future: Box::pin(async move {
                    info!("Starting {}", name);
                    let result = base.await;
                    match &result {
                        Ok(..) => {
                            info!("Finishing {}", name);
                        }
                        Err(error) => {
                            error!("Error in {}: {}", name, error.message);
                        }
                    }
                    result
                }),
                multiwaker: waker.to_owned(),
                waker: Arc::new(waker.into_waker())
            }))
        }
    }
}

impl <'a, T> Future for CloneableFutureWrapper<'a, T> where T: 'a + ?Sized {
    type Output = CloneableResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match state.deref_mut() {
            CloneableFutureWrapperState::Finished { result } => Poll::Ready(Ok(result.to_owned())),
            CloneableFutureWrapperState::Err { error } => Poll::Ready(Err(anyhoo!(error.message.to_owned()))),
            CloneableFutureWrapperState::Upcoming { future, multiwaker, waker} => {
                let new_result = future.poll_unpin(
                    &mut Context::from_waker(waker.deref()));
                if let Poll::Ready(finished_result) = new_result {
                    match finished_result {
                        Ok(successful_result) => {
                            let result_arc: Arc<Box<T>> = Arc::from(successful_result);
                            waker.wake_by_ref();
                            *state = CloneableFutureWrapperState::Finished {result: result_arc.to_owned()};
                            Poll::Ready(Ok(result_arc))
                        },
                        Err(error) => {
                            waker.wake_by_ref();
                            *state = CloneableFutureWrapperState::Err {error: Arc::new(error.to_owned())};
                            Poll::Ready(Err(error))
                        }
                    }
                } else {
                    multiwaker.add_waker(cx.waker().to_owned());
                    Poll::Pending
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
pub struct TaskGraphBuildingContext<'a, 'b, E, Ix> where Ix: IndexType, 'b : 'a {
    pub graph: TaskGraph<E, Ix>,
    pixmap_task_to_future_map: HashMap<&'a ToPixmapTaskSpec, (NodeIndex<Ix>, TaskResultFuture<'b, Pixmap>)>,
    alpha_task_to_future_map: HashMap<&'a ToAlphaChannelTaskSpec, (NodeIndex<Ix>, TaskResultFuture<'b, AlphaChannel>)>,
    pub output_task_to_future_map: HashMap<&'a SinkTaskSpec, (NodeIndex<Ix>, TaskResultFuture<'b, ()>)>
}

impl <'a,'b,E,Ix> TaskGraphBuildingContext<'a,'b,E,Ix> where Ix: IndexType {
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

pub fn from_svg_task(name: &str) -> Box<ToPixmapTaskSpec> {
    Box::new(ToPixmapTaskSpec::FromSvg {source: name_to_svg_path(name)})
}

pub fn svg_alpha_task(name: &str) -> Box<ToAlphaChannelTaskSpec> {
    Box::new(ToAlphaChannelTaskSpec::from(*from_svg_task(name)))
}


pub fn paint_task(base: Box<ToAlphaChannelTaskSpec>, color: ComparableColor) -> Box<ToPixmapTaskSpec> {
    Box::new(
        ToPixmapTaskSpec::PaintAlphaChannel {base, color})
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> Box<ToPixmapTaskSpec> {
    paint_task(Box::new(ToAlphaChannelTaskSpec::FromPixmap { base: from_svg_task(name) }),
               color)
}

pub fn semitrans_svg_task(name: &str, alpha: f32) -> Box<ToAlphaChannelTaskSpec> {
    Box::new(ToAlphaChannelTaskSpec::MakeSemitransparent {base: Box::from(ToAlphaChannelTaskSpec::FromPixmap { base: from_svg_task(name) }),
        alpha: alpha.into()})
}

pub fn path(name: &str) -> Vec<PathBuf> {
    vec![name_to_out_path(name)]
}

pub fn out_task(name: &str, base: Box<ToPixmapTaskSpec>) -> Box<SinkTaskSpec> {
    Box::new(SinkTaskSpec::PngOutput {base, destinations: path(name)})
}

#[macro_export]
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        Box::new($crate::image_tasks::task_spec::ToPixmapTaskSpec::StackLayerOnLayer {
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
macro_rules! stack_alpha {
    ( $first_layer:expr, $second_layer:expr ) => {
        Box::new($crate::image_tasks::task_spec::ToAlphaChannelTaskSpec::StackAlphaOnAlpha {
            background: $first_layer.into(),
            foreground: $second_layer.into()
        })
    };
    ( $first_layer:expr, $second_layer:expr, $( $more_layers:expr ),+ ) => {{
        let mut layers_so_far = $crate::stack_alpha!($first_layer, $second_layer);
        $( layers_so_far = crate::stack_alpha!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

#[macro_export]
macro_rules! stack_on {
    ( $background:expr, $foreground:expr ) => {
        Box::new($crate::image_tasks::task_spec::ToPixmapTaskSpec::StackLayerOnColor {
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
            $crate::stack_alpha!(
                $(
                    crate::image_tasks::task_spec::svg_alpha_task($layers)
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
        *paint_task(Box::new(self), rhs)
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
