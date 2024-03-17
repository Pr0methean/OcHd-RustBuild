use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::pickaxe::simple_pickaxe_block::TERRACOTTA as PLAIN_TERRACOTTA;
use crate::texture_base::material::TricolorMaterial;
use crate::{dyed_block, stack_on};

dyed_block!(
    TERRACOTTA = stack_on!(
        color!(),
        paint_svg_task("bigDotsBottomLeftTopRight", PLAIN_TERRACOTTA.shadow()),
        paint_svg_task("bigDotsTopLeftBottomRight", PLAIN_TERRACOTTA.highlight()),
        paint_svg_task("bigDotsFillTopLeftBottomRight", color!() * 0.5),
        paint_svg_task("bigDotsFillBottomLeftTopRight", color!() * 0.5),
        paint_svg_task("borderRoundDots", PLAIN_TERRACOTTA.color())
    )
);
