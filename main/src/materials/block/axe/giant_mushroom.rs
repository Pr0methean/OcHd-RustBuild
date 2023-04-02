use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::{group, single_texture_block};
use crate::stack_on;
use crate::repaint_stack;

single_texture_block!(RED_MUSHROOM_BLOCK = stack_on!(
    ComparableColor::RED,
    repaint_stack!(ComparableColor::WHITE,
            from_svg_task("bigDotsTopLeftBottomRight"),
            from_svg_task("dots0"),
            from_svg_task("borderRoundDots")
    )
));
single_texture_block!(BROWN_MUSHROOM_BLOCK = stack_on!(
    c(0x915431),
    paint_svg_task("rings", c(0x9d825e))
));
single_texture_block!(MUSHROOM_STEM = stack_on!(
    c(0xd0d0c4),
    paint_svg_task("stripesThick", c(0xc0c0ac)),
    paint_svg_task("borderShortDashes", c(0xc4c4b4))
));
single_texture_block!(MUSHROOM_BLOCK_INSIDE = stack_on!(
    c(0xc7a877),
        repaint_stack!(ComparableColor::WHITE,
            from_svg_task("bigDotsTopLeftBottomRight"),
            from_svg_task("dots0"),
            from_svg_task("borderRoundDots"))
));
group!(GIANT_MUSHROOM = RED_MUSHROOM_BLOCK, BROWN_MUSHROOM_BLOCK, MUSHROOM_STEM, MUSHROOM_BLOCK_INSIDE);