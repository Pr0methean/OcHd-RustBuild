use crate::{dyed_block, stack_on};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::pickaxe::simple_pickaxe_block::TERRACOTTA as PLAIN_TERRACOTTA;
use crate::texture_base::material::TricolorMaterial;

dyed_block!(TERRACOTTA = stack_on!(color!(),
    paint_svg_task("bigDotsBottomLeftTopRight", PLAIN_TERRACOTTA.shadow() * 0.5),
    paint_svg_task("bigDotsTopLeftBottomRight", PLAIN_TERRACOTTA.highlight() * 0.5),
    paint_svg_task("bigRingsTopLeftBottomRight", PLAIN_TERRACOTTA.highlight()),
    paint_svg_task("bigRingsBottomLeftTopRight", PLAIN_TERRACOTTA.shadow()),
    paint_svg_task("borderRoundDots", PLAIN_TERRACOTTA.color())
));