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
use crate::image_tasks::stack::stack;

/// Specification of a task that produces and/or consumes at least one [Pixmap]. Created
/// to de-duplicate copies of the same task, since function closures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum TaskSpec {
    Animate {background: Arc<TaskSpec>, frames: Vec<Arc<TaskSpec>>},
    FromSvg {source: PathBuf},
    MakeSemitransparent {base: Arc<TaskSpec>, alpha: OrderedFloat<f32>},
    PngOutput {base: Arc<TaskSpec>, destinations: Arc<Vec<PathBuf>>},
    Repaint {base: Arc<TaskSpec>, color: ComparableColor},
    Stack {background: ComparableColor, layers: Vec<Arc<TaskSpec>>},
    ToAlphaChannel {base: Arc<TaskSpec>}
}

impl Display for TaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskSpec::Animate { background, frames: _frames } => {
                write!(f, "Animate({};", background)
            }
            TaskSpec::FromSvg { source } => {
                write!(f, "{}", source.to_string_lossy())
            }
            TaskSpec::MakeSemitransparent { base, alpha } => {
                write!(f, "{}@{}", base, alpha)
            }
            TaskSpec::PngOutput { base: _base, destinations } => {
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
        }
    }
}

impl <'a, 'b> TaskSpec {
    fn add_to<F: Sized + Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError> + 'a>
                (&'a self, graph: &mut Graph,
                 existing_nodes: &'b mut HashSet<&'a TaskSpec>,
                 tile_width: u32) -> () {
        if existing_nodes.contains(self) {
            return;
        }
        let name = self.to_string();
        match self {
            TaskSpec::Animate { background, frames } => {
                let frame_count = frames.len();
                background.add_to::<F>(graph, existing_nodes, tile_width);
                for frame in frames {
                    frame.add_to::<F>(graph, existing_nodes, tile_width);
                }
                let frame_asset_strings: Vec<String> = (0..frame_count).map(|index| format!("{name}::frame{index}")).collect();
                let background_asset_string = format!("{}::background", name);
                let mut input_asset_strings = vec![background_asset_string.clone()];
                input_asset_strings.extend_from_slice(&frame_asset_strings[..]);
                let input_asset_s = input_asset_strings.clone();
                let output_asset_string = format!("{}::output", name);
                let output_asset_s = output_asset_string.clone();
                let frame_count_ = frame_count;
                let node = Node::new(name, move |solver: &mut GraphSolver| {
                    let (background_asset_string, frame_asset_strings) =
                        input_asset_s.split_first().unwrap();
                    let background_asset_string = background_asset_string.clone();
                    let background_pixmap: Pixmap =
                        solver.get_value::<Pixmap>(&background_asset_string)?;
                    let frame_pixmaps: Vec<Pixmap> = frame_asset_strings.iter()
                        .map(|asset_string| (Self::get_pixmap(solver, &asset_string))).collect();
                    if !solver.input_is_new(&background_pixmap, &background_asset_string)
                        && (0..frame_count_).any(|index| {
                            solver.input_is_new(&frame_pixmaps[index], &frame_asset_strings[index])
                    }) {
                        let outs = vec!(output_asset_s.clone());
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
            TaskSpec::FromSvg { source } => {
                let tile_width_ = tile_width;
                let node = create_node!(name: name, (source: PathBuf) -> (output: Pixmap)
                    output = from_svg(source, tile_width_).unwrap()
                );
                graph.add_node(node).unwrap();
                graph.define_freestanding_asset(&*format!("{}::source", self),
                                                source.clone()).unwrap();
            },
            TaskSpec::MakeSemitransparent { base, alpha } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let alpha_ = alpha.into_inner();
                let node = create_node!(name: name, (base: Pixmap) -> (output: Pixmap)
                    output = make_semitransparent(base, alpha_).unwrap()
                );
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::PngOutput { base, destinations } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let node = create_node!(name: name,
                    (destinations: Vec<PathBuf>, base: Pixmap) -> (output: ()) {
                        output = png_output(base, destinations).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
                graph.define_freestanding_asset(&*format!("{}::destinations", self),
                        destinations.clone()).unwrap();
            },
            TaskSpec::Repaint { base, color } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let color_ = color.clone();
                let node = create_node!(name: name, (base: AlphaChannel) -> (output: Pixmap) {
                    output = paint(base, color_).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::Stack { background, layers } => {
                let layer_count = layers.len();
                for layer in layers {
                    layer.add_to::<F>(graph, existing_nodes, tile_width);
                }
                let layer_asset_strings: Vec<String> = (0..layer_count).map(|index| format!("{name}::frame{index}")).collect();
                let layer_asset_s = layer_asset_strings.clone();
                let output_asset_string = format!("{}::output", name);
                let output_asset_s = output_asset_string.clone();
                let layer_count_ = layer_count;
                let background_ = background.clone();
                let node = Node::new(name, move |solver: &mut GraphSolver| {
                    let layer_pixmaps: Vec<Pixmap> = layer_asset_s.iter().map(|asset_string| {
                        solver.get_value::<Pixmap>(asset_string).unwrap()
                    }).collect();
                    if !(0..layer_count_).any(|index| {
                        solver.input_is_new(&layer_pixmaps[index], &layer_asset_s[index])
                    }) {
                        let outs = vec!(output_asset_s.to_owned());
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let output = stack(background_, Box::new(layer_pixmaps.into_iter())).unwrap();
                    solver.save_value(&output_asset_s, output);
                    return Ok(SolverStatus::Executed);
                }, layer_asset_strings.to_owned(), vec!(output_asset_string.to_owned()));
                graph.add_node(node).unwrap();
                for index in 0..layer_count {
                    let source = &format!("{}::output", layers[index]);
                    let sink = &layer_asset_strings[index];
                    graph.bind_asset(source, sink).unwrap();
                };
            },
            TaskSpec::ToAlphaChannel { base } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let node =
                    create_node!(name: name, (base: Pixmap) -> (output: AlphaChannel) {
                        output = AlphaChannel::from(&base)
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            }
        }
        existing_nodes.insert(self);
    }

    fn get_pixmap(solver: &mut GraphSolver, asset_string: &&String) -> Pixmap {
        solver.get_value::<Pixmap>(&asset_string).unwrap()
    }
}

lazy_static! {
    static ref OUT_DIR: &'static Path = Path::new("./out/");
    static ref SVG_DIR: &'static Path = Path::new("./svg/");
}

fn name_to_out_path(name: String) -> Box<Path> {
    return OUT_DIR.with_file_name(format!("{}.png", name)).as_path().into();
}

fn name_to_svg_path(name: String) -> PathBuf {
    return SVG_DIR.with_file_name(format!("{}.svg", name)).as_path().into();
}

impl FromStr for TaskSpec {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TaskSpec::FromSvg {
            source: name_to_svg_path(s.to_string())
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
        let clone = self.clone();
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