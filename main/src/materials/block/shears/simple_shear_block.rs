use crate::image_tasks::color::ComparableColor;
use crate::{paint_stack, single_texture_block, stack_alpha, group};
use crate::image_tasks::task_spec::{paint_svg_task, paint_task};

single_texture_block!(COBWEB = ComparableColor::TRANSPARENT,
    paint_svg_task("ringsCentralBullseye", ComparableColor::WHITE * 0.75),
    paint_task(stack_alpha!(
        "strokeBottomLeftTopRight", "strokeTopLeftBottomRight", "cross"
    ), ComparableColor::WHITE * 0.85)
);

single_texture_block!(VINE = ComparableColor::TRANSPARENT,
    paint_svg_task("wavyVines", ComparableColor::LIGHT_BIOME_COLORABLE),
    paint_svg_task("waves", ComparableColor::DARK_BIOME_COLORABLE)
);

group!(SIMPLE_SHEAR_BLOCKS = COBWEB, VINE);