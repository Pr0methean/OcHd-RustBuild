use crate::{block_with_colors, group, paint_stack};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::texture_base::material::TricolorMaterial;

block_with_colors!(SAND = c(0xddddaa), c(0xd1ba8a), c(0xeaead0),
    color!(),
    paint_stack!(shadow!(), "borderSolid", "checksSmall"),
    paint_svg_task("checksSmallOutline", highlight!())
);
block_with_colors!(GRAVEL = c(0x737373), c(0x515151), c(0xaaaaaa),
    color!(),
    paint_svg_task("checksLarge", highlight!()),
    paint_stack!(shadow!(), "diagonalChecksTopLeftBottomRight", "diagonalChecksFillBottomLeftTopRight"),
    paint_svg_task("diagonalChecksFillerSquaresTopLeftBottomRight", highlight!())
);
block_with_colors!(RED_SAND = c(0xbf6721), c(0xac5700), c(0xd97b30),
    color!(),
    paint_stack!(highlight!(), "borderSolid", "checksSmall"),
    paint_svg_task("checksSmallOutline", shadow!())
);
block_with_colors!(CLAY = c(0x9e9eb3), c(0x9494a4), c(0xA8BEC5),
    color!(),
    paint_svg_task("diagonalChecksTopLeftBottomRight", highlight!()),
    paint_stack!(shadow!(), "diagonalChecksBottomLeftTopRight",
        "diagonalChecksFillerSquaresTopLeftBottomRight"),
    paint_svg_task("diagonalChecksFillerSquaresBottomLeftTopRight", highlight!())
);
block_with_colors!(MUD = c(0x3a3a3a), c(0x333333), c(0x515151),
    color!(),
    paint_svg_task("strokeTopLeftBottomRight2", highlight!()),
    paint_svg_task("strokeBottomLeftTopRight2", shadow!()),
    paint_svg_task("borderSolid", highlight!()),
    paint_svg_task("borderDotted", shadow!())
);
block_with_colors!(MOSS_BLOCK = c(0x647233),c(0x42552d),c(0x70922d),
    color!(),
    paint_stack!(highlight!(), "strokeTopLeftBottomRight4", "borderSolid"),
    paint_svg_task("strokeBottomLeftTopRight4xorBorder", shadow!())
);
block_with_colors!(SOUL_SAND = c(0x624033), c(0x3F2D23), c(0x915431),
    color!(),
    from_svg_task("borderSolid"),
    paint_svg_task("borderSolidTopLeftBottomRight", color!()),
    paint_svg_task("checksSmall", highlight!()),
    paint_svg_task("bigDotsTopLeftBottomRight", shadow!()),
    paint_svg_task("soulFaces", highlight!())
);
block_with_colors!(SOUL_SOIL = c(0x3F2D23), ComparableColor::BLACK, c(0x915431),
    shadow!(),
    paint_svg_task("borderSolid", color!()),
    paint_svg_task("borderSolidTopLeftBottomRight", shadow!()),
    paint_stack!(highlight!(), "strokeBottomLeftTopRight4xorBorder", "bigDotsTopLeftBottomRight"),
    paint_svg_task("soulFaces", color!())
);
block_with_colors!(PACKED_MUD = c(0x8c674f),c(0x5e4841),c(0xab8661),
    color!(),
    paint_svg_task("strokeTopLeftBottomRight2", highlight!()),
    paint_svg_task("strokeBottomLeftTopRight2", shadow!()),
    paint_svg_task("borderDotted", MUD.highlight())
);
block_with_colors!(FARMLAND = c(0x966c4a),c(0x593d29),c(0xb9855c),
    highlight!(),
    paint_svg_task("bambooThick", color!()),
    paint_svg_task("bambooThinMinusBorder", shadow!()),
    paint_svg_task("dots0", ComparableColor::BLACK)
);
block_with_colors!(FARMLAND_MOIST = c(0x552e00),c(0x341900),c(0x6e3c15),
    highlight!(),
    paint_svg_task("bambooThick", color!()),
    paint_svg_task("bambooThinMinusBorder", shadow!()),
    paint_svg_task("dots0", ComparableColor::STONE_SHADOW)
);
block_with_colors!(DIRT = c(0x966c4a), c(0x593d29), c(0xb9855c),
    color!(),
    paint_svg_task("dots3", shadow!()),
    paint_stack!(highlight!(), "dots2", "borderDotted")
);
block_with_colors!(POWDER_SNOW = ComparableColor::WHITE,  c(0xcfcfdf), ComparableColor::WHITE,
    color!(),
    paint_svg_task("snowXorChecksSmall", shadow!())
);

group!(SIMPLE_SOFT_EARTH = GRAVEL, SAND, RED_SAND, CLAY, MUD, MOSS_BLOCK, SOUL_SAND, SOUL_SOIL,
        PACKED_MUD, FARMLAND, FARMLAND_MOIST, DIRT, POWDER_SNOW);