use std::path::PathBuf;

use log::info;

use resvg::{FitTo, render};
use tiny_skia::{Pixmap};
use tiny_skia_path::Transform;
use usvg::{Options, Tree, TreeParsing};

use crate::anyhoo;
use crate::image_tasks::{allocate_pixmap_empty, MaybeFromPool};
use crate::image_tasks::task_spec::{CloneableError, SVG_DIR};

pub const COLOR_SVGS: &[&str] = &[
    "bed.svg",
    "blastFurnaceHolesLit.svg",
    "blastFurnaceHolesLit1.svg",
    "bonemeal.svg",
    "bonemealSmall.svg",
    "bonemealSmallNoBorder.svg",
    "bookShelves.svg",
    "chain.svg",
    "commandBlockChains.svg",
    "commandBlockChains4x.svg",
    "commandBlockGrid.svg",
    "commandBlockGridFront.svg",
    "doorKnob.svg",
    "furnaceFrontLit.svg",
    "loopArrow4x.svg",
    "soulFlameTorch.svg",
    "soulFlameTorchSmall.svg",
    "torchFlame.svg",
    "torchFlameSmall.svg",
    "torchRedstoneHead.svg",
    "vineBerries.svg",
];

pub fn from_svg(path: &PathBuf, width: u32) -> Result<MaybeFromPool<Pixmap>,CloneableError> {
    let path_str = path.to_string_lossy();
    info!("Starting task: Import svg from {}", path_str);
    let svg = SVG_DIR.get_file(path).ok_or(anyhoo!(format!("File not found: {}", path_str)))?;
    let svg_tree = Tree::from_data(svg.contents(), &Options::default()).map_err(|error| anyhoo!(error))?;
    let view_box = svg_tree.view_box;
    let height = f64::from(width) * view_box.rect.height() / view_box.rect.width();
    let mut out = allocate_pixmap_empty(width, height as u32);
    render(
        &svg_tree,
        FitTo::Width(width),
        Transform::default(),
        out.as_mut())
        .ok_or(anyhoo!("Failed to render output Pixmap"))?;
    info!("Finishing task: Import svg from {}", path.to_string_lossy());
    Ok(out)
}
