use crate::{block_with_colors, group, paint_stack};
use crate::image_tasks::color::{c};
use crate::image_tasks::task_spec::paint_svg_task;

block_with_colors!(SAND = c(0xdfd5aa), c(0xd1ba8a), c(0xEaEaD0),
    color!(),
    paint_stack!(shadow!(), "borderSolid", "checksSmall"),
    paint_svg_task("checksSmallOutline", highlight!())
);

block_with_colors!(GRAVEL = c(0x737373), c(0x515151), c(0xaaaaaa),
    color!(),
    paint_svg_task("checksLarge", highlight!()),
    paint_svg_task("diagonalChecksFillBottomLeftTopRight", shadow!())
);

group!(SIMPLE_SOFT_EARTH = GRAVEL, SAND);