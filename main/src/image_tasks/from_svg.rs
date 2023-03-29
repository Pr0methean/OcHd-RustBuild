use std::fs;
use std::fs::DirEntry;
use std::path::Path;
use anyhow::anyhow;
use resvg::{FitTo, render};
use tiny_skia::Pixmap;
use tiny_skia_path::Transform;
use usvg::{Options, Tree, TreeParsing};

const COLOR_SVGS: Vec<&'static str> = vec![
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

pub fn from_svg(path: Box<Path>, width: u32) -> Result<Pixmap, anyhow::Error> {
    let svg_data = fs::read(path)?;
    let svg_tree = Tree::from_data(&svg_data, &Options::default())?;
    let view_box = svg_tree.view_box;
    let height = f64::from(width) * view_box.rect.height() / view_box.rect.width();
    let mut out = Pixmap::new(width, height as u32)
        .ok_or(anyhow!("Failed to create output Pixmap"))?;
    render(
        &svg_tree,
        FitTo::Width(width),
        Transform::default(),
        out.as_mut())
        .ok_or(anyhow!("Failed to render output Pixmap"))?;
    return Ok(out);
}
