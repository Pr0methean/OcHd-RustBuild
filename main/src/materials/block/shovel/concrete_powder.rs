use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::{dyed_block, stack_on};

dyed_block!(
    CONCRETE_POWDER = stack_on!(
        color!(),
        paint_svg_task("checksSmallOutline", ComparableColor::STONE_SHADOW * 0.75),
        paint_svg_task("checksLarge", ComparableColor::STONE_HIGHLIGHT * 0.5)
    )
);
