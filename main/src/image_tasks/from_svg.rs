use std::fs;
use std::path::PathBuf;

use resvg::{FitTo, render};
use tiny_skia::Pixmap;
use tiny_skia_path::Transform;
use usvg::{Options, Tree, TreeParsing};

use crate::anyhoo;
use crate::image_tasks::task_spec::TaskResult;

const COLOR_SVGS: &'static [&str] = &[
    "bed",
    "blastFurnaceHoles",
    "blastFurnaceHoles1",
    "bonemeal",
    "bonemealSmall",
    "bonemealSmallNoBorder",
    "bookShelves",
    "chain",
    "commandBlockChains",
    "commandBlockChains4x",
    "commandBlockGrid",
    "commandBlockGridFront",
    "doorKnob",
    "furnaceFrontLit",
    "loopArrow4x",
    "soulFlameTorch",
    "soulFlameTorchSmall",
    "torchFlame",
    "torchFlameSmall",
];

pub fn from_svg(path: PathBuf, width: u32) -> TaskResult {
    let svg_data = fs::read(path).map_err(|error| anyhoo!(error))?;
    let svg_tree = Tree::from_data(&svg_data, &Options::default()).map_err(|error| anyhoo!(error))?;
    let view_box = svg_tree.view_box;
    let height = f64::from(width) * view_box.rect.height() / view_box.rect.width();
    let mut out = Pixmap::new(width.to_owned(), height as u32)
        .ok_or(anyhoo!("Failed to create output Pixmap"))?;
    render(
        &svg_tree,
        FitTo::Width(width.to_owned()),
        Transform::default(),
        out.as_mut())
        .ok_or(anyhoo!("Failed to render output Pixmap"))?;
    return TaskResult::Pixmap {value: out};
}
