use std::collections::{HashSet};
use std::fmt::{Display, Formatter};
use std::ops::Mul;
use std::path::{Path, PathBuf};

use std::str::FromStr;
use std::sync::Arc;
use cached::lazy_static::lazy_static;
use ordered_float::OrderedFloat;
use rgraph::*;
use tiny_skia::Pixmap;
use crate::image_tasks::from_svg::from_svg;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::png_output::png_output;
use crate::image_tasks::repaint::AlphaChannel;
use crate::image_tasks::animate::animate;
use crate::image_tasks::repaint::paint;
use crate::image_tasks::stack::{stack_layer_on_background, stack_layer_on_layer};
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
    StackLayerOnColor {background: ComparableColor, foreground: Arc<TaskSpec>},
    StackLayerOnLayer {background: Arc<TaskSpec>, foreground: Arc<TaskSpec>},
    ToAlphaChannel {base: Arc<TaskSpec>}
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

trait NodeType: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError> {}

impl <'a, 'b> TaskSpec {
    pub fn add_to(&'a self, graph: &mut Graph,
                 existing_nodes: &'b mut HashSet<&'a TaskSpec>,
                 tile_width: u32) -> () {
        if existing_nodes.contains(self) {
            return;
        }
        let name = self;
        match self {
            TaskSpec::Animate { background, frames } => {
                let frame_count = frames.len();
                background.add_to(graph, existing_nodes, tile_width);
                for frame in frames {
                    frame.add_to(graph, existing_nodes, tile_width.to_owned());
                }
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
                graph.add_node(create_node!(name: name.to_string(),
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
            TaskSpec::StackLayerOnColor { background, foreground } => {
                foreground.add_to(graph, existing_nodes, tile_width);
                let node = create_node!(name: name.to_string(), (background: ComparableColor, foreground: Pixmap) -> (output: Pixmap) {
                    output = stack_layer_on_background(background, foreground).unwrap();
                });
                graph.add_node(node).unwrap();
            },
            TaskSpec::StackLayerOnLayer { background, foreground } => {
                background.add_to(graph, existing_nodes, tile_width);
                foreground.add_to(graph, existing_nodes, tile_width);
                let node = create_node!(name: name.to_string(), (background: Pixmap, foreground: Pixmap) -> (output: Pixmap) {
                    output = stack_layer_on_layer(background, foreground).unwrap();
                });
                graph.add_node(node).unwrap();
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
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr ) => {
        std::sync::Arc::new(crate::image_tasks::task_spec::TaskSpec::StackLayerOnLayer {
            background: $first_layer,
            foreground: $second_layer
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
            background: $background.to_owned(),
            foreground: $foreground
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
