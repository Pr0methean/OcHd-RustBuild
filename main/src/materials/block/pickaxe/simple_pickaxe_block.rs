use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task, ToPixmapTaskSpec};
use crate::materials::block::pickaxe::ore_base::DEEPSLATE;
use crate::{block_with_colors, group, make_tricolor_block_macro, paint_stack, single_texture_block, stack_on};
use crate::materials::block::pickaxe::ore::{QUARTZ, COPPER};
use crate::materials::block::pickaxe::polishable::BLACKSTONE;
use crate::materials::block::shovel::simple_soft_earth::{MOSS_BLOCK, SAND, RED_SAND, MUD, PACKED_MUD};
use crate::texture_base::material::{SingleTextureMaterial, TricolorMaterial};
use crate::texture_base::material::block;

single_texture_block!(DEEPSLATE_BRICKS = ComparableColor::TRANSPARENT,
    DEEPSLATE.material.texture(),
    paint_svg_task("bricksSmall", DEEPSLATE.shadow()),
    paint_svg_task("borderDotted", DEEPSLATE.highlight()),
    paint_svg_task("borderDottedBottomRight", DEEPSLATE.shadow())
);

single_texture_block!(DEEPSLATE_TOP = ComparableColor::TRANSPARENT,
        DEEPSLATE.material.texture(),
        paint_svg_task("cross", DEEPSLATE.shadow()),
        paint_svg_task("borderSolid", DEEPSLATE.highlight())
);

make_tricolor_block_macro!(quartz, QUARTZ.color(), QUARTZ.shadow(), QUARTZ.highlight());

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

make_tricolor_block_macro!(sandstone, SAND.color(), SAND.shadow(), SAND.highlight());

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
    CUT_SANDSTONE.material.texture(),
    paint_svg_task("creeperFaceSmall", shadow!())
);

lazy_static!{
    static ref RED_SANDSTONE_BASE: ToPixmapTaskSpec = stack_on!(
        RED_SAND.color(),
        paint_svg_task("checksLarge", RED_SAND.highlight()),
        paint_svg_task("checksLargeOutline", RED_SAND.shadow())
    );
}

make_tricolor_block_macro!(red_sandstone, RED_SAND.color(), RED_SAND.shadow(), RED_SAND.highlight());

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
    CUT_RED_SANDSTONE.material.texture(),
    paint_svg_task("witherSymbol", shadow!())
);

red_sandstone!(RED_SANDSTONE =
    color!(),
    paint_svg_task("topPart", shadow!()),
    paint_stack!(highlight!(), "topStripeThick", "borderSolid"),
    paint_svg_task("borderShortDashes", shadow!())
);

make_tricolor_block_macro!(basalt, ComparableColor::STONE_EXTREME_SHADOW, ComparableColor::BLACK,
    ComparableColor::STONE_SHADOW);

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
    STONE_BRICKS.material.texture(),
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

block_with_colors!(POLISHED_BLACKSTONE_BRICKS =
    BLACKSTONE.color(),
    BLACKSTONE.shadow(),
    BLACKSTONE.highlight(),

    color!(),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderSolidTopLeft", highlight!()),
    paint_svg_task("bricksSmall", shadow!())
);

block_with_colors!(NETHER_BRICKS = c(0x302020), ComparableColor::BLACK, c(0x442929),

    color!(),
    paint_stack!(highlight!(), "bricksSmall", "borderDotted"),
    paint_svg_task("borderDottedBottomRight", shadow!())
);

block_with_colors!(RED_NETHER_BRICKS = c(0x500000),c(0x2e0000),c(0x730000),

    color!(),
    paint_svg_task("bricksSmall", shadow!()),
    paint_svg_task("borderDotted", highlight!()),
    paint_svg_task("borderDottedBottomRight", shadow!())
);

make_tricolor_block_macro!(amethyst, c(0xc890ff),c(0x7a5bb5),c(0xffcbff));

amethyst!(AMETHYST_BLOCK =
    shadow!(),
    paint_svg_task("triangles1", highlight!()),
    paint_svg_task("triangles2", color!())
);

single_texture_block!(BUDDING_AMETHYST = ComparableColor::TRANSPARENT,
    AMETHYST_BLOCK.material.texture(),
    paint_svg_task("buddingAmethystCenter", c(0x462b7d))
);

amethyst!(AMETHYST_CLUSTER = ComparableColor::TRANSPARENT,
    paint_svg_task("amethystCluster1", highlight!()),
    paint_svg_task("amethystCluster2", color!())
);

amethyst!(LARGE_AMETHYST_BUD = ComparableColor::TRANSPARENT,
    paint_svg_task("largeAmethystBud1", color!()),
    paint_svg_task("largeAmethystBud2", shadow!()),
    paint_svg_task("largeAmethystBud3", highlight!())
);

amethyst!(MEDIUM_AMETHYST_BUD = ComparableColor::TRANSPARENT,
    paint_svg_task("mediumAmethystBud1", color!()),
    paint_svg_task("mediumAmethystBud2", shadow!()),
    paint_svg_task("largeAmethystBud3", highlight!())
);

amethyst!(SMALL_AMETHYST_BUD = ComparableColor::TRANSPARENT,
    paint_svg_task("smallAmethystBud1", color!()),
    paint_svg_task("smallAmethystBud2", shadow!())
);

make_tricolor_block_macro!(purpur, c(0xac7bac), c(0x906590), c(0xc7a8c7));

purpur!(PURPUR_BLOCK = color!(),
    paint_svg_task("bigCircle", highlight!() * 0.25),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("2x2TopLeft", highlight!())
);

purpur!(PURPUR_PILLAR = color!(),
    paint_svg_task("bigCircle", highlight!() * 0.25),
    paint_stack!(shadow!(), "borderSolid", "stripesVerticalThick"),
    paint_svg_task("borderSolidTopLeft", highlight!())
);

purpur!(PURPUR_PILLAR_TOP = highlight!(),
    paint_svg_task("bigCircle", color!() * 0.75),
    paint_svg_task("borderSolidThick", color!()),
    paint_svg_task("borderSolid", shadow!()),
    paint_svg_task("borderSolidTopLeft", highlight!())
);

block_with_colors!(CUT_COPPER = COPPER.color(), COPPER.shadow(), COPPER.highlight(),
    color!(),
    paint_svg_task("streaks", highlight!()),
    paint_stack!(shadow!(), "borderSolid", "cross"),
    paint_svg_task("2x2TopLeft", highlight!())
);

block_with_colors!(BLACK_GLAZED_TERRACOTTA = c(0x2f2f2f), ComparableColor::BLACK, c(0x992222),
    shadow!(),
    paint_svg_task("asymmetricalQuarterCircles", color!()),
    paint_stack!(highlight!(), "bigRingsBottomLeftTopRight", "cornerRoundTopLeft")
);

block_with_colors!(BLUE_GLAZED_TERRACOTTA = c(0x4040aa), c(0x2d2d8f), c(0x4577d3),

    shadow!(),
    paint_svg_task("checksQuarterCircles", color!()),
    from_svg_task("bigDotsTopLeftBottomRight"),
    paint_svg_task("bigRingsTopLeftBottomRight", color!()),
    paint_stack!(highlight!(), "checksLargeOutline", "cornerRingTopLeft")
);

block_with_colors!(PURPLE_GLAZED_TERRACOTTA = c(0x8900b8), c(0x5f0093), c(0xa254e0),
    color!(),
    paint_svg_task("borderSolidThick", shadow!()),
    from_svg_task("asymmetricalQuarterCircles"),
    from_svg_task("strokeTopLeftBottomRightThick"),
    paint_stack!(highlight!(), "cornerRingTopLeft", "strokeTopLeftBottomRight2")
);

block_with_colors!(BROWN_GLAZED_TERRACOTTA = c(0x8c5a35), c(0x007788), c(0xcd917c),
    color!(),
    paint_svg_task("cornersRound", shadow!()),
    paint_stack!(highlight!(), "ray", "cornerCrosshairs")
);

block_with_colors!(GRAY_GLAZED_TERRACOTTA = c(0x737373), c(0x333333), c(0x999999),
    color!(),
    paint_svg_task("asymmetricalQuarterCircles", shadow!()),
    paint_stack!(highlight!(), "cornerCrosshairs", "cornerRoundTopLeft",
        "comparator", "repeaterSideInputs")
);

block_with_colors!(GREEN_GLAZED_TERRACOTTA = c(0x729b24), c(0x495b24), c(0xd6d6d6),
    color!(),
    paint_svg_task("railCorner", shadow!()),
    paint_stack!(highlight!(), "strokeTopLeftBottomRight", "cornerRingTopLeft")
);

block_with_colors!(RED_GLAZED_TERRACOTTA = c(0xb82b2b), c(0x8e2020), c(0xce4848),
    color!(),
    paint_svg_task("cornersRound", highlight!()),
    paint_svg_task("topPart", color!()),
    paint_svg_task("ringsSpiral", highlight!()),
    paint_svg_task("cornerRoundTopLeft", shadow!())
);
block_with_colors!(PINK_GLAZED_TERRACOTTA = c(0xff8baa), c(0xd6658f), c(0xffb5cb),
    shadow!(),
    paint_svg_task("strokeTopLeftBottomRight4", highlight!()),
    paint_stack!(shadow!(), "cornersTri", "fishTail", "fishFins"),
    paint_svg_task("fishBody", color!()),
    paint_svg_task("fishStripe", highlight!())
);

block_with_colors!(MAGENTA_GLAZED_TERRACOTTA = c(0xdc68dc), c(0xae33ae), c(0xffa5bf),
    shadow!(),
    paint_svg_task("stripesVerticalThick", color!()),
    paint_svg_task("arrowUpExpanded", highlight!()),
    paint_svg_task("arrowUp", shadow!())
);
block_with_colors!(CYAN_GLAZED_TERRACOTTA = c(0x828282), c(0x333333), c(0x009c9c),
    color!(),
    paint_svg_task("strokeBottomLeftTopRight4", highlight!()),
    paint_svg_task("strokeBottomLeftTopRight2", shadow!()),
    paint_svg_task("craftingGridSquare", highlight!()),
    paint_svg_task("creeperFaceSmall", shadow!())
);
block_with_colors!(LIGHT_BLUE_GLAZED_TERRACOTTA = c(0x2389c7), c(0x2d2d8f), c(0x57bddf),
    // TODO: maybe add the parallelogram-shaped pieces and white corners?
    shadow!(),
    paint_svg_task("bottomHalf", ComparableColor::WHITE),
    paint_svg_task("checksLarge", highlight!()),
    paint_svg_task("emeraldTopLeft", ComparableColor::WHITE),
    paint_svg_task("emeraldBottomRight", color!())
);
block_with_colors!(LIME_GLAZED_TERRACOTTA = c(0x8bd922), c(0x5ea900), c(0xffffc4),
    color!(),
    paint_svg_task("borderSolidTopLeft", shadow!()),
    paint_svg_task("strokeTopLeftBottomRight", shadow!()),
    paint_svg_task("railCornerInverted", highlight!())
);
block_with_colors!(LIGHT_GRAY_GLAZED_TERRACOTTA = ComparableColor::STONE_SHADOW, c(0x009c9c), ComparableColor::LIGHTEST_GRAY,
    color!(),
    from_svg_task("strokeBottomLeftTopRightThick"),
    paint_svg_task("strokeBottomLeftTopRight2", shadow!()),
    paint_svg_task("bigQuarterCircleTopRightFilled", highlight!()),
    paint_svg_task("cornerBullseyeBottomLeft", shadow!())
);
block_with_colors!(YELLOW_GLAZED_TERRACOTTA = c(0xffb000), c(0xa4764c), c(0xffff9d),
    color!(),
    paint_svg_task("cornersRound", shadow!()),
    paint_stack!(highlight!(), "sunflowerPetals", "cross", "bigDotsTopLeftBottomRight",
        "cornerRoundTopLeft")
);
block_with_colors!(ORANGE_GLAZED_TERRACOTTA = c(0xff8000), c(0x009c9c), c(0x00c6c6),
    highlight!(),
    paint_stack!(color!(), "bigDotsBottomLeftTopRight", "cornerTwoLobesSolidTopLeft"),
    paint_svg_task("dots0", shadow!()),
    paint_svg_task("strokeTopLeftBottomRight", ComparableColor::WHITE),
    paint_stack!(shadow!(), "cornerTwoLobesBorderTopLeft", "cornerRoundBottomRight"),
    paint_svg_task("cornerRingBottomRight", color!())
);
block_with_colors!(WHITE_GLAZED_TERRACOTTA = c(0x3ab3da), c(0x2389c7), c(0xffd83d),
    ComparableColor::WHITE,
    paint_stack!(highlight!(), "borderSolidTopLeft", "cornerRoundTopLeft"),
    paint_svg_task("strokeBottomLeftTopRightThick", shadow!()),
    paint_svg_task("strokeBottomLeftTopRight", highlight!()),
    paint_svg_task("cornerRoundBottomRight", color!()),
    paint_svg_task("cornerBullseyeBottomRight", color!())
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
group!(TERRACOTTA_VARIANTS = TERRACOTTA, BLACK_GLAZED_TERRACOTTA, RED_GLAZED_TERRACOTTA,
        GREEN_GLAZED_TERRACOTTA, BROWN_GLAZED_TERRACOTTA, BLUE_GLAZED_TERRACOTTA,
        PURPLE_GLAZED_TERRACOTTA, CYAN_GLAZED_TERRACOTTA, GRAY_GLAZED_TERRACOTTA,
        LIGHT_GRAY_GLAZED_TERRACOTTA, PINK_GLAZED_TERRACOTTA,
        LIME_GLAZED_TERRACOTTA, LIGHT_BLUE_GLAZED_TERRACOTTA, MAGENTA_GLAZED_TERRACOTTA,
        YELLOW_GLAZED_TERRACOTTA, ORANGE_GLAZED_TERRACOTTA, WHITE_GLAZED_TERRACOTTA);
group!(MISC_BRICKS = MUD_BRICKS, BRICKS, POLISHED_BLACKSTONE_BRICKS, NETHER_BRICKS,
    RED_NETHER_BRICKS);
group!(AMETHYST = AMETHYST_BLOCK, BUDDING_AMETHYST, AMETHYST_CLUSTER,
    LARGE_AMETHYST_BUD, MEDIUM_AMETHYST_BUD, SMALL_AMETHYST_BUD);
group!(PURPUR = PURPUR_BLOCK, PURPUR_PILLAR, PURPUR_PILLAR_TOP);
group!(SIMPLE_PICKAXE_BLOCKS = DEEPSLATE_VARIANTS, QUARTZ_VARIANTS, STONE_VARIANTS,
        COBBLESTONE_VARIANTS, SANDSTONE_VARIANTS, RED_SANDSTONE_VARIANTS, BASALT_VARIANTS,
        GLOWSTONE, END_STONE_VARIANTS, MISC_BRICKS, TERRACOTTA_VARIANTS, AMETHYST, PURPUR,
        CUT_COPPER);