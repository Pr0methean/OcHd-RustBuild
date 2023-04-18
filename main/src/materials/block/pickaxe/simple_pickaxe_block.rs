use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{paint_svg_task, ToPixmapTaskSpec};
use crate::materials::block::pickaxe::ore_base::DEEPSLATE;
use crate::{block_with_colors, group, paint_stack, single_texture_block, stack_on};
use crate::materials::block::pickaxe::ore::QUARTZ;
use crate::materials::block::shovel::simple_soft_earth::{MOSS_BLOCK, RED_SAND, MUD, PACKED_MUD};
use crate::texture_base::material::{SingleTextureMaterial, TricolorMaterial};
use crate::texture_base::material::block;

single_texture_block!(DEEPSLATE_BRICKS = ComparableColor::TRANSPARENT,
    DEEPSLATE.material.texture.to_owned(),
    paint_svg_task("bricksSmall", DEEPSLATE.shadow()),
    paint_svg_task("borderDotted", DEEPSLATE.highlight()),
    paint_svg_task("borderDottedBottomRight", DEEPSLATE.shadow())
);

single_texture_block!(DEEPSLATE_TOP = ComparableColor::TRANSPARENT,
        DEEPSLATE.material.texture.to_owned(),
        paint_svg_task("cross", DEEPSLATE.shadow()),
        paint_svg_task("borderSolid", DEEPSLATE.highlight())
);

macro_rules! quartz {
    ($name:ident = $background:expr, $($layers:expr),*) => {
        crate::block_with_colors!($name =
            crate::materials::block::pickaxe::ore::QUARTZ.color(),
            crate::materials::block::pickaxe::ore::QUARTZ.shadow(),
            crate::materials::block::pickaxe::ore::QUARTZ.highlight(),

            $background,
            $($layers),*
        );
    }
}

quartz!(QUARTZ_BLOCK_TOP = color!(),
                paint_svg_task("borderSolid", shadow!()),
                paint_stack!(highlight!(), "borderSolidTopLeft", "streaks")
);

lazy_static! {
    pub static ref QUARTZ_BLOCK_BOTTOM: SingleTextureMaterial = block("quartz_block_bottom",
            (QUARTZ.refined_block)(&QUARTZ));
    pub static ref QUARTZ_BLOCK_SIDE: SingleTextureMaterial = block("quartz_block_side",
            (QUARTZ.raw_block)(&QUARTZ));
}

macro_rules! stone {
    ($name:ident = $background:expr, $($layers:expr),*) => {
        #[allow(unused_macros)]
        macro_rules! extreme_highlight {
            () => {
                crate::image_tasks::color::ComparableColor::STONE_EXTREME_HIGHLIGHT
            }
        }
        #[allow(unused_macros)]
        macro_rules! extreme_shadow {
            () => {
                crate::image_tasks::color::ComparableColor::STONE_EXTREME_SHADOW
            }
        }
        crate::block_with_colors!($name =
            crate::materials::block::pickaxe::ore_base::STONE.color(),
            crate::materials::block::pickaxe::ore_base::STONE.shadow(),
            crate::materials::block::pickaxe::ore_base::STONE.highlight(),

            $background,
            $($layers),*
        );
    }
}

stone!(SMOOTH_STONE =
    color!(),
    paint_svg_task("borderSolid", extreme_shadow!())
);

lazy_static! {
    static ref COBBLESTONE_BASE: ToPixmapTaskSpec = stack_on!(
        ComparableColor::STONE_HIGHLIGHT,
        paint_svg_task("checksLarge", ComparableColor::STONE_SHADOW),
        paint_svg_task("checksSmall", ComparableColor::STONE)
    );
}

stone!(COBBLESTONE =
    ComparableColor::TRANSPARENT,
    COBBLESTONE_BASE.to_owned(),
    paint_svg_task("borderSolid", extreme_highlight!()),
    paint_svg_task("borderShortDashes", extreme_shadow!())
);

single_texture_block!(MOSSY_COBBLESTONE =
    ComparableColor::TRANSPARENT,
    COBBLESTONE_BASE.to_owned(),
    paint_svg_task("dots3", MOSS_BLOCK.color()),
    paint_svg_task("dots2", MOSS_BLOCK.shadow()),
    paint_svg_task("dots1", MOSS_BLOCK.color()),
    paint_svg_task("borderSolid", MOSS_BLOCK.highlight()),
    paint_svg_task("borderShortDashes", MOSS_BLOCK.shadow())
);

single_texture_block!(COBBLED_DEEPSLATE =
    DEEPSLATE.shadow(),
    paint_svg_task("checksLarge", DEEPSLATE.highlight()),
    paint_svg_task("checksSmall", DEEPSLATE.color())
);

macro_rules! sandstone {
    ($name:ident = $background:expr, $($layers:expr),*) => {
        crate::block_with_colors!($name =
            crate::materials::block::shovel::simple_soft_earth::SAND.color(),
            crate::materials::block::shovel::simple_soft_earth::SAND.shadow(),
            crate::materials::block::shovel::simple_soft_earth::SAND.highlight(),

            $background,
            $($layers),*
        );
    }
}

sandstone!(SANDSTONE_BOTTOM =
    color!(),
    paint_svg_task("checksLarge", shadow!()),
    paint_svg_task("borderLongDashes", highlight!())
);

sandstone!(SANDSTONE_TOP =
    color!(),
    paint_svg_task("borderSolid", highlight!()),
    paint_svg_task("checksLarge", shadow!())
);

sandstone!(SANDSTONE =
    color!(),
    paint_stack!(shadow!(), "topPart", "borderSolid"),
    paint_stack!(highlight!(), "topStripeThick", "borderShortDashes")
);

sandstone!(CUT_SANDSTONE =
    color!(),
    paint_svg_task("checksLargeOutline", highlight!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderSolidTopLeft", highlight!()),
    paint_svg_task("borderLongDashes", color!())
);

sandstone!(CHISELED_SANDSTONE =
    ComparableColor::TRANSPARENT,
    CUT_SANDSTONE.material.texture.to_owned(),
    paint_svg_task("creeperFaceSmall", shadow!())
);

lazy_static!{
    static ref RED_SANDSTONE_BASE: ToPixmapTaskSpec = stack_on!(
        RED_SAND.color(),
        paint_svg_task("checksLarge", RED_SAND.highlight()),
        paint_svg_task("checksLargeOutline", RED_SAND.shadow())
    );
}

macro_rules! red_sandstone {
    ($name:ident = $background:expr, $($layers:expr),*) => {
        crate::block_with_colors!($name =
            crate::materials::block::shovel::simple_soft_earth::RED_SAND.color(),
            crate::materials::block::shovel::simple_soft_earth::RED_SAND.shadow(),
            crate::materials::block::shovel::simple_soft_earth::RED_SAND.highlight(),

            $background,
            $($layers),*
        );
    }
}

red_sandstone!(RED_SANDSTONE_BOTTOM =
    ComparableColor::TRANSPARENT,
    RED_SANDSTONE_BASE.to_owned(),
    paint_svg_task("borderLongDashes", color!())
);

red_sandstone!(RED_SANDSTONE_TOP =
    ComparableColor::TRANSPARENT,
    RED_SANDSTONE_BASE.to_owned(),
    paint_svg_task("borderSolidThick", shadow!()),
    paint_svg_task("borderSolid", highlight!()),
    paint_svg_task("borderLongDashes", color!())
);

red_sandstone!(CUT_RED_SANDSTONE =
    color!(),
    paint_svg_task("checksLarge", highlight!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderSolidTopLeft", highlight!()),
    paint_svg_task("borderLongDashes", color!())
);

red_sandstone!(CHISELED_RED_SANDSTONE =
    ComparableColor::TRANSPARENT,
    CUT_RED_SANDSTONE.material.texture.to_owned(),
    paint_svg_task("witherSymbol", shadow!())
);

red_sandstone!(RED_SANDSTONE =
    color!(),
    paint_svg_task("topPart", shadow!()),
    paint_stack!(highlight!(), "topStripeThick", "borderSolid"),
    paint_svg_task("borderShortDashes", shadow!())
);

macro_rules! basalt {
    ($name:ident = $background:expr, $($layers:expr),*) => {
        crate::block_with_colors!($name =
            ComparableColor::STONE_EXTREME_SHADOW,
            c(0x003939),
            ComparableColor::STONE_SHADOW,

            $background,
            $($layers),*
        );
    }
}

basalt!(BASALT_TOP =
    color!(),
    paint_svg_task("borderSolid", highlight!()),
    paint_svg_task("borderLongDashes", shadow!()),
    paint_svg_task("bigRingsBottomLeftTopRight", highlight!()),
    paint_svg_task("bigRingsTopLeftBottomRight", shadow!()),
    paint_stack!(color!(), "strokeBottomLeftTopRight",
        "strokeTopLeftBottomRight", "bigDiamond")
);

basalt!(BASALT_SIDE =
    shadow!(),
    paint_svg_task("stripesVerticalThick", color!()),
    paint_svg_task("borderLongDashes", highlight!())
);

basalt!(POLISHED_BASALT_TOP =
    color!(),
    paint_svg_task("ringsCentralBullseye", shadow!()),
    paint_svg_task("rings", highlight!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_stack!(highlight!(), "borderSolidTopLeft", "cross"),
    paint_svg_task("crossDotted", shadow!())
);

basalt!(POLISHED_BASALT_SIDE =
    color!(),
    paint_svg_task("stripesVerticalThick", highlight!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderSolidTopLeft", highlight!())
);

block_with_colors!(GLOWSTONE = c(0xcc8654), c(0x6f4522), c(0xffda74),
    color!(),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("checksSmall", highlight!()),
    paint_svg_task("lampOn", ComparableColor::WHITE)
);

block_with_colors!(END_STONE = c(0xdeffa4),c(0xc5be8b),c(0xffffb4),
    color!(),
    paint_stack!(shadow!(), "checksLargeOutline", "checksQuarterCircles"),
    paint_svg_task("bigRingsTopLeftBottomRight", highlight!())
);

block_with_colors!(END_STONE_BRICKS = c(0xdeffa4),c(0xc5be8b),c(0xffffb4),
    highlight!(),
    paint_svg_task("checksSmall", color!()),
    paint_svg_task("bricksSmall", shadow!()),
    paint_svg_task("borderShortDashes", highlight!())
);

quartz!(QUARTZ_PILLAR =
    shadow!(),
    paint_svg_task("tntSticksSide", color!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderDotted", highlight!())
);

quartz!(QUARTZ_PILLAR_TOP =
    color!(),
    paint_svg_task("rings", highlight!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderDotted", highlight!())
);

single_texture_block!(MUD_BRICKS =
    PACKED_MUD.color(),
    paint_svg_task("bricks", MUD.shadow()),
    paint_svg_task("strokeTopLeftBottomRight2", PACKED_MUD.highlight()),
    paint_svg_task("strokeBottomLeftTopRight2", PACKED_MUD.shadow()),
    paint_svg_task("borderDotted", MUD.highlight())
);

stone!(STONE_BRICKS =
    color!(),
    paint_svg_task("checksLarge", highlight!()),
    paint_svg_task("bricks", extreme_shadow!()),
    paint_svg_task("borderShortDashes", shadow!())
);

stone!(CRACKED_STONE_BRICKS =
    ComparableColor::TRANSPARENT,
    STONE_BRICKS.material.texture.to_owned(),
    paint_svg_task("streaks", extreme_shadow!())
);

stone!(MOSSY_STONE_BRICKS =
    color!(),
    paint_svg_task("checksLarge", highlight!()),
    paint_svg_task("bricks", extreme_shadow!()),
    paint_svg_task("dots2", MOSS_BLOCK.highlight()),
    paint_svg_task("dots1", MOSS_BLOCK.color()),
    paint_svg_task("borderSolid", MOSS_BLOCK.highlight()),
    paint_stack!(MOSS_BLOCK.shadow(), "borderShortDashes", "dots3")
);

stone!(CHISELED_STONE_BRICKS =
    color!(),
    paint_stack!(extreme_shadow!(),"rings2","borderSolid"),
    paint_stack!(extreme_highlight!(),"ringsCentralBullseye","borderSolidTopLeft")
);

block_with_colors!(TERRACOTTA = c(0x945b43), c(0x885533), c(0x9b6045),
    color!(),
    paint_svg_task("bigDotsTopLeftBottomRight", highlight!()),
    paint_stack!(shadow!(),
        "bigRingsTopLeftBottomRight",
        "bigDotsBottomLeftTopRight"
    ),
    paint_stack!(highlight!(),
        "bigRingsBottomLeftTopRight",
        "borderRoundDots"
    )
);

single_texture_block!(BRICKS =
    c(0x945b43),
    paint_svg_task("bricksSmall", c(0xa2867d)),
    paint_svg_task("borderDotted", c(0xa2867d) * 0.5)
);

quartz!(QUARTZ_BRICKS =
    color!(),
    paint_svg_task("streaks", highlight!()),
    paint_svg_task("bricks", shadow!()),
    paint_svg_task("borderDotted", highlight!())
);

group!(DEEPSLATE_VARIANTS = DEEPSLATE_BRICKS, DEEPSLATE_TOP, COBBLED_DEEPSLATE);
group!(QUARTZ_VARIANTS = QUARTZ_BLOCK_TOP, QUARTZ_BLOCK_BOTTOM, QUARTZ_BLOCK_SIDE,
        QUARTZ_PILLAR, QUARTZ_PILLAR_TOP, QUARTZ_BRICKS);
group!(STONE_VARIANTS = SMOOTH_STONE, STONE_BRICKS, CRACKED_STONE_BRICKS, MOSSY_STONE_BRICKS,
        CHISELED_STONE_BRICKS);
group!(COBBLESTONE_VARIANTS = COBBLESTONE, MOSSY_COBBLESTONE);
group!(SANDSTONE_VARIANTS = SANDSTONE_BOTTOM, SANDSTONE_TOP, SANDSTONE, CUT_SANDSTONE,
        CHISELED_SANDSTONE);
group!(RED_SANDSTONE_VARIANTS = RED_SANDSTONE_BOTTOM, RED_SANDSTONE_TOP, RED_SANDSTONE,
        CUT_RED_SANDSTONE, CHISELED_RED_SANDSTONE);
group!(BASALT_VARIANTS = BASALT_TOP, BASALT_SIDE, POLISHED_BASALT_TOP, POLISHED_BASALT_SIDE);
group!(END_STONE_VARIANTS = END_STONE, END_STONE_BRICKS);
group!(TERRACOTTA_VARIANTS = TERRACOTTA);
group!(MISC_BRICKS = MUD_BRICKS, BRICKS);
group!(SIMPLE_PICKAXE_BLOCKS = DEEPSLATE_VARIANTS, QUARTZ_VARIANTS, STONE_VARIANTS,
        COBBLESTONE_VARIANTS, SANDSTONE_VARIANTS, RED_SANDSTONE_VARIANTS, BASALT_VARIANTS,
        GLOWSTONE, END_STONE_VARIANTS, MISC_BRICKS, TERRACOTTA_VARIANTS);