use crate::{ground_cover_block, stack, paint_stack, stack_on, group};
use crate::image_tasks::color::c;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::pickaxe::ore_base::NETHERRACK;

ground_cover_block!(CRIMSON_NYLIUM = NETHERRACK,
    c(0x854242),
    c(0x7b0000),
    c(0xbd3030),

    stack!(
        paint_svg_task("topPart", color!()),
        paint_svg_task("strokeTopLeftBottomRight2TopPart", shadow!()),
        paint_svg_task("strokeBottomLeftTopRight2TopPart", highlight!())
    ),
    stack_on!(
        color!(),
        paint_svg_task("strokeTopLeftBottomRight2", shadow!()),
        paint_stack!(highlight!(), "strokeBottomLeftTopRight2",
                "borderLongDashes")
    )
);

ground_cover_block!(WARPED_NYLIUM = NETHERRACK,
    c(0x568353),
    c(0x456b52),
    c(0xac2020),

    stack!(
        paint_svg_task("topPart", color!()),
        paint_svg_task("strokeTopLeftBottomRight2TopPart", highlight!()),
        paint_svg_task("strokeBottomLeftTopRight2TopPart", shadow!())
    ),
    stack_on!(
        color!(),
        paint_svg_task("strokeTopLeftBottomRight2", highlight!()),
        paint_stack!(shadow!(), "strokeBottomLeftTopRight2",
                "borderLongDashes")
    )
);

group!(NYLIUM = CRIMSON_NYLIUM, WARPED_NYLIUM);
