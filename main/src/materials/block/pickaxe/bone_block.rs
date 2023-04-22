use crate::{block_with_colors, group, paint_stack};
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::paint_svg_task;

const COLOR: ComparableColor = c(0xe1ddca);
const SHADOW: ComparableColor = c(0xc3bfa1);
const HIGHLIGHT: ComparableColor = c(0xeaead0);

block_with_colors!(BONE_BLOCK_TOP = COLOR, SHADOW, HIGHLIGHT,
    shadow!(),
    paint_stack!(highlight!(), "borderSolid", "boneBottomLeftTopRightNoCross"),
    paint_svg_task("boneTopLeftBottomRightNoCross", color!())
);

block_with_colors!(BONE_BLOCK_SIDE = COLOR, SHADOW, HIGHLIGHT,
    color!(),
    paint_stack!(shadow!(), "borderSolid", "boneBottomLeftTopRightNoCross"),
    paint_stack!(highlight!(), "borderDotted", "boneTopLeftBottomRightNoCross")
);

group!(BONE_BLOCK = BONE_BLOCK_TOP, BONE_BLOCK_SIDE);
