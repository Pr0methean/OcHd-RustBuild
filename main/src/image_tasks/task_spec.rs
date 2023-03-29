use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;
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
    FromSvg {source: Box<Path>},
    MakeSemitransparent {base: Arc<TaskSpec>, alpha: OrderedFloat<f32>},
    PngOutput {base: Arc<TaskSpec>, destinations: Vec<Box<Path>>},
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
                        None => {"Unknown PNG file"}
                        Some(name) => {&(name.to_os_string().into_string().unwrap())}
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

impl TaskSpec {
    fn add_to<F: Sized + Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>>
                (&self, graph: &mut Graph,
                 node_map: &mut HashMap<&TaskSpec, &dyn NodeRunner>,
                 tile_width: u32) -> () {
        if node_map.contains_key(self) {
            return;
        }
        let name = self.to_string();
        let node_ref: &dyn NodeRunner;
        match self {
            TaskSpec::Animate { background, frames } => {
                background.add_to::<F>(graph, node_map, tile_width);
                frames.iter().for_each(|layer| layer.add_to::<F>(graph, node_map, tile_width));
                let frame_count = frames.len();
                let frame_asset_strings: Vec<String> = (0..frame_count).map(|index| {
                    format!("{}::frame{}", name, index)
                }).collect();
                let background_asset_string = format!("{}::background", name);
                let mut input_asset_strings = vec!(background_asset_string);
                input_asset_strings.extend_from_slice(frame_asset_strings.as_slice());
                let output_asset_string = format!("{}::output", name);
                let node = Node::new(name, |solver: &mut GraphSolver| {
                    let background_pixmap: Pixmap =
                        solver.get_value::<Pixmap>(&background_asset_string)?;
                    let frame_pixmaps_by_name: Vec<(&String, Pixmap)> = frame_asset_strings.iter().map(|asset_string| {
                        (asset_string, solver.get_value::<Pixmap>(asset_string).unwrap())
                    }).collect();
                    if !solver.input_is_new(&background_pixmap, &background_asset_string)
                        && !frame_pixmaps_by_name.iter().any(|(asset_string, input_asset)| {
                        solver.input_is_new(input_asset, asset_string)
                    }) {
                        let outs = vec!(output_asset_string);
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let frame_pixmaps =
                        (0..frame_count).map(|index| frame_pixmaps_by_name[index].1).take(frame_count);
                    let output = animate(background_pixmap, Box::new(frame_pixmaps)).unwrap();
                    solver.save_value(&output_asset_string, output);
                    return Ok(SolverStatus::Executed);
                }, input_asset_strings, vec!(output_asset_string));
                graph.bind_asset(&*format!("{}::output", background), &*background_asset_string).unwrap();
                (0..frame_count).for_each(|index| {
                    let source = &format!("{}::output", frames[index]);
                    let sink = &frame_asset_strings[index];
                    graph.bind_asset(source, sink).unwrap();
                });
                node_ref = &node;
            }
            TaskSpec::FromSvg { source } => {
                let node = create_node!(name: name, () -> (output: Pixmap)
                    output = from_svg(*source, tile_width).unwrap()
                );
                graph.add_node(node).unwrap();
                node_ref = &node;
            }
            TaskSpec::MakeSemitransparent { base, alpha } => {
                base.add_to::<F>(graph, node_map, tile_width);
                let node = create_node!(name: name, (base: Pixmap) -> (output: Pixmap)
                    output = make_semitransparent(base, alpha.into_inner()).unwrap()
                );
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
                node_ref = &node;
            }
            TaskSpec::PngOutput { base, destinations } => {
                base.add_to::<F>(graph, node_map, tile_width);
                let node = create_node!(name: name, (base: Pixmap) -> (output: ()) {
                    output = png_output(base, *destinations).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
                node_ref = &node;
            }
            TaskSpec::Repaint { base, color } => {
                base.add_to::<F>(graph, node_map, tile_width);
                let node = create_node!(name: name, (base: AlphaChannel) -> (output: Pixmap) {
                    output = paint(base, *color).unwrap()
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
                node_ref = &node;
            }
            TaskSpec::Stack { background, layers } => {
                layers.iter().for_each(|layer| layer.add_to::<F>(graph, node_map, tile_width));
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
                        let outs = vec!(output_asset_string);
                        if solver.use_old_ouput(&outs) {
                            return Ok(SolverStatus::Cached);
                        }
                    }
                    let layer_pixmaps =
                        (0..layer_count).map(|index| layer_pixmaps_by_name[index].1).take(layer_count);
                    let output = stack(*background, Box::new(layer_pixmaps)).unwrap();
                    solver.save_value(&output_asset_string, output);
                    return Ok(SolverStatus::Executed);
                }, layer_asset_strings, vec!(output_asset_string));
                (0..layer_count).for_each(|index| {
                    let source = &format!("{}::output", layers[index]);
                    let sink = &layer_asset_strings[index];
                    graph.bind_asset(source, sink).unwrap();
                });
                node_ref = &node;
            }
            TaskSpec::ToAlphaChannel { base } => {
                base.add_to::<F>(graph, node_map, tile_width);
                let node =
                    create_node!(name: name, (base: Pixmap) -> (output: AlphaChannel) {
                        output = AlphaChannel::from(base)
                });
                graph.add_node(node).unwrap();
                graph.bind_asset(&*format!("{}::output", base),
                                 &*format!("{}::base", self)).unwrap();
                node_ref = &node;
            }
        }
        node_map.insert(self, node_ref);
    }
}