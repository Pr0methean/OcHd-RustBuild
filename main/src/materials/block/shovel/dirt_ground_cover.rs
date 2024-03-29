use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::materials::block::shovel::simple_soft_earth::DIRT;
use crate::materials::block::shovel::simple_soft_earth::POWDER_SNOW;
use crate::stack;
use crate::stack_on;
use crate::texture_base::material::{ground_cover_block, GroundCoverBlock, TricolorMaterial};
use crate::{block_with_colors, copy_block, group, paint_stack, single_texture_block};
use once_cell::sync::Lazy;

pub const GRASS_COLOR: ComparableColor = c(0x83b253);
pub const GRASS_SHADOW: ComparableColor = c(0x64a43a);
pub const GRASS_HIGHLIGHT: ComparableColor = c(0x9ccb6c);

pub static GRASS_BLOCK: Lazy<GroundCoverBlock> = Lazy::new(|| {
    ground_cover_block(
        "grass_block",
        "_top",
        &DIRT.material,
        GRASS_COLOR,
        GRASS_SHADOW,
        GRASS_HIGHLIGHT,
        stack!(
            paint_svg_task("topPart", GRASS_COLOR),
            paint_svg_task("veesTop", GRASS_SHADOW)
        ),
        stack_on!(
            ComparableColor::LIGHT_BIOME_COLORABLE,
            paint_stack!(
                ComparableColor::MEDIUM_BIOME_COLORABLE,
                "borderDotted",
                "vees"
            )
        ),
    )
});

single_texture_block!(
    GRASS_BLOCK_SIDE_OVERLAY = ComparableColor::TRANSPARENT,
    paint_svg_task("topPart", ComparableColor::LIGHT_BIOME_COLORABLE),
    paint_svg_task("veesTop", ComparableColor::MEDIUM_BIOME_COLORABLE)
);

pub const PODZOL_COLOR: ComparableColor = c(0x6a4418);
pub const PODZOL_SHADOW: ComparableColor = c(0x4a3018);
pub const PODZOL_HIGHLIGHT: ComparableColor = c(0x8b5920);

pub static PODZOL: Lazy<GroundCoverBlock> = Lazy::new(|| {
    ground_cover_block(
        "podzol",
        "_top",
        &DIRT.material,
        PODZOL_COLOR,
        PODZOL_SHADOW,
        PODZOL_HIGHLIGHT,
        stack!(
            paint_svg_task("topPart", PODZOL_COLOR),
            paint_svg_task("zigzagBrokenTopPart", PODZOL_HIGHLIGHT)
        ),
        stack_on!(
            PODZOL_COLOR,
            paint_svg_task("zigzagBroken", PODZOL_HIGHLIGHT),
            paint_svg_task("borderDotted", PODZOL_SHADOW)
        ),
    )
});

copy_block!(COMPOSTER_COMPOST = PODZOL, "top");

single_texture_block!(
    COMPOSTER_READY = ComparableColor::TRANSPARENT,
    PODZOL.top.to_owned(),
    from_svg_task("bonemealSmallNoBorder")
);

pub const MYCELIUM_COLOR: ComparableColor = c(0x6a656a);
pub const MYCELIUM_SHADOW: ComparableColor = c(0x5a5a52);
pub const MYCELIUM_HIGHLIGHT: ComparableColor = c(0x7b6d73);

pub static MYCELIUM: Lazy<GroundCoverBlock> = Lazy::new(|| {
    ground_cover_block(
        "mycelium",
        "_top",
        &DIRT.material,
        MYCELIUM_COLOR,
        MYCELIUM_SHADOW,
        MYCELIUM_HIGHLIGHT,
        stack!(
            paint_svg_task("topPart", MYCELIUM_COLOR),
            paint_svg_task("mushroomTopRight", MYCELIUM_SHADOW),
            paint_svg_task("mushroomTopLeft", MYCELIUM_HIGHLIGHT),
        ),
        stack_on!(
            MYCELIUM_COLOR,
            paint_svg_task("mushroomsBottomLeftTopRight", MYCELIUM_SHADOW),
            paint_svg_task("mushroomsTopLeftBottomRight", MYCELIUM_HIGHLIGHT)
        ),
    )
});

block_with_colors!(
    GRASS_BLOCK_SNOW = POWDER_SNOW.color(),
    POWDER_SNOW.shadow(),
    POWDER_SNOW.highlight(),
    ComparableColor::TRANSPARENT,
    DIRT.material.texture(),
    paint_svg_task("topPart", color!()),
    paint_svg_task("diagonalChecksTopLeft", shadow!()),
    paint_stack!(
        highlight!(),
        "diagonalChecksTopRight",
        "diagonalChecksFillTopLeft"
    ),
    paint_svg_task("diagonalChecksFillTopRight", shadow!())
);

block_with_colors!(
    SNOW = POWDER_SNOW.color(),
    POWDER_SNOW.shadow(),
    POWDER_SNOW.highlight(),
    color!(),
    paint_svg_task("snow", shadow!())
);

group!(
    DIRT_GROUND_COVER = GRASS_BLOCK,
    GRASS_BLOCK_SIDE_OVERLAY,
    PODZOL,
    COMPOSTER_COMPOST,
    COMPOSTER_READY,
    MYCELIUM,
    GRASS_BLOCK_SNOW,
    SNOW
);
