use crate::{block_with_colors, group, paint_stack, single_texture_block};
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::materials::block::pickaxe::simple_pickaxe_block::END_STONE;

block_with_colors!(BEDROCK =
    ComparableColor::STONE_EXTREME_SHADOW,
    ComparableColor::DARKEST_GRAY,
    ComparableColor::STONE_HIGHLIGHT,

    color!(),
    paint_stack!(shadow!(), "borderSolid", "strokeTopLeftBottomRight2"),
    paint_svg_task("strokeBottomLeftTopRight2", highlight!())
);

single_texture_block!(END_PORTAL_FRAME_SIDE =
    ComparableColor::TRANSPARENT,
    END_STONE.material.texture.to_owned(),
    paint_svg_task("endPortalFrameSide", c(0x26002a))
);

single_texture_block!(END_PORTAL_FRAME_TOP =
    ComparableColor::TRANSPARENT,
    END_STONE.material.texture.to_owned(),
    paint_svg_task("endPortalFrameTop", c(0x26002a)),
    from_svg_task("railDetector") // Good shape & size for pearl hole
);

group!(SIMPLE_INDESTRUCTIBLE_BLOCKS = BEDROCK, END_PORTAL_FRAME_SIDE, END_PORTAL_FRAME_TOP);
