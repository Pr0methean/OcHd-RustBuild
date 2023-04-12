use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use resvg::{FitTo, render};

use tiny_skia_path::Transform;
use tracing::instrument;
use usvg::{Options, Tree, TreeParsing};

use crate::anyhoo;
use crate::image_tasks::allocate_pixmap;
use crate::image_tasks::task_spec::TaskResult;

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
pub fn from_svg<'a>(path: PathBuf, width: u32) -> TaskResult<'a> {
    let svg_data = fs::read(path).map_err(|error| anyhoo!(error))?;
    let svg_tree = Tree::from_data(&svg_data, &Options::default()).map_err(|error| anyhoo!(error))?;
    let view_box = svg_tree.view_box;
    let height = f64::from(width) * view_box.rect.height() / view_box.rect.width();
    let mut out = allocate_pixmap(width, height as u32);
    render(
        &svg_tree,
        FitTo::Width(width),
        Transform::default(),
        out.as_mut())
        .ok_or(anyhoo!("Failed to render output Pixmap"))?;
    TaskResult::Pixmap {value: Arc::new(out)}
}
