use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::{group, make_tricolor_block_macro};

make_tricolor_block_macro!(tnt, c(0xdb2f00), c(0x912d00), c(0xff4300));

tnt!(
    TNT_BOTTOM = ComparableColor::BLACK,
    paint_svg_task("tntSticksEnd", color!())
);

tnt!(
    TNT_TOP = ComparableColor::TRANSPARENT,
    TNT_BOTTOM.material.texture(),
    from_svg_task("tntFuzes")
);

tnt!(
    TNT_SIDE = shadow!(),
    paint_svg_task("tntSticksSide", color!()),
    paint_svg_task("borderDotted", highlight!()),
    paint_svg_task("tntStripe", ComparableColor::WHITE),
    from_svg_task("tntSign")
);

group!(TNT = TNT_BOTTOM, TNT_TOP, TNT_SIDE);
