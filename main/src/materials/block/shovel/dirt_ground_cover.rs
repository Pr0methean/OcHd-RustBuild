use lazy_static::lazy_static;
use crate::{block_with_colors, copy_block, group, paint_stack, single_texture_block};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::materials::block::shovel::simple_soft_earth::DIRT;
use crate::materials::block::shovel::simple_soft_earth::POWDER_SNOW;
use crate::stack;
use crate::stack_on;
use crate::texture_base::material::{GroundCoverBlock, TricolorMaterial, ground_cover_block};

pub const GRASS_COLOR: ComparableColor = c(0x83b253);
pub const GRASS_SHADOW: ComparableColor = c(0x64a43a);
pub const GRASS_HIGHLIGHT: ComparableColor = c(0x9ccb6c);

lazy_static!{
    pub static ref GRASS_BLOCK: GroundCoverBlock = ground_cover_block(
        "grass_block", "_top", &DIRT.material, GRASS_COLOR, GRASS_SHADOW, GRASS_HIGHLIGHT,
        stack!(
            paint_svg_task("topPart", GRASS_COLOR),
            paint_svg_task("veesTop", GRASS_SHADOW)
        ),
        stack_on!(
            ComparableColor::LIGHT_BIOME_COLORABLE,
            paint_stack!(ComparableColor::MEDIUM_BIOME_COLORABLE, "borderDotted", "vees")
        )
    );
}

single_texture_block!(GRASS_BLOCK_SIDE_OVERLAY = ComparableColor::TRANSPARENT,
    paint_svg_task("topPart", ComparableColor::LIGHT_BIOME_COLORABLE),
    paint_svg_task("veesTop", ComparableColor::MEDIUM_BIOME_COLORABLE)
);

pub const PODZOL_COLOR: ComparableColor = c(0x6a4418);
pub const PODZOL_SHADOW: ComparableColor = c(0x4a3018);
pub const PODZOL_HIGHLIGHT: ComparableColor = c(0x8b5920);

lazy_static! {
    pub static ref PODZOL: GroundCoverBlock = ground_cover_block(
        "podzol", "_top", &DIRT.material, PODZOL_COLOR, PODZOL_SHADOW, PODZOL_HIGHLIGHT,
        stack!(
            paint_svg_task("topPart", PODZOL_COLOR),
            paint_svg_task("zigzagBrokenTopPart", PODZOL_HIGHLIGHT)
        ),
        stack_on!(
            PODZOL_COLOR,
            paint_svg_task("zigzagBroken", PODZOL_HIGHLIGHT),
            paint_svg_task("borderDotted", PODZOL_SHADOW)
        )
    );
}

copy_block!(COMPOSTER_COMPOST = PODZOL, "top");

single_texture_block!(COMPOSTER_READY = ComparableColor::TRANSPARENT,
    PODZOL.top.to_owned(),
    from_svg_task("bonemealSmallNoBorder")
);

pub const MYCELIUM_COLOR: ComparableColor = c(0x6a656a);
pub const MYCELIUM_SHADOW: ComparableColor = c(0x5a5a52);
pub const MYCELIUM_HIGHLIGHT: ComparableColor = c(0x7b6d73);

lazy_static! {
    pub static ref MYCELIUM: GroundCoverBlock = ground_cover_block(
        "mycelium", "_top", &DIRT.material, MYCELIUM_COLOR, MYCELIUM_SHADOW, MYCELIUM_HIGHLIGHT,
        stack!(
            paint_svg_task("topPart", MYCELIUM_COLOR),
            paint_svg_task("diagonalChecksTopLeft", MYCELIUM_SHADOW),
            paint_stack!(MYCELIUM_HIGHLIGHT, "diagonalChecksTopRight",
                "diagonalChecksFillTopLeft"),
            paint_svg_task("diagonalChecksFillTopRight", MYCELIUM_SHADOW)
        ),
        stack_on!(
            MYCELIUM_COLOR,
            paint_svg_task("diagonalChecksTopLeftBottomRight", MYCELIUM_SHADOW),
            paint_stack!(MYCELIUM_HIGHLIGHT, "diagonalChecksBottomLeftTopRight",
                "diagonalChecksFillTopLeftBottomRight"),
            paint_svg_task("diagonalChecksFillBottomLeftTopRight", MYCELIUM_SHADOW)
        )
    );
}

block_with_colors!(GRASS_BLOCK_SNOW =
    POWDER_SNOW.color(),
    POWDER_SNOW.shadow(),
    POWDER_SNOW.highlight(),

    ComparableColor::TRANSPARENT,
    DIRT.material.texture.to_owned(),
    paint_svg_task("topPart", color!()),
    paint_svg_task("diagonalChecksTopLeft", shadow!()),
    paint_stack!(highlight!(), "diagonalChecksTopRight",
        "diagonalChecksFillTopLeft"),
    paint_svg_task("diagonalChecksFillTopRight", shadow!())
);

block_with_colors!(SNOW =
    POWDER_SNOW.color(),
    POWDER_SNOW.shadow(),
    POWDER_SNOW.highlight(),

    color!(),
    paint_svg_task("snow", shadow!())
);

group!(DIRT_GROUND_COVER = GRASS_BLOCK,
    GRASS_BLOCK_SIDE_OVERLAY, PODZOL, COMPOSTER_COMPOST, COMPOSTER_READY,
        MYCELIUM, GRASS_BLOCK_SNOW, SNOW);
