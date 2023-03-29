use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::ops::Mul;
use std::path::Path;
use std::rc::Rc;
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
enum TaskSpec<'a> {
    Animate {background: Arc<TaskSpec<'a>>, frames: Vec<Arc<TaskSpec<'a>>>},
    FromSvg {source: &'a Path},
    MakeSemitransparent {base: Arc<TaskSpec<'a>>, alpha: OrderedFloat<f32>},
    PngOutput {base: Arc<TaskSpec<'a>>, destinations: Vec<&'a Path>},
    Repaint {base: Arc<TaskSpec<'a>>, color: ComparableColor},
    Stack {background: ComparableColor, layers: Vec<Arc<TaskSpec<'a>>>},
    ToAlphaChannel {base: Arc<TaskSpec<'a>>}
}

impl Display for TaskSpec<'_> {
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

impl <'a> TaskSpec<'a> {
    fn add_to<F: Sized + Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError> + 'a>
                (&'a self, graph: &'a mut Graph,
                 existing_nodes: &'a mut HashSet<&TaskSpec>,
                 tile_width: u32) -> () {
        if existing_nodes.contains(self) {
            return;
        }
        let name = self.to_string();
        match self {
            TaskSpec::Animate { background, frames } => {
                background.add_to::<F>(graph, existing_nodes, tile_width);
                frames.iter().for_each(|layer| layer.add_to::<F>(graph, existing_nodes, tile_width));
                let frame_count = frames.len();
                let frame_asset_strings: Vec<String> = (0..frame_count).map(|index| {
                    format!("{}::frame{}", name, index)
                }).collect();
                let background_asset_string = format!("{}::background", name);
                let mut input_asset_strings = vec!(background_asset_string);
                input_asset_strings.extend_from_slice(&*frame_asset_strings.into_boxed_slice());
                let output_asset_string = format!("{}::output", name);
                let node = Node::new(name, |solver: &mut GraphSolver| {
                    let background_pixmap: Pixmap =
                        solver.get_value::<Pixmap>(&background_asset_string)?;
                    let frame_pixmaps_by_name: Vec<(Pixmap, &String)> = frame_asset_strings.iter()
                        .map(|asset_string| {
                            (solver.get_value::<Pixmap>(&asset_string).unwrap(),
                             asset_string)
                        }).collect();
                    if !solver.input_is_new(&background_pixmap, &background_asset_string)
                        && !frame_pixmaps_by_name.iter().any(|(input_asset, asset_string)| {
                        solver.input_is_new(&input_asset, &asset_string)
                    }) {
                        let outs = vec!(output_asset_string.to_owned());
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let frame_pixmaps =
                        (0..frame_count).map(|index| frame_pixmaps_by_name[index].0).take(frame_count);
                    let output = animate(background_pixmap, Box::new(frame_pixmaps)).unwrap();
                    solver.save_value(&output_asset_string, output);
                    return Ok(SolverStatus::Executed);
                }, input_asset_strings, vec!(output_asset_string.to_owned()));
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", background), &*background_asset_string).unwrap();
                (0..frame_count).for_each(|index| {
                    let source = &format!("{}::output", frames[index]);
                    let sink = &frame_asset_strings[index];
                    graph.bind_asset(source, sink).unwrap();
                });
            },
            TaskSpec::FromSvg { source } => {
                let node = create_node!(name: name, () -> (output: Pixmap)
                    output = from_svg(source, tile_width).unwrap()
                );
                graph.add_node(node).unwrap();
            },
            TaskSpec::MakeSemitransparent { base, alpha } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let node = create_node!(name: name, (base: Pixmap) -> (output: Pixmap)
                    output = make_semitransparent(base, alpha.into_inner()).unwrap()
                );
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::PngOutput { base, destinations } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let node = create_node!(name: name, (base: Pixmap) -> (output: ()) {
                    output = png_output(base, *destinations).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::Repaint { base, color } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let node = create_node!(name: name, (base: AlphaChannel) -> (output: Pixmap) {
                    output = paint(base, *color).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            },
            TaskSpec::Stack { background, layers } => {
                layers.iter().for_each(|layer| layer.add_to::<F>(graph, existing_nodes, tile_width));
                let layer_count = layers.len();
                let layer_asset_strings: Vec<String> = (0..layer_count).map(|index| {
                    format!("{}::frame{}", name, index)
                }).collect();
                let output_asset_string = format!("{}::output", name);
                let node = Node::new(name, |solver: &mut GraphSolver| {
                    let layer_pixmaps_by_name: Vec<(&String, Pixmap)> = layer_asset_strings.iter().map(|asset_string| {
                        (asset_string, solver.get_value::<Pixmap>(asset_string).unwrap())
                    }).collect();
                    if !layer_pixmaps_by_name.iter().any(|(asset_string, input_asset)| {
                        solver.input_is_new(input_asset, asset_string)
                    }) {
                        let outs = vec!(output_asset_string.to_owned());
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let layer_pixmaps =
                        (0..layer_count).map(|index| layer_pixmaps_by_name[index].1).take(layer_count);
                    let output = stack(*background, Box::new(layer_pixmaps)).unwrap();
                    solver.save_value(&output_asset_string, output);
                    return Ok(SolverStatus::Executed);
                }, layer_asset_strings.to_owned(), vec!(output_asset_string.to_owned()));
                graph.add_node(node).unwrap();
                (0..layer_count).for_each(|index| {
                    let source = &format!("{}::output", layers[index]);
                    let sink = &layer_asset_strings[index];
                    graph.bind_asset(source, sink).unwrap();
                });
            },
            TaskSpec::ToAlphaChannel { base } => {
                base.add_to::<F>(graph, existing_nodes, tile_width);
                let node =
                    create_node!(name: name, (base: Pixmap) -> (output: AlphaChannel) {
                        output = AlphaChannel::from(base)
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
            }
        }
        existing_nodes.insert(self);
    }
}

lazy_static! {
    static ref OUT_DIR: &'static Path = Path::new("./out/");
    static ref SVG_DIR: &'static Path = Path::new("./svg/");
}

fn name_to_out_path(name: String) -> Box<Path> {
    return OUT_DIR.with_file_name(format!("{}.png", name)).as_path().into();
}

fn name_to_svg_path(name: String) -> Box<Path> {
    return SVG_DIR.with_file_name(format!("{}.svg", name)).as_path().into();
}

impl FromStr for TaskSpec<'_> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TaskSpec::FromSvg {
            source: &*name_to_svg_path(s.to_string()).to_owned()
        })
    }
}

impl<'a> Mul<f32> for TaskSpec<'a> {
    type Output = TaskSpec<'a>;

    fn mul(self, rhs: f32) -> Self::Output {
        TaskSpec::MakeSemitransparent {
            base: Arc::from(self),
            alpha: OrderedFloat::from(rhs)
        }
    }
}

impl<'a> Mul<ComparableColor> for TaskSpec<'a> {
    type Output = TaskSpec<'a>;

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