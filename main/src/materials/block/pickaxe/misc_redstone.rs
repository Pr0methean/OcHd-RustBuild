use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task, ToPixmapTaskSpec};
use crate::materials::block::pickaxe::ore::REDSTONE;
use crate::materials::block::pickaxe::simple_pickaxe_block::SMOOTH_STONE;
use crate::texture_base::material::TricolorMaterial;
use crate::{
    block_with_colors, group, paint_stack, redstone_off_on_block, single_texture_block, stack,
};
use once_cell::sync::Lazy;

static RC_BASE: Lazy<ToPixmapTaskSpec> = Lazy::new(|| {
    stack!(
        SMOOTH_STONE.material.texture(),
        paint_svg_task("repeaterSideInputs", ComparableColor::STONE_SHADOW)
    )
});

redstone_off_on_block!(
    REPEATER = stack!(
        RC_BASE.to_owned(),
        paint_svg_task("repeater", state_color!())
    )
);

redstone_off_on_block!(
    COMPARATOR = stack!(
        RC_BASE.to_owned(),
        paint_svg_task("comparator", state_color!())
    )
);

single_texture_block!(
    REDSTONE_LAMP = REDSTONE.color(),
    from_svg_task("borderSolid"),
    paint_svg_task("borderSolidTopLeft", REDSTONE.highlight()),
    paint_stack!(REDSTONE.shadow(), "glow", "redstone")
);

block_with_colors!(
    REDSTONE_LAMP_ON = c(0xe6994a),
    c(0x946931),
    c(0xFFCDB2),
    color!(),
    paint_stack!(shadow!(), "borderSolid", "redstone"),
    paint_svg_task("borderSolidTopLeft", highlight!()),
    paint_svg_task("glow", highlight!()),
    paint_svg_task("railDetector", ComparableColor::WHITE)
);

group!(
    MISC_REDSTONE = REPEATER,
    COMPARATOR,
    REDSTONE_LAMP,
    REDSTONE_LAMP_ON
);
