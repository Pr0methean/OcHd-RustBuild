use crate::image_tasks::color::ComparableColor;
use crate::{dyed_block, paint_stack, stack_on};

dyed_block!(
    WOOL = stack_on!(
        color!(),
        paint_stack!(ComparableColor::BLACK * 0.25, "zigzagBroken", "borderSolid"),
        paint_stack!(
            ComparableColor::WHITE * 0.25,
            "zigzagBroken2",
            "borderDotted"
        )
    )
);
