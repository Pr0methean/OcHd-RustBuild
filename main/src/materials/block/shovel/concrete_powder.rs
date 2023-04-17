use crate::{dyed_block, stack_on};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::paint_svg_task;

dyed_block!(CONCRETE_POWDER = stack_on!(color!(),
    paint_svg_task("checksSmallOutline", ComparableColor::STONE_SHADOW * 0.5),
    paint_svg_task("checksLarge", ComparableColor::STONE_HIGHLIGHT * 0.5)
));