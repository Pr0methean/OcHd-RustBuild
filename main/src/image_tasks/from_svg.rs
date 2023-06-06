use std::path::PathBuf;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree, TreeParsing};

use crate::anyhoo;
use crate::image_tasks::{allocate_pixmap_empty, MaybeFromPool};
use crate::image_tasks::task_spec::{CloneableError, SVG_DIR};

pub const COLOR_SVGS: &[&str] = &[
    "barrelSlats",
    "bed",
    "blastFurnaceHolesLit",
    "blastFurnaceHolesLit1",
    "bonemeal",
    "bonemealSmall",
    "bonemealSmallNoBorder",
    "bookShelves",
    "bookShelvesChiseled",
    "bookShelvesChiseledEmpty",
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
    "torchRedstoneHead",
    "vineBerries",
];

pub const SEMITRANSPARENCY_FREE_SVGS: &[&str] = &[
    "barrelSlats",
    "blastFurnaceHolesLit",
    "blastFurnaceHolesLit1",
    "bookShelves",
    "borderDotted",
    "borderDottedBottomRight",
    "borderLongDashes",
    "borderShortDashes",
    "borderSolid",
    "borderSolidExtraThick",
    "borderSolidThick",
    "borderSolidTopLeftBottomRight",
    "bottomHalf",
    "bricks",
    "bricksSmall",
    "checksLarge",
    "checksLargeOutline",
    "checksLargeTop",
    "checksSmall",
    "checksSmallOutline",
    "checksSmallTop",
    "checksTiny",
    "commandBlockGrid",
    "commandBlockGridFront",
    "commandBlockSquare4x",
    "craftingGridSpaces",
    "craftingGridSpacesCross",
    "craftingGridSquare",
    "cross",
    "crossDotted",
    "cutInQuarters1",
    "cutInQuarters2",
    "diagonalChecksFillerSquaresBottomLeftTopRight",
    "diagonalChecksFillerSquaresTopLeftBottomRight",
    "flowerStemBottomBorder",
    "flowerStemShortBorder",
    "flowerStemTallBorder",
    "grassShort",
    "grassTall",
    "grassVeryShort",
    "gridSpaces4x",
    "gridSpacesCross4x",
    "ingotBorder",
    "ingotMask",
    "jigsawLock",
    "leaves1",
    "leaves1a",
    "leaves2",
    "leaves2a",
    "leaves3",
    "leaves3a",
    "leaves3b",
    "mushroomStem",
    "paneTop",
    "planksTopBorderVertical",
    "planksTopVertical",
    "rail",
    "railTies",
    "repeaterSideInputs",
    "saplingStem",
    "stripesVerticalThick",
    "thirdRail",
    "tntSticksEnd",
    "tntSticksSide",
    "tntStripe",
    "topPart",
    "topStripeThick",
    "torchBase",
    "torchShadow",
    "trapdoor1"
];

pub fn from_svg(mut path: String, width: u32) -> Result<MaybeFromPool<Pixmap>,CloneableError> {
    path.push_str(".svg");
    let svg = SVG_DIR.get_file(PathBuf::from(&path)).ok_or(
        anyhoo!(format!("File not found: {}", path)))?;
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
