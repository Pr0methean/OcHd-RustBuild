use crate::{block_with_colors, copy_block, ground_cover_block, group, paint_stack, single_texture_block};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::materials::block::shovel::simple_soft_earth::DIRT;
use crate::materials::block::shovel::simple_soft_earth::POWDER_SNOW;
use crate::stack;
use crate::stack_on;
use crate::texture_base::material::TricolorMaterial;

ground_cover_block!(GRASS_BLOCK = DIRT, c(0x83b253), c(0x64a43a), c(0x9ccb6c),
    stack!(
        paint_svg_task("topPart", color!()),
        paint_svg_task("veesTop", shadow!())
    ),
    stack_on!(
        ComparableColor::LIGHT_BIOME_COLORABLE,
        paint_svg_task("borderShortDashes", ComparableColor::MEDIUM_BIOME_COLORABLE),
        paint_svg_task("borderDotted", ComparableColor::DARK_BIOME_COLORABLE)
    )
);

single_texture_block!(GRASS_BLOCK_SIDE_OVERLAY = ComparableColor::TRANSPARENT,
    paint_svg_task("topPart", ComparableColor::MEDIUM_BIOME_COLORABLE),
    paint_svg_task("veesTop", ComparableColor::DARK_BIOME_COLORABLE)
);

ground_cover_block!(PODZOL = DIRT, c(0x6a4418), c(0x4a3018), c(0x8b5920),
    stack!(
        paint_svg_task("topPart", color!()),
        paint_svg_task("zigzagBrokenTopPart", highlight!())
    ),
    stack_on!(
        c(0x6a4418),
        paint_svg_task("zigzagBroken", highlight!()),
        paint_svg_task("borderDotted", shadow!())
    )
);

copy_block!(COMPOSTER_COMPOST = PODZOL, "top");

single_texture_block!(COMPOSTER_READY = ComparableColor::TRANSPARENT,
    PODZOL.top.to_owned(),
    from_svg_task("bonemealSmallNoBorder")
);

ground_cover_block!(MYCELIUM = DIRT, c(0x6a656a),c(0x5a5a52),c(0x7b6d73),
    stack!(
        paint_svg_task("topPart", color!()),
        paint_svg_task("diagonalChecksTopLeft", shadow!()),
        paint_stack!(highlight!(), "diagonalChecksTopRight",
            "diagonalChecksFillTopLeft"),
        paint_svg_task("diagonalChecksFillTopRight", shadow!())
    ),
    stack_on!(
        color!(),
        paint_svg_task("diagonalChecksTopLeftBottomRight", shadow!()),
        paint_stack!(highlight!(), "diagonalChecksBottomLeftTopRight",
            "diagonalChecksFillTopLeftBottomRight"),
        paint_svg_task("diagonalChecksFillBottomLeftTopRight", shadow!())
    )
);

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

group!(DIRT_GROUND_COVER = GRASS_BLOCK_SIDE_OVERLAY, PODZOL, COMPOSTER_COMPOST, COMPOSTER_READY,
        MYCELIUM, GRASS_BLOCK_SNOW, SNOW);
