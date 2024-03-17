use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::axe::giant_mushroom::{
    BROWN_MUSHROOM_BACKGROUND, MUSHROOM_STEM_MAIN_COLOR, RED_MUSHROOM_BACKGROUND,
};
use crate::{block_with_colors, copy_block, group, single_layer_block, single_texture_block};
block_with_colors!(
    SUGARCANE = c(0xaadb74),
    c(0x82a859),
    c(0x91ff32),
    ComparableColor::TRANSPARENT,
    paint_svg_task("bambooThick", shadow!()),
    paint_svg_task("bambooThin", highlight!()),
    paint_svg_task("bambooThinMinusBorder", color!())
);

single_texture_block!(
    BROWN_MUSHROOM = ComparableColor::TRANSPARENT,
    paint_svg_task("mushroomStem", MUSHROOM_STEM_MAIN_COLOR),
    paint_svg_task("mushroomCapBrown", BROWN_MUSHROOM_BACKGROUND)
);

single_texture_block!(
    RED_MUSHROOM = ComparableColor::TRANSPARENT,
    paint_svg_task("mushroomStem", MUSHROOM_STEM_MAIN_COLOR),
    paint_svg_task("mushroomCapRed", RED_MUSHROOM_BACKGROUND)
);

single_layer_block!(REDSTONE_DUST_DOT = "redstone", ComparableColor::WHITE);
single_layer_block!(REDSTONE_DUST_LINE0 = "redstoneLine", ComparableColor::WHITE);
copy_block!(REDSTONE_DUST_LINE1 = REDSTONE_DUST_LINE0, "");

const TWISTING_VINE_COLOR: ComparableColor = c(0x008383);
single_layer_block!(TWISTING_VINES_PLANT = "zigzagSolid", TWISTING_VINE_COLOR);
single_layer_block!(
    TWISTING_VINES = "zigzagSolidBottomPart",
    TWISTING_VINE_COLOR
);

const WEEPING_VINE_COLOR: ComparableColor = c(0x7b0000);
single_layer_block!(WEEPING_VINES_PLANT = "zigzagSolid", WEEPING_VINE_COLOR);
single_layer_block!(WEEPING_VINES = "zigzagSolidTopPart", WEEPING_VINE_COLOR);

pub const HONEYCOMB_BORDER: ComparableColor = c(0xffce5d);
pub const HONEYCOMB_CELLS: ComparableColor = c(0xd87800);

single_texture_block!(
    HONEYCOMB_BLOCK = HONEYCOMB_CELLS,
    paint_svg_task("honeycomb", HONEYCOMB_BORDER)
);

group!(
    SIMPLE_BARE_HAND_BLOCKS = SUGARCANE,
    BROWN_MUSHROOM,
    RED_MUSHROOM,
    REDSTONE_DUST_DOT,
    REDSTONE_DUST_LINE0,
    REDSTONE_DUST_LINE1,
    TWISTING_VINES_PLANT,
    TWISTING_VINES,
    WEEPING_VINES_PLANT,
    WEEPING_VINES,
    HONEYCOMB_BLOCK
);
