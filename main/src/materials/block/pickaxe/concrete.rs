use crate::{dyed_block, stack_on};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::paint_svg_task;
dyed_block!(CONCRETE = stack_on!(color!(),
        paint_svg_task("strokeBottomLeftTopRight2", ComparableColor::STONE * 0.25),
        paint_svg_task("strokeTopLeftBottomRight2", ComparableColor::STONE_EXTREME_SHADOW * 0.25),
        paint_svg_task("borderDotted", ComparableColor::STONE_EXTREME_HIGHLIGHT * 0.5)
));