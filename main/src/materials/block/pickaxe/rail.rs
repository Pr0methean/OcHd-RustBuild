use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::axe::wood::OAK;
use crate::materials::block::pickaxe::ore::{GOLD, IRON};
use crate::{group, redstone_off_on_block, single_texture_block, stack};

single_texture_block!(
    RAIL = ComparableColor::TRANSPARENT,
    paint_svg_task("railTies", OAK.color),
    paint_svg_task("rail", IRON.refined_colors.shadow)
);
single_texture_block!(
    RAIL_CORNER = ComparableColor::TRANSPARENT,
    paint_svg_task("railTieCorner", OAK.color),
    paint_svg_task("railCorner", IRON.refined_colors.shadow)
);
redstone_off_on_block!(
    POWERED_RAIL = stack!(
        paint_svg_task("railTies", OAK.shadow),
        paint_svg_task("thirdRail", state_color!()),
        paint_svg_task("rail", GOLD.colors.color)
    )
);
redstone_off_on_block!(
    ACTIVATOR_RAIL = stack!(
        paint_svg_task("railTies", OAK.shadow),
        paint_svg_task("thirdRail", state_color!()),
        paint_svg_task("rail", IRON.refined_colors.shadow)
    )
);
redstone_off_on_block!(
    DETECTOR_RAIL = stack!(
        paint_svg_task("railTies", OAK.shadow),
        paint_svg_task("railDetectorPlate", state_color!()),
        paint_svg_task("rail", IRON.refined_colors.shadow)
    )
);

group!(
    RAILS = RAIL,
    RAIL_CORNER,
    POWERED_RAIL,
    ACTIVATOR_RAIL,
    DETECTOR_RAIL
);
