use crate::{group, single_layer_item, single_texture_item};
use crate::image_tasks::color::c;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::bare_hand::simple_bare_hand_block::{HONEYCOMB_BORDER, HONEYCOMB_CELLS};
use crate::materials::block::pickaxe::simple_pickaxe_block::AMETHYST_BLOCK;
use crate::texture_base::material::TricolorMaterial;

single_layer_item!(BONE = "boneBottomLeftTopRight", c(0xeaead0));
single_layer_item!(BONE_MEAL = "bonemealSmall");

single_texture_item!(HONEYCOMB =
    paint_svg_task("honeycombBorder", HONEYCOMB_BORDER),
    paint_svg_task("honeycombNoHalfCells", HONEYCOMB_CELLS)
);

single_texture_item!(AMETHYST_SHARD =
    paint_svg_task("trianglesSmallCenter1", AMETHYST_BLOCK.highlight()),
    paint_svg_task("trianglesSmallCenter2", AMETHYST_BLOCK.color())
);

// TODO: Rotten flesh

group!(SIMPLE_ITEMS = BONE, BONE_MEAL, HONEYCOMB, AMETHYST_SHARD);