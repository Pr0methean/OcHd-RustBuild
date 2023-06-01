use std::path::PathBuf;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree, TreeParsing};

use crate::anyhoo;
use crate::image_tasks::{allocate_pixmap_empty, MaybeFromPool};
use crate::image_tasks::task_spec::{CloneableError, SVG_DIR};

pub const COLOR_SVGS: &[&str] = &[
    "barrelSlats.svg",
    "bed.svg",
    "blastFurnaceHolesLit.svg",
    "blastFurnaceHolesLit1.svg",
    "bonemeal.svg",
    "bonemealSmall.svg",
    "bonemealSmallNoBorder.svg",
    "bookShelves.svg",
    "bookShelvesChiseled.svg",
    "bookShelvesChiseledEmpty.svg",
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

pub const SEMITRANSPARENCY_FREE_SVGS: &[&str] = &[
    "barrelSlats.svg",
    "blastFurnaceHolesLit.svg",
    "blastFurnaceHolesLit1.svg",
    "bookShelves.svg",
    "borderDotted.svg",
    "borderDottedBottomRight.svg",
    "borderLongDashes.svg",
    "borderShortDashes.svg",
    "borderSolid.svg",
    "borderSolidExtraThick.svg",
    "borderSolidThick.svg",
    "borderSolidTopLeftBottomRight.svg",
    "bottomHalf.svg",
    "bricks.svg",
    "bricksSmall.svg",
    "checksLarge.svg",
    "checksLargeOutline.svg",
    "checksLargeTop.svg",
    "checksSmall.svg",
    "checksSmallOutline.svg",
    "checksSmallTop.svg",
    "checksTiny.svg",
    "commandBlockGrid.svg",
    "commandBlockGridFront.svg",
    "commandBlockSquare4x.svg",
    "craftingGridSpaces.svg",
    "craftingGridSpacesCross.svg",
    "craftingGridSquare.svg",
    "cross.svg",
    "crossDotted.svg",
    "cutInQuarters1.svg",
    "cutInQuarters2.svg",
    "diagonalChecksFillerSquaresBottomLeftTopRight.svg",
    "diagonalChecksFillerSquaresTopLeftBottomRight.svg",
    "flowerStemBottomBorder.svg",
    "flowerStemShortBorder.svg",
    "flowerStemTallBorder.svg",
    "grassShort.svg",
    "grassTall.svg",
    "grassVeryShort.svg",
    "gridSpaces4x.svg",
    "gridSpacesCross4x.svg",
    "ingotBorder.svg",
    "ingotMask.svg",
    "jigsawLock.svg",
    "leaves1.svg",
    "leaves1a.svg",
    "leaves2.svg",
    "leaves2a.svg",
    "leaves3.svg",
    "leaves3a.svg",
    "leaves3b.svg",
    "mushroomStem.svg",
    "paneTop.svg",
    "planksTopBorderVertical.svg",
    "planksTopVertical.svg",
    "rail.svg",
    "railTies.svg",
    "repeaterSideInputs.svg",
    "saplingStem.svg",
    "stripesVerticalThick.svg",
    "thirdRail.svg",
    "tntSticksEnd.svg",
    "tntSticksSide.svg",
    "tntStripe.svg",
    "topPart.svg",
    "topStripeThick.svg",
    "torchBase.svg",
    "torchShadow.svg",
    "trapdoor1.svg"
];

pub fn from_svg(path: &PathBuf, width: u32) -> Result<MaybeFromPool<Pixmap>,CloneableError> {
    let path_str = path.to_string_lossy();
    let svg = SVG_DIR.get_file(path).ok_or(anyhoo!(format!("File not found: {}", path_str)))?;
    let svg_tree = resvg::Tree::from_usvg(
        &Tree::from_data(svg.contents(), &Options::default())?);
    let view_box = svg_tree.view_box;
    let height = f64::from(width) * view_box.rect.height() as f64 / view_box.rect.width() as f64;
    let scale = (width as f64 / svg_tree.size.width() as f64) as f32;
    let mut out = allocate_pixmap_empty(width, height as u32);
    svg_tree.render(
        Transform::from_scale(scale, scale),
        &mut out.as_mut());
    Ok(out)
}
