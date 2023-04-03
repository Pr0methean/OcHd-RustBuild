use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{paint_svg_task};
use crate::{group, single_texture_block};
use crate::paint_stack;

single_texture_block!(RED_MUSHROOM_BLOCK = ComparableColor::RED,
    paint_stack!(ComparableColor::WHITE, "bigDotsTopLeftBottomRight", "dots0", "borderRoundDots")
);
single_texture_block!(BROWN_MUSHROOM_BLOCK = c(0x915431),
    paint_svg_task("rings", c(0x9d825e))
);
single_texture_block!(MUSHROOM_STEM = c(0xd0d0c4),
    paint_stack!(c(0xc0c0ac), "stripesThick", "borderShortDashes")
);
single_texture_block!(MUSHROOM_BLOCK_INSIDE = c(0xD7C187),
    paint_stack!(c(0xab9066), "bigDotsTopLeftBottomRight", "dots0", "borderRoundDots")
);
group!(GIANT_MUSHROOM = RED_MUSHROOM_BLOCK, BROWN_MUSHROOM_BLOCK, MUSHROOM_STEM, MUSHROOM_BLOCK_INSIDE);