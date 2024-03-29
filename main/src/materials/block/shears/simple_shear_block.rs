use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{paint_svg_task, paint_task, stack_alpha, svg_alpha_task};
use crate::{group, single_texture_block, stack_alpha};

single_texture_block!(
    COBWEB = ComparableColor::TRANSPARENT,
    paint_task(
        stack_alpha(vec![
            svg_alpha_task("ringsCentralBullseye") * 0.75,
            stack_alpha!(
                "strokeBottomLeftTopRight",
                "strokeTopLeftBottomRight",
                "cross"
            ) * 0.85
        ]),
        ComparableColor::WHITE * 0.85
    )
);

single_texture_block!(
    VINE = ComparableColor::TRANSPARENT,
    paint_svg_task("wavyVines", ComparableColor::LIGHT_BIOME_COLORABLE),
    paint_svg_task("waves", ComparableColor::DARK_BIOME_COLORABLE)
);

group!(SIMPLE_SHEAR_BLOCKS = COBWEB, VINE);
