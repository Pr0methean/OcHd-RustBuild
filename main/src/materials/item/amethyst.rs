use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::single_texture_item;
use crate::materials::block::pickaxe::simple_pickaxe_block::AMETHYST_BLOCK;
use crate::texture_base::material::TricolorMaterial;

single_texture_item!(AMETHYST_SHARD = ComparableColor::TRANSPARENT,
    paint_svg_task("trianglesSmallCenter1", AMETHYST_BLOCK.highlight()),
    paint_svg_task("trianglesSmallCenter2", AMETHYST_BLOCK.color())
);