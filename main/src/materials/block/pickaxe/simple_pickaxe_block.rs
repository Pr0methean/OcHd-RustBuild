use lazy_static::lazy_static;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::materials::block::pickaxe::ore_base::DEEPSLATE;
use crate::{group, paint_stack, single_texture_block};
use crate::materials::block::pickaxe::ore::QUARTZ;
use crate::texture_base::material::SingleTextureMaterial;
use crate::texture_base::material::block;

single_texture_block!(DEEPSLATE_BRICKS = ComparableColor::TRANSPARENT,
    DEEPSLATE.material.texture.to_owned(),
    paint_svg_task("bricksSmall", ComparableColor::DEEPSLATE_SHADOW),
    paint_svg_task("borderDotted", ComparableColor::DEEPSLATE_HIGHLIGHT),
    paint_svg_task("borderDottedBottomRight", ComparableColor::DEEPSLATE_SHADOW)
);

single_texture_block!(DEEPSLATE_TOP = ComparableColor::TRANSPARENT,
        DEEPSLATE.material.texture.to_owned(),
        paint_svg_task("cross", ComparableColor::DEEPSLATE_SHADOW),
        paint_svg_task("borderSolid", ComparableColor::DEEPSLATE_HIGHLIGHT)
);

single_texture_block!(QUARTZ_BLOCK_TOP = QUARTZ.colors.color,
                paint_svg_task("borderSolid", QUARTZ.colors.shadow),
                paint_stack!(QUARTZ.colors.highlight, "borderSolidTopLeft", "streaks")
);

lazy_static! {
    pub static ref QUARTZ_BLOCK_BOTTOM: SingleTextureMaterial = block("quartz_block_bottom",
            (QUARTZ.refined_block)(&QUARTZ));
    pub static ref QUARTZ_BLOCK_SIDE: SingleTextureMaterial = block("quartz_block_side",
            (QUARTZ.raw_block)(&QUARTZ));
}

group!(SIMPLE_PICKAXE_BLOCKS = DEEPSLATE_BRICKS, DEEPSLATE_TOP, QUARTZ_BLOCK_TOP,
        QUARTZ_BLOCK_BOTTOM, QUARTZ_BLOCK_SIDE);