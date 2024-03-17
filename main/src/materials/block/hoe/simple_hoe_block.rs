use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::pickaxe::ore::REDSTONE;
use crate::{block_with_colors, group};

block_with_colors!(
    SHROOMLIGHT = c(0xffac6d),
    c(0xd75100),
    c(0xffffb4),
    color!(),
    paint_svg_task("borderSolid", highlight!()),
    paint_svg_task("checksSmall", shadow!()),
    paint_svg_task("shroomlightOn", highlight!())
);

block_with_colors!(
    TARGET_SIDE = c(0xffd7ba),
    REDSTONE.colors.shadow,
    ComparableColor::WHITE,
    color!(),
    paint_svg_task("grassTall", highlight!()),
    paint_svg_task("ringsCentralBullseye", shadow!())
);

block_with_colors!(
    TARGET_TOP = c(0xffd7ba),
    REDSTONE.colors.shadow,
    ComparableColor::WHITE,
    color!(),
    paint_svg_task("checksSmall", highlight!()),
    paint_svg_task("ringsCentralBullseye", shadow!())
);

group!(SIMPLE_HOE_BLOCKS = SHROOMLIGHT, TARGET_SIDE, TARGET_TOP);
