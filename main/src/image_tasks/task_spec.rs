use std::collections::{HashMap};
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::future::{Future, IntoFuture};
use std::ops::{Deref, DerefMut, FromResidual, Mul};
use std::path::{Path, PathBuf};
use std::pin::{Pin};
use std::str::FromStr;
use std::sync::{Arc, PoisonError, Mutex};
use std::task::{Context, Poll, Waker};
use anyhow::Error;
use cached::lazy_static::lazy_static;
use chashmap_next::CHashMap;
use fn_graph::{DataAccessDyn, FnGraphBuilder, FnId, TypeIds};
use futures::{FutureExt};
use futures::future::{BoxFuture, ready};
use ordered_float::OrderedFloat;
use tiny_skia::Pixmap;


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
    Animate {background: Arc<TaskSpec>, frames: Vec<Arc<TaskSpec>>},
    FromSvg {source: PathBuf},
    MakeSemitransparent {base: Arc<TaskSpec>, alpha: OrderedFloat<f32>},
    PngOutput {base: Arc<TaskSpec>, destinations: Vec<PathBuf>},
    Repaint {base: Arc<TaskSpec>, color: ComparableColor},
    StackLayerOnColor {background: ComparableColor, foreground: Arc<TaskSpec>},
    StackLayerOnLayer {background: Arc<TaskSpec>, foreground: Arc<TaskSpec>},
    ToAlphaChannel {base: Arc<TaskSpec>}
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

#[derive(Clone)]
pub struct CloneableFutureWrapper<'a, T> where T: Clone + Send {
    result: Arc<Mutex<Option<T>>>,
    future: Arc<Mutex<BoxFuture<'a, T>>>,
    wakers: Arc<Mutex<Vec<Arc<Waker>>>>
}

impl <'a, T> CloneableFutureWrapper<'a, T> where T: Clone + Send {
    pub fn new<U>(base: U) -> CloneableFutureWrapper<'a, T>
            where U : Future<Output=T> + Send + 'a {
        return CloneableFutureWrapper {
            result: Arc::new(Mutex::new(None)),
            future: Arc::new(Mutex::new(Box::pin(base.into_future()))),
            wakers: Arc::new(Mutex::new(vec![]))
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
                                let mut wakers_lock = self.wakers.lock().unwrap();
                                let wakers = wakers_lock.deref_mut();
                                for waker in wakers.to_owned() {
                                    waker.wake_by_ref();
                                }
                                wakers.clear();
                                *locked_result = Some(new_result.to_owned());
                                Poll::Ready(new_result)
                            },
                            Poll::Pending => {
                                let mut wakers_lock = self.wakers.lock().unwrap();
                                let wakers = wakers_lock.deref_mut();
                                for waker in wakers.to_owned() {
                                    let _ = future.poll_unpin(&mut Context::from_waker(&waker));
                                }
                                wakers.push(Arc::new(cx.waker().to_owned()));
                                Poll::Pending
                            }
                        }
                    }
                }
            }
            Err(PoisonError {..}) => {
                panic!("Got a PoisonError while polling self")
            }
        }
    }
}

lazy_static!(
    static ref RESULTS_MAP: CHashMap<Arc<TaskSpec>, CloneableFutureWrapper<'static, TaskResult>> = CHashMap::new();
);

impl IntoFuture for TaskSpec {
    type Output = TaskResult;

    type IntoFuture = CloneableFutureWrapper<'static, TaskResult>;
    fn into_future<'a>(self) -> CloneableFutureWrapper<'static, TaskResult> {
        let owned_arc_self = Arc::new(self.to_owned());
        let second_owned_arc_self = owned_arc_self.clone();
        //let third_owned_arc_self = owned_arc_self.clone();
        RESULTS_MAP.upsert(owned_arc_self, move || {
            let name = self.to_string();
            let future_without_logging: BoxFuture<TaskResult> = match self {
                TaskSpec::None { .. } => Box::pin(
                    ready(TaskResult::Err{value: anyhoo!("Call to into_future() on a None task") })
                ),
                TaskSpec::Animate { background, frames } => {
                    let background = background.to_owned().as_future();
                    let frames: Vec<CloneableFutureWrapper<TaskResult>>
                        = frames.iter().map(|task_spec| task_spec.as_future()).collect();
                    Box::pin(background.then(|background| async {
                        match background {
                            TaskResult::Pixmap {value} => animate(value, frames).await,
                            _ => TaskResult::Err {value: anyhoo!("Got {:?} instead of Pixmap for background", background)}
                        }
                    }))
                },
                FromSvg { source } => {
                    let source = source.to_owned();
                    Box::pin(async { from_svg(source, *TILE_SIZE) })
                },
                TaskSpec::MakeSemitransparent { base, alpha } => {
                    let alpha = alpha.0.to_owned();
                    let base = base.as_future();
                    Box::pin(async move { make_semitransparent(base.await.try_into()?, alpha) })
                },
                PngOutput { base, destinations } => {
                    let base = base.as_future();
                    let destinations = destinations.to_owned();
                    Box::pin(async move  { png_output(base.await.try_into()?, &destinations) })
                },
                TaskSpec::Repaint { base, color } => {
                    let base = base.as_future();
                    let color = color.to_owned();
                    Box::pin(async move { paint(base.await.try_into()?, &color) })
                },
                TaskSpec::StackLayerOnColor { background, foreground } => {
                    let foreground = foreground.as_future();
                    let background = background.to_owned();
                    Box::pin(async move { stack_layer_on_background(&background, foreground.await.try_into()?) })
                },
                TaskSpec::StackLayerOnLayer { background, foreground } => {
                    let background = background.as_future();
                    let foreground = foreground.as_future();
                    Box::pin(async { stack_layer_on_layer(background.await.try_into()?, foreground.await.try_into()?) })
                },
                TaskSpec::ToAlphaChannel { base } => {
                    let base = base.as_future();
                    Box::pin(async { to_alpha_channel(base.await.try_into()?) })
                }
            };
            CloneableFutureWrapper::new(Box::pin(tokio::spawn(async move {
                println!("Starting task {}", name);
                let result = future_without_logging.await;
                //RESULTS_MAP.remove(&*third_owned_arc_self);
                println!("Finishing task {}", name);
                result
            }).map(Result::unwrap)))
        }, |&mut _| {});
        return RESULTS_MAP.get(&*second_owned_arc_self).unwrap().deref().clone();
    }
}

impl TaskSpec {
    pub fn as_future(&self) -> CloneableFutureWrapper<'static, TaskResult> {
        return self.clone().into_future();
    }

    pub fn add_to(&'static self,
                  graph: &mut FnGraphBuilder<&TaskSpec>,
                  existing_nodes: &mut HashMap<&TaskSpec, FnId>) -> FnId
    {
        if existing_nodes.contains_key(self) {
            println!("Matched an existing node: {}", self);
            return *existing_nodes.get(self).unwrap();
        }
        println!("No existing node found for: {}", self);
        let self_id: FnId = match self {
            TaskSpec::Animate { background, frames } => {
                let background_id = background.add_to(graph, existing_nodes);
                let mut frame_ids: Vec<FnId> = vec![];
                for frame in frames {
                    frame_ids.push(frame.add_to(graph, existing_nodes));
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
                let base_id = base.add_to(graph, existing_nodes);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            PngOutput { base, .. } => {
                let base_id = base.add_to(graph, existing_nodes);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::Repaint { base, .. } => {
                let base_id = base.add_to(graph, existing_nodes);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::StackLayerOnColor { foreground, .. } => {
                let base_id = foreground.add_to(graph, existing_nodes);
                let self_id = graph.add_fn(self);
                graph.add_edge(base_id, self_id).expect("Failed to add edge");
                self_id
            },
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                let background_id = background.add_to(graph, existing_nodes);
                let foreground_id = foreground.add_to(graph, existing_nodes);
                let self_id = graph.add_fn(self);
                graph.add_edge(background_id, self_id).expect("Failed to add background edge");
                graph.add_edge(foreground_id, self_id).expect("Failed to add foreground edge");
                self_id
            },
            TaskSpec::ToAlphaChannel { base } => {
                let base_id = base.add_to(graph, existing_nodes);
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

        pub fn from_svg_task(name: &str) -> Arc<TaskSpec> {
            return Arc::new(FromSvg {source: name_to_svg_path(name)});
        }

        pub fn paint_task(base: Arc<TaskSpec>, color: ComparableColor) -> Arc<TaskSpec> {
            return Arc::new(
                TaskSpec::Repaint {base: Arc::new(TaskSpec::ToAlphaChannel { base }), color});
        }

        pub fn paint_svg_task(name: &str, color: ComparableColor) -> Arc<TaskSpec> {
            return paint_task(from_svg_task(name), color);
        }

        pub fn semitrans_svg_task(name: &str, alpha: f32) -> Arc<TaskSpec> {
            return Arc::new(TaskSpec::MakeSemitransparent {base: from_svg_task(name),
                alpha: alpha.into()});
        }

        pub fn path(name: &str) -> Vec<PathBuf> {
            return vec![name_to_out_path(name)];
        }

        pub fn out_task(name: &str, base: Arc<TaskSpec>) -> Arc<TaskSpec> {
            return Arc::new(PngOutput {base, destinations: path(name)});
        }

        #[macro_export]
        macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        std::sync::Arc::new(crate::image_tasks::task_spec::TaskSpec::StackLayerOnLayer {
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
        std::sync::Arc::new(crate::image_tasks::task_spec::TaskSpec::StackLayerOnColor {
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
            base: Arc::new(self),
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
                    base: Arc::new(owned_self),
                    color: rhs
                }
            },
            _ => TaskSpec::Repaint {
                base: Arc::new(TaskSpec::ToAlphaChannel { base: Arc::new(self) }),
                color: rhs
            }
        };
    }
}
