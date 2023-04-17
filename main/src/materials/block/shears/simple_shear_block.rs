use crate::image_tasks::color::ComparableColor;
use crate::{paint_stack, single_texture_block, group};
use crate::image_tasks::task_spec::paint_svg_task;

single_texture_block!(COBWEB = ComparableColor::TRANSPARENT,
    paint_svg_task("ringsCentralBullseye", ComparableColor::WHITE * 0.75),
    paint_stack!(ComparableColor::WHITE * 0.85,
        "strokeBottomLeftTopRight", "strokeTopLeftBottomRight", "cross"
    )
);

single_texture_block!(VINE = ComparableColor::TRANSPARENT,
    paint_svg_task("wavyVines", ComparableColor::LIGHT_BIOME_COLORABLE),
    paint_svg_task("waves", ComparableColor::DARK_BIOME_COLORABLE)
);

group!(SIMPLE_SHEAR_BLOCKS = COBWEB, VINE);