use crate::image_tasks::color::c;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::bare_hand::simple_bare_hand_block::{
    HONEYCOMB_BORDER, HONEYCOMB_CELLS,
};
use crate::materials::block::pickaxe::simple_pickaxe_block::AMETHYST_BLOCK;
use crate::texture_base::material::TricolorMaterial;
use crate::{group, paint_stack, single_layer_item, single_texture_item};

single_layer_item!(BONE = "boneBottomLeftTopRight", c(0xeaead0));
single_layer_item!(BONE_MEAL = "bonemealSmall");

single_texture_item!(
    HONEYCOMB = paint_svg_task("honeycombBorder", HONEYCOMB_BORDER),
    paint_svg_task("honeycombNoHalfCells", HONEYCOMB_CELLS)
);

single_texture_item!(
    AMETHYST_SHARD = paint_svg_task("trianglesSmallCenter1", AMETHYST_BLOCK.highlight()),
    paint_svg_task("trianglesSmallCenter2", AMETHYST_BLOCK.color())
);

single_texture_item!(
    SALMON = paint_stack!(c(0xbd928b), "fishTail", "fishFins"),
    paint_svg_task("fishBody", c(0xbe4644))
);

single_texture_item!(
    COOKED_SALMON = paint_stack!(c(0xd39c74), "fishTail", "fishFins"),
    paint_svg_task("fishBody", c(0xba4f23)),
    paint_svg_task("fishStripe", c(0xdf7d53))
);

single_texture_item!(
    COD = paint_stack!(c(0xd6c5ad), "fishTail", "fishFins"),
    paint_svg_task("fishBody", c(0xc6a271))
);

single_texture_item!(
    COOKED_COD =
    // Cod loses its fins when cooked, somehow
    paint_svg_task("fishTail", c(0xd6c5ad)),
    paint_svg_task("fishBody", c(0xe2e5c6)),
    paint_svg_task("fishStripe", c(0xae8b67))
);

// TODO: Rotten flesh

group!(
    SIMPLE_ITEMS = BONE,
    BONE_MEAL,
    HONEYCOMB,
    AMETHYST_SHARD,
    SALMON,
    COOKED_SALMON,
    COD,
    COOKED_COD
);
