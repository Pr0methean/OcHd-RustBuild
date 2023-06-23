use once_cell::sync::Lazy;
use crate::{stack, paint_stack, stack_on, group};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::pickaxe::ore_base::NETHERRACK;
use crate::texture_base::material::{ground_cover_block, GroundCoverBlock};

const CRIMSON_NYLIUM_COLOR: ComparableColor = c(0x854242);
const CRIMSON_NYLIUM_SHADOW: ComparableColor = c(0x7b0000);
const CRIMSON_NYLIUM_HIGHLIGHT: ComparableColor = c(0xbd3030);

pub static CRIMSON_NYLIUM: Lazy<GroundCoverBlock> = Lazy::new(|| ground_cover_block(
    "crimson_nylium", "", &NETHERRACK.material,
    CRIMSON_NYLIUM_COLOR,
    CRIMSON_NYLIUM_SHADOW,
    CRIMSON_NYLIUM_HIGHLIGHT,
    stack!(
        paint_stack!(CRIMSON_NYLIUM_COLOR, "topPart", "bigDotsTop"),
        paint_svg_task("mushroomTopLeft", CRIMSON_NYLIUM_HIGHLIGHT),
        paint_svg_task("mushroomTopRight", CRIMSON_NYLIUM_SHADOW)
    ),
    stack_on!(
        CRIMSON_NYLIUM_COLOR,
        paint_svg_task("mushroomsTopLeftBottomRight", CRIMSON_NYLIUM_SHADOW),
        paint_stack!(CRIMSON_NYLIUM_HIGHLIGHT, "mushroomsBottomLeftTopRight", "borderRoundDots")
    )
));

const WARPED_NYLIUM_COLOR: ComparableColor = c(0x568353);
const WARPED_NYLIUM_SHADOW: ComparableColor = c(0x456b52);
const WARPED_NYLIUM_HIGHLIGHT: ComparableColor = c(0xac2020);

pub static WARPED_NYLIUM: Lazy<GroundCoverBlock> = Lazy::new(|| ground_cover_block(
    "warped_nylium", "", &NETHERRACK.material,
    WARPED_NYLIUM_COLOR,
    WARPED_NYLIUM_SHADOW,
    WARPED_NYLIUM_HIGHLIGHT,
    stack!(
        paint_stack!(WARPED_NYLIUM_HIGHLIGHT, "topPart", "bigDotsTop"),
        paint_svg_task("mushroomTopLeft", WARPED_NYLIUM_COLOR),
        paint_svg_task("mushroomTopRight", WARPED_NYLIUM_SHADOW)
    ),
    stack_on!(
        WARPED_NYLIUM_HIGHLIGHT,
        paint_svg_task("mushroomsTopLeftBottomRight", WARPED_NYLIUM_COLOR),
        paint_stack!(WARPED_NYLIUM_SHADOW, "mushroomsBottomLeftTopRight",
            "borderRoundDotsVaryingSize")
    )
));

group!(NYLIUM = CRIMSON_NYLIUM, WARPED_NYLIUM);