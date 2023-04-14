
use std::collections::{HashMap};
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};

use std::ops::{Deref, FromResidual, Mul};
use std::path::{Path, PathBuf};

use std::str::FromStr;
use std::sync::{Arc, LazyLock};

use anyhow::{Error};

use cached::lazy_static::lazy_static;

use fn_graph::{DataAccessDyn, TypeIds};
use fn_graph::daggy::Dag;
use futures::{FutureExt};



use log::{info};
use ordered_float::OrderedFloat;
use petgraph::adj::DefaultIx;
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

pub type TaskResultLazy = Arc<LazyLock<TaskResult, Box<dyn FnOnce() -> TaskResult + Send>>>;

#[derive(Clone)]
pub struct TaskSpecNodeInfo {
    pub(crate) lazy: TaskResultLazy,
    pub(crate) node_id: NodeIndex<DefaultIx>
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
            TaskResult::Pixmap { .. } => Err(anyhoo!("Tried to cast a Pixmap result to AlphaChannel")),
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

pub type TaskGraph = Dag<TaskSpec, (), DefaultIx>;
pub type TaskToGraphNodeMap = HashMap<TaskSpec,TaskSpecNodeInfo>;

impl TaskSpec {
    pub fn dependencies(&self) -> Vec<&TaskSpec> {
        match self {
            TaskSpec::None { .. } => panic!("dependencies() called on None task"),
            TaskSpec::Animate { background, frames } => {
                let mut deps: Vec<&TaskSpec> = Vec::with_capacity(frames.len() + 1);
                deps.push(background);
                frames.iter().map(|task_box| &*task_box).collect_into(&mut deps);
                deps
            }
            FromSvg { .. } => vec![],
            TaskSpec::MakeSemitransparent { base, .. } => vec![base],
            PngOutput { base, .. } => vec![base],
            TaskSpec::Repaint { base, .. } => vec![base],
            TaskSpec::StackLayerOnColor { foreground, .. } => vec![foreground],
            TaskSpec::StackLayerOnLayer { background, foreground } => vec![background, foreground],
            TaskSpec::ToAlphaChannel { base, .. } => vec![base]
        }
    }

    /// Adds this task to the given graph if it's not already present.
    /// [existing_nodes] is used to track tasks already added to the graph so that they are reused
    /// if this task also consumes them. This task is added in case other tasks that depend on it
    /// are added later.
    pub fn add_to(&self,
                     graph: &mut TaskGraph,
                     existing_nodes: &mut TaskToGraphNodeMap)
                                         -> TaskSpecNodeInfo
    {
        let name: String = (&self).to_string();
        if let Some(existing_node) = existing_nodes.get(&self) {
            info!("Matched an existing node: {}", name);
            return existing_node.to_owned();
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
        let node_id = graph.add_node(self.to_owned());
        let dependencies = self.dependencies();
        let mut dependency_infos = Vec::with_capacity(dependencies.len());
        for dependency in dependencies {
            let dependency_info = dependency.add_to(graph, existing_nodes);
            graph.add_edge(dependency_info.node_id, node_id, ())
                .expect("Tried to create a cycle");
            dependency_infos.push(dependency_info);
        }
        let lazy: TaskResultLazy = Arc::new(LazyLock::new(match self {
                TaskSpec::None { .. } => panic!("Tried to invoke None task"),
                TaskSpec::Animate { .. } => {
                    let background_lazy = dependency_infos[0].lazy.to_owned();
                    let frame_lazies = dependency_infos[1..].into_iter()
                        .map(|info| info.lazy.to_owned()).collect();
                    Box::new(move || animate(background_lazy, frame_lazies))
                }
                FromSvg { source } => {
                    let source = source.to_owned();
                    Box::new(move || from_svg(&source, *TILE_SIZE))
                },
                TaskSpec::MakeSemitransparent { alpha, .. } => {
                    let base_lazy = dependency_infos[0].lazy.to_owned();
                    let alpha = alpha.0;
                    Box::new(move || {
                        let mut result: AlphaChannel = Arc::unwrap_or_clone((&**base_lazy).try_into()?);
                        make_semitransparent(&mut result, alpha);
                        TaskResult::AlphaChannel {value: Arc::new(result)}
                    })
                }
                PngOutput { destinations, .. } => {
                    let base_lazy = dependency_infos[0].lazy.to_owned();
                    let destinations = destinations.to_owned();
                    Box::new(move || {
                        let base: Arc<Pixmap> = (&**base_lazy).try_into()?;
                        png_output(&*base, &destinations)
                    })
                }
                TaskSpec::Repaint { color, .. } => {
                    let base_lazy = dependency_infos[0].lazy.to_owned();
                    let color = color.to_owned();
                    Box::new(move || {
                        let base: Arc<AlphaChannel> = (&**base_lazy).try_into()?;
                        TaskResult::Pixmap {
                            value: Arc::from(paint(&*base, &color)) }})
                }
                TaskSpec::StackLayerOnColor { background, .. } => {
                    let fg_lazy = dependency_infos[0].lazy.to_owned();
                    let background = background.to_owned();
                    Box::new(move || {
                        let foreground: Arc<Pixmap> = (&**fg_lazy).try_into()?;
                        stack_layer_on_background(&background, &*foreground)
                    })
                }
                TaskSpec::StackLayerOnLayer { .. } => {
                    let bg_lazy = dependency_infos[0].lazy.to_owned();
                    let fg_lazy = dependency_infos[1].lazy.to_owned();
                    Box::new(move || {
                        let mut result: Pixmap = Arc::unwrap_or_clone((&**bg_lazy).try_into()?);
                        let foreground: Arc<Pixmap> = (&**fg_lazy).try_into()?;
                        stack_layer_on_layer(&mut result, &*foreground);
                        TaskResult::Pixmap { value: Arc::new(result) }
                    })
                }
                TaskSpec::ToAlphaChannel { .. } => {
                    let base_lazy = dependency_infos[0].lazy.to_owned();
                    Box::new(move || {
                        let base: Arc<Pixmap> = (&**base_lazy).try_into()?;
                        to_alpha_channel(&*base)
                    })
                }
            }));
        let info = TaskSpecNodeInfo {lazy, node_id};
        let info_copy = info.to_owned();
        existing_nodes.insert(self.to_owned(), info_copy);
        info
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
            _ => TaskSpec::Repaint {
                base: Box::new(TaskSpec::ToAlphaChannel { base: Box::new(self) }),
                color: rhs
            }
        }
    }
}
