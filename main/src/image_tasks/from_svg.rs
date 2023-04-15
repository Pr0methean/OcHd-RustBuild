use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use log::info;

use resvg::{FitTo, render};
use tiny_skia::Pixmap;
use tiny_skia_path::Transform;
use tracing::instrument;
use usvg::{Options, Tree, TreeParsing};

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError};

pub const COLOR_SVGS: &[&str] = &[
    "./svg/bed.svg",
    "./svg/blastFurnaceHoles.svg",
    "./svg/blastFurnaceHoles1.svg",
    "./svg/bonemeal.svg",
    "./svg/bonemealSmall.svg",
    "./svg/bonemealSmallNoBorder.svg",
    "./svg/bookShelves.svg",
    "./svg/chain.svg",
    "./svg/commandBlockChains.svg",
    "./svg/commandBlockChains4x.svg",
    "./svg/commandBlockGrid.svg",
    "./svg/commandBlockGridFront.svg",
    "./svg/doorKnob.svg",
    "./svg/furnaceFrontLit.svg",
    "./svg/loopArrow4x.svg",
    "./svg/soulFlameTorch.svg",
    "./svg/soulFlameTorchSmall.svg",
    "./svg/torchFlame.svg",
    "./svg/torchFlameSmall.svg",
];

#[instrument]
pub fn from_svg(path: &PathBuf, width: u32) -> Result<Pixmap,CloneableError> {
    info!("Starting task: Import svg from {}", path.to_string_lossy());
    let svg_data = fs::read(path).map_err(|error| anyhoo!(error))?;
    let svg_tree = Tree::from_data(&svg_data, &Options::default()).map_err(|error| anyhoo!(error))?;
    let view_box = svg_tree.view_box;
    let height = f64::from(width) * view_box.rect.height() / view_box.rect.width();
    let mut out = Pixmap::new(width, height as u32)
        .ok_or(anyhoo!("Failed to create output Pixmap"))?;
    render(
        &svg_tree,
        FitTo::Width(width),
        Transform::default(),
        out.as_mut())
        .ok_or(anyhoo!("Failed to render output Pixmap"))?;
    info!("Finishing task: Import svg from {}", path.to_string_lossy());
    Ok(out)
}
