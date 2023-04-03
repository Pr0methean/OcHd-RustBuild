use std::any::TypeId;
use fn_graph::{DataAccessDyn, FnGraphBuilder, FnId, TypeIds};
use resman::{FnRes, IntoFnRes, IntoFnResource, Resources};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::ops::Mul;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use cached::lazy_static::lazy_static;
use ordered_float::OrderedFloat;
use tiny_skia::Pixmap;
use crate::image_tasks::from_svg::from_svg;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::png_output::png_output;
use crate::image_tasks::repaint::AlphaChannel;
use crate::image_tasks::animate::animate;
use crate::image_tasks::repaint::paint;
use crate::image_tasks::stack::stack;
use crate::image_tasks::task_spec::TaskSpec::{FromSvg, PngOutput};
use crate::image_tasks::task_spec::TaskSpecDecorator::{MakeSemitransparent, Repaint};

/// Specification of a task that produces and/or consumes at least one [Pixmap]. Created
/// to de-duplicate copies of the same task, since function closures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum TaskSpec {
    None {},
    Animate {background: Arc<TaskSpec>, frames: Vec<Arc<TaskSpec>>},
    FromSvg {source: PathBuf},
    MakeSemitransparent {base: Arc<TaskSpec>, alpha: OrderedFloat<f32>},
    PngOutput {base: Arc<TaskSpec>, destinations: Arc<Vec<PathBuf>>},
    Repaint {base: Arc<TaskSpec>, color: ComparableColor},
    Stack {background: ComparableColor, layers: Vec<Arc<TaskSpec>>},
    ToAlphaChannel {base: Arc<TaskSpec>}
}

const PIXMAP: TypeId = TypeId::of::<Pixmap>();

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
            TaskSpec::Stack { background, layers } => {
                write!(f, "{{{};{}}}", background, layers.iter()
                    .map(|spec| spec.to_string())
                    .collect::<Vec<String>>().as_slice().join(","))
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

impl <'a, 'b> TaskSpec {
    pub fn add_to<F>(&self,
                     graph: &mut FnGraphBuilder<&'a F>,
                     existing_nodes: &'b mut HashMap<&'a TaskSpec, FnId>,
                     tile_width: u32) -> &FnId {
        if existing_nodes.contains_key(self) {
            return existing_nodes.get(self).unwrap();
        }
        let name = self.to_string();
        match self {
            TaskSpec::Animate { background, frames } => {
                let frame_count = frames.len();
                let background_id = background.add_to(graph, existing_nodes, tile_width);
                let frame_ids: Vec<&FnId> = frames.iter()
                    .map(|frame| frame.add_to(graph, existing_nodes, tile_width))
                    .collect();
                let animate_id = graph.add_fn(animate.into_fn_res());
                let frame_asset_strings: Vec<String> = (0..frame_count)
                    .map(|index| format!("{name}::frame{index}"))
                    .collect();
                let background_asset_string = format!("{}::background", name);
                let mut input_asset_strings = vec![background_asset_string.to_owned()];
                input_asset_strings.extend_from_slice(&frame_asset_strings[..]);
                let input_asset_s = input_asset_strings.to_owned();
                let output_asset_string = format!("{}::output", name);
                let output_asset_s = output_asset_string.to_owned();
                let frame_count_ = frame_count;
                let node = Node::new(name.to_string(), move |solver: &mut GraphSolver| {
                    let (background_asset_string, frame_asset_strings) =
                        input_asset_s.split_first().unwrap();
                    let background_pixmap: Pixmap =
                        solver.get_value::<Pixmap>(&background_asset_string)?;
                    let frame_pixmaps: Vec<Pixmap> = frame_asset_strings.iter()
                        .map(|asset_string| (Self::get_pixmap(solver, &*asset_string))).collect();
                    if !solver.input_is_new(&background_pixmap, &background_asset_string.to_string())
                        && (0..frame_count_).any(|index| {
                            solver.input_is_new(&frame_pixmaps[index], &frame_asset_strings[index].to_string())
                    }) {
                        let outs = vec!(output_asset_s.to_owned());
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let output = animate(background_pixmap, Box::new(frame_pixmaps.into_iter())).unwrap();
                    solver.save_value(&output_asset_s, output);
                    return Ok(SolverStatus::Executed);
                }, input_asset_strings.to_owned(), vec!(output_asset_string.to_owned()));
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", background), &*background_asset_string).unwrap();
                for index in 0..frame_count {
                    let source = &format!("{}::output", frames[index]);
                    let sink = &frame_asset_strings[index];
                    graph.bind_asset(source, sink).unwrap();
                };
            },
            FromSvg { source } => {
                let tile_width_ = tile_width;
                let node = create_node!(name: name.to_string(), (source: PathBuf) -> (output: Pixmap)
                    output = from_svg(source, tile_width_).unwrap()
                );
                graph.add_node(node).unwrap();
                graph.define_freestanding_asset(&*format!("{}::source", self),
                                                source.to_owned()).unwrap_or_default();
            },
            TaskSpec::MakeSemitransparent { base, alpha } => {
                base.add_to(graph, existing_nodes, tile_width);
                let alpha_ = alpha.into_inner();
                let node = create_node!(name: name.to_string(), (base: Pixmap) -> (output: Pixmap)
                    output = make_semitransparent(base, alpha_).unwrap()
                );
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            PngOutput { base, destinations } => {
                base.add_to(graph, existing_nodes, tile_width);
                graph.add_node::<dyn NodeType<Output=()>>(create_node!(name: name.to_string(),
                    (destinations: Vec<PathBuf>, base: Pixmap) -> () {
                        png_output(base, destinations).unwrap();
                })).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
                graph.define_freestanding_asset(&*format!("{}::destinations", self),
                        destinations.to_owned()).unwrap_or_default();
            },
            TaskSpec::Repaint { base, color } => {
                base.add_to(graph, existing_nodes, tile_width);
                let color_ = color.to_owned();
                let node = create_node!(name: name.to_string(), (base: AlphaChannel) -> (output: Pixmap) {
                    output = paint(base, color_).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::Stack { background, layers } => {
                let layer_count = layers.len();
                for layer in layers {
                    layer.add_to(graph, existing_nodes, tile_width.to_owned());
                }
                let layer_asset_strings: Vec<String> = (0..layer_count)
                    .map(|index| format!("{name}::frame{index}"))
                    .collect();
                let layer_asset_s = layer_asset_strings.to_owned();
                let output_asset_string = format!("{}::output", name);
                let output_asset_s = output_asset_string.to_owned();
                let layer_count_ = layer_count;
                let background_ = background.to_owned();
                let node = Node::new(name.to_string(), move |solver: &mut GraphSolver| {
                    let layer_pixmaps: Vec<Pixmap> = layer_asset_strings.iter().map(|asset_string| {
                        solver.get_value::<Pixmap>(asset_string).unwrap()
                    }).collect();
                    if !(0..layer_count_).any(|index| {
                        solver.input_is_new(&layer_pixmaps[index], &layer_asset_strings[index])
                    }) {
                        let outs = vec!(output_asset_s.to_owned());
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let output = stack(background_, Box::new(layer_pixmaps.into_iter())).unwrap();
                    solver.save_value(&output_asset_s, output);
                    return Ok(SolverStatus::Executed);
                }, layer_asset_s.to_owned(), vec!(output_asset_string.to_owned()));
                graph.add_node(node).unwrap();
                for index in 0..layer_count {
                    let source = &format!("{}::output", layers[index]);
                    let sink = &layer_asset_s[index];
                    graph.bind_asset(source, sink).unwrap();
                };
            },
            TaskSpec::ToAlphaChannel { base } => {
                base.add_to(graph, existing_nodes, tile_width);
                let node =
                    create_node!(name: name.to_string(), (base: Pixmap) -> (output: AlphaChannel) {
                        output = AlphaChannel::from(&base)
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::None {} => {
                panic!("Attempted to add a None task to graph");
            }
        }
        existing_nodes.insert(self);
    }

    fn get_pixmap(solver: &mut GraphSolver, asset_string: &str) -> Pixmap {
        solver.get_value::<Pixmap>(&asset_string).unwrap()
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

pub fn from_svg_task(name: &str) -> Arc<TaskSpec> {
    return Arc::new(FromSvg {source: name_to_svg_path(name)});
}

pub fn repaint_task(base: Arc<TaskSpec>, color: ComparableColor) -> Arc<TaskSpec> {
    return Arc::new(TaskSpec::Repaint {base, color});
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> Arc<TaskSpec> {
    return repaint_task(from_svg_task(name), color);
}

pub fn semitrans_svg_task(name: &str, alpha: f32) -> Arc<TaskSpec> {
    return Arc::new(TaskSpec::MakeSemitransparent {base: from_svg_task(name),
            alpha: alpha.into()});
}

pub fn path(name: &str) -> Arc<Vec<PathBuf>> {
    return Arc::new(vec!(name_to_out_path(name)));
}

pub fn out_task(name: &str, base: Arc<TaskSpec>) -> Arc<TaskSpec> {
    return Arc::new(PngOutput {base, destinations: path(name)});
}

#[macro_export]
macro_rules! stack_on {
    ( $background:expr, $( $layers:expr ),* ) => {
        std::sync::Arc::new(crate::image_tasks::task_spec::TaskSpec::Stack {
            background: $background.to_owned(),
            layers: vec![$($layers),*]
        })
    }
}

#[macro_export]
macro_rules! stack {
    ( $( $layers:expr ),* ) => {
        crate::stack_on!(crate::image_tasks::color::ComparableColor::TRANSPARENT, $($layers),*)
    }
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
            base: Arc::from(self),
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
                    base: Arc::from(clone),
                    color: rhs
                }
            },
            _ => TaskSpec::Repaint {
                base: Arc::from(TaskSpec::ToAlphaChannel { base: self.into() }),
                color: rhs
            }
        };
    }
}

pub enum TaskSpecDecorator {
    MakeSemitransparent {alpha: OrderedFloat<f32>},
    Repaint {color: ComparableColor}
}

impl TaskSpecDecorator {
    fn apply(&self, base: TaskSpec) -> TaskSpec {
        return match self {
            MakeSemitransparent { alpha }
                => TaskSpec::MakeSemitransparent {base: Arc::new(base), alpha: alpha.to_owned() },
            Repaint { color }
                => TaskSpec::Repaint {base: Arc::new(base), color: color.to_owned()}
        }
    }
}

impl From<ComparableColor> for TaskSpecDecorator {
    fn from(value: ComparableColor) -> Self {
        Repaint {color: value}
    }
}

impl From<f32> for TaskSpecDecorator {
    fn from(value: f32) -> Self {
        MakeSemitransparent {alpha: value.into()}
    }
}
