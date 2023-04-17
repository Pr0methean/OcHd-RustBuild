use lazy_static::lazy_static;


use crate::{group, paint_stack, stack, stack_on};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, out_task, paint_svg_task, SinkTaskSpec, ToPixmapTaskSpec};

use crate::texture_base::material::{Material, TricolorMaterial};

pub struct Wood {
    pub color: ComparableColor,
    pub highlight: ComparableColor,
    pub shadow: ComparableColor,
    bark_color: ComparableColor,
    bark_highlight: ComparableColor,
    bark_shadow: ComparableColor,
    leaves_color: ComparableColor,
    leaves_highlight: ComparableColor,
    leaves_shadow: ComparableColor,
    log_synonym: &'static str,
    leaves_synonym: &'static str,
    sapling_synonym: &'static str,
    name: &'static str,
    planks_highlight_strength: f32,
    planks_shadow_strength: f32,
    bark: Box<dyn (Fn(&Self) -> ToPixmapTaskSpec) + Sync + Send>,
    stripped_log_side: Box<dyn (Fn(&Self) -> ToPixmapTaskSpec) + Sync + Send>,
    log_top: Box<dyn (Fn(&Self, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
    stripped_log_top: Box<dyn (Fn(&Self) -> ToPixmapTaskSpec) + Sync + Send>,
    trapdoor: Box<dyn (Fn(&Self, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
    door_top: Box<dyn (Fn(&Self, ToPixmapTaskSpec, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
    door_bottom: Box<dyn (Fn(&Self, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
    leaves: Box<dyn (Fn(&Self) -> ToPixmapTaskSpec) + Sync + Send>,
    sapling: Box<dyn (Fn(&Self) -> ToPixmapTaskSpec) + Sync + Send>,
    door_common_layers: Box<dyn (Fn(&Self) -> ToPixmapTaskSpec) + Sync + Send>,
}

impl Wood {
    pub fn planks(&self) -> ToPixmapTaskSpec {
        stack_on!(self.color,
                paint_svg_task("waves2", self.highlight * self.planks_highlight_strength),
                paint_svg_task("waves", self.shadow * self.planks_shadow_strength),
                paint_svg_task("planksTopBorder", self.shadow),
                paint_svg_task("borderShortDashes", self.highlight)
        )
    }

    pub fn overworld_bark(&self) -> ToPixmapTaskSpec {
        stack_on!(self.bark_color,
            paint_svg_task("borderSolid", self.bark_shadow),
            paint_svg_task("borderDotted", self.bark_highlight),
            paint_svg_task("zigzagSolid", self.bark_shadow),
            paint_svg_task("zigzagSolid2", self.bark_highlight)
        )
    }

    pub fn fungus_bark(&self) -> ToPixmapTaskSpec {
        stack_on!(self.bark_color,
            paint_stack!(self.bark_shadow, "borderSolid", "waves2"),
            paint_svg_task("waves", self.bark_highlight)
        )
    }

    pub fn overworld_stripped_log_side(&self) -> ToPixmapTaskSpec {
        stack_on!(self.color,
            paint_svg_task("borderSolid", self.shadow),
            paint_svg_task("borderShortDashes", self.highlight)
        )
    }

    pub fn fungus_stripped_log_side(&self) -> ToPixmapTaskSpec {
        stack_on!(self.color,
            paint_svg_task("borderSolid", self.shadow),
            paint_svg_task("borderDotted", self.highlight))
    }

    pub fn overworld_stripped_log_top(&self) -> ToPixmapTaskSpec {
        stack_on!(
            self.color,
            paint_svg_task("ringsCentralBullseye", self.highlight),
            paint_svg_task("rings", self.shadow)
        )
    }

    pub fn fungus_stripped_log_top(&self) -> ToPixmapTaskSpec {
        stack_on!(
            self.color,
            stack!(
                paint_svg_task("ringsCentralBullseye", self.shadow),
                paint_svg_task("rings2", self.highlight)
            )
        )
    }

    pub fn overworld_log_top(&self, stripped_log_top: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
        stack!(
            stripped_log_top,
            paint_svg_task("borderSolid", self.bark_color),
            paint_svg_task("borderDotted", self.bark_shadow)
        )
    }

    pub fn fungus_log_top(&self, _stripped_log_top: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
        stack_on!(self.color,
            stack!(
                paint_svg_task("ringsCentralBullseye", self.shadow),
                paint_svg_task("rings2", self.highlight)
            ),
            paint_svg_task("borderSolid", self.bark_color),
            paint_svg_task("borderShortDashes", self.bark_shadow)
        )
    }

    pub fn default_door_top(&self, door_bottom: ToPixmapTaskSpec, _: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
        stack!(
            door_bottom,
            from_svg_task("doorKnob")
        )
    }
}

impl TricolorMaterial for Wood {
    fn color(&self) -> ComparableColor {
        self.color
    }

    fn shadow(&self) -> ComparableColor {
        self.shadow
    }

    fn highlight(&self) -> ComparableColor {
        self.highlight
    }
}

pub fn empty_task() -> Box<dyn (Fn(&Wood) -> ToPixmapTaskSpec) + Sync + Send> {
    Box::new(/*door_common_layers*/ |_wood| ToPixmapTaskSpec::None {})
}

pub fn overworld_wood(name: &'static str, color: ComparableColor,
                      highlight: ComparableColor, shadow: ComparableColor,
                      bark_color: ComparableColor, bark_highlight: ComparableColor,
                      bark_shadow: ComparableColor,
                      door_common_layers: Box<dyn (Fn(&Wood) -> ToPixmapTaskSpec) + Sync + Send>,
                      trapdoor: Box<dyn (Fn(&Wood, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
                      door_bottom: Box<dyn (Fn(&Wood, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
                      door_top: Box<dyn (Fn(&Wood, ToPixmapTaskSpec, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
                      leaves: Box<dyn (Fn(&Wood) -> ToPixmapTaskSpec) + Sync + Send>,
                      sapling: Box<dyn (Fn(&Wood) -> ToPixmapTaskSpec) + Sync + Send>) -> Wood {
    Wood {
        color,
        highlight,
        shadow,
        bark_color,
        bark_highlight,
        bark_shadow,
        leaves_color: ComparableColor::MEDIUM_BIOME_COLORABLE,
        leaves_highlight: ComparableColor::LIGHT_BIOME_COLORABLE,
        leaves_shadow: ComparableColor::DARK_BIOME_COLORABLE,
        log_synonym: "log",
        leaves_synonym: "leaves",
        sapling_synonym: "sapling",
        name,
        planks_highlight_strength: 0.5,
        planks_shadow_strength: 0.5,
        bark: Box::new(Wood::overworld_bark),
        stripped_log_side: Box::new(Wood::overworld_stripped_log_side),
        log_top: Box::new(Wood::overworld_log_top),
        stripped_log_top: Box::new(Wood::overworld_stripped_log_top),
        trapdoor,
        door_top,
        door_bottom,
        leaves,
        sapling,
        door_common_layers,
    }
}

pub fn nether_fungus(name: &'static str, color: ComparableColor,
                     highlight: ComparableColor, shadow: ComparableColor,
                     bark_color: ComparableColor, bark_highlight: ComparableColor,
                     bark_shadow: ComparableColor, leaves_color: ComparableColor,
                     leaves_highlight: ComparableColor, leaves_shadow: ComparableColor,
                     trapdoor: Box<dyn (Fn(&Wood, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
                     door_bottom: Box<dyn (Fn(&Wood, ToPixmapTaskSpec) -> ToPixmapTaskSpec) + Sync + Send>,
                     leaves: Box<dyn (Fn(&Wood) -> ToPixmapTaskSpec) + Sync + Send>,
                     sapling: Box<dyn (Fn(&Wood) -> ToPixmapTaskSpec) + Sync + Send>) -> Wood {
    return Wood {
        color,
        highlight,
        shadow,
        bark_color,
        bark_highlight,
        bark_shadow,
        leaves_color,
        leaves_highlight,
        leaves_shadow,
        log_synonym: "stem",
        leaves_synonym: "wart_block",
        sapling_synonym: "fungus",
        planks_highlight_strength: 0.75,
        planks_shadow_strength: 0.75,
        name,
        bark: Box::new(Wood::fungus_bark),
        stripped_log_side: Box::new(Wood::fungus_stripped_log_side),
        stripped_log_top: Box::new(Wood::fungus_stripped_log_top),
        log_top: Box::new(Wood::fungus_log_top),
        door_common_layers: empty_task(),
        trapdoor,
        door_bottom,
        door_top: Box::new(Wood::default_door_top),
        leaves,
        sapling,
    }
 }

lazy_static! {pub static ref ACACIA: Wood = overworld_wood(
    "acacia",
    c(0xad5d32),
    c(0xc26d3f),
    c(0x915431),
    c(0x70583B),
    c(0x898977),
    c(0x4a4a39),
    Box::new(/*door_common_layers*/ |_wood| stack!(
        paint_svg_task("borderSolidThick", ACACIA.color),
        paint_svg_task("borderSolid", ACACIA.highlight),
        paint_svg_task("bigDiamond", ACACIA.shadow)
    )),
    Box::new(/*trapdoor*/ |_wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_SHADOW),
        paint_svg_task("trapdoorHinges", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(/*door_bottom*/ |_wood, door_common_layers| stack!(
        door_common_layers,
        paint_stack!(ACACIA.color, "strokeBottomLeftTopRight", "strokeTopLeftBottomRight"),
        paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW),
        paint_svg_task("doorHinges", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(Wood::default_door_top),
    Box::new(/*leaves*/ |_wood| stack!(
        paint_svg_task("leaves1", ACACIA.leaves_shadow),
        paint_svg_task("leaves1a", ACACIA.leaves_highlight)
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("saplingStem", ACACIA.bark_color),
        paint_svg_task("acaciaSapling", c(0x6c9e38)),
        paint_svg_task("acaciaSapling2", c(0xc9d7a5))
    )),
);}
lazy_static! {pub static ref BIRCH: Wood = {
    let mut base = overworld_wood(
        "birch",
        c(0xc8b77a),
        c(0xD7C187),
        c(0x915431),
        c(0xeeffea),
        ComparableColor::WHITE,
        c(0x5f5f4f),
        Box::new(/*door_common_layers*/ |_wood| stack_on!(BIRCH.bark_highlight,
            paint_svg_task("borderSolidExtraThick", BIRCH.color),
            paint_svg_task("craftingGridSquare", BIRCH.highlight),
            paint_svg_task("craftingGridSpaces", BIRCH.bark_highlight),
            paint_svg_task("borderSolid", BIRCH.shadow)
        )),
        Box::new(/*trapdoor*/ |_wood, door_common_layers| stack!(
            door_common_layers,
            paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_SHADOW)
        )),
        Box::new(/*door_bottom*/ |_wood, _door_common_layers| stack_on!(BIRCH.highlight,
                paint_svg_task("borderSolidExtraThick", BIRCH.color),
                paint_svg_task("craftingGridSquare", BIRCH.shadow),
                paint_svg_task("borderSolid", BIRCH.shadow),
                paint_svg_task("craftingGridSpaces", BIRCH.highlight),
                paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW)
        )),
        Box::new(/*door_top*/ |_wood, _, door_common_layers| stack!(
            door_common_layers,
            paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW),
            from_svg_task("doorKnob")
        )),
        Box::new(/*leaves*/ |_wood| stack!(
            paint_svg_task("leaves2", BIRCH.leaves_highlight),
            paint_svg_task("leaves2a", BIRCH.leaves_shadow)
        )),
        Box::new(/*sapling*/ |_wood| stack!(
            paint_svg_task("saplingStem", BIRCH.bark_color),
            paint_svg_task("flowerStemBottomBorder", BIRCH.bark_shadow),
            paint_svg_task("saplingLeaves", c(0x6c9e38))
        )),
    );
    base.planks_highlight_strength = 1.0;
    base
};}
lazy_static! {pub static ref DARK_OAK: Wood = overworld_wood(
    "dark_oak",
    c(0x3f2d23),
    c(0x3a2400),
    c(0x4a4a39),
    c(0x483800),
    c(0x2b2000),
    c(0x624033),
    Box::new(/*door_common_layers*/ |_wood| stack_on!(DARK_OAK.color,
        paint_stack!(DARK_OAK.highlight, "borderSolid", "cross"),
        paint_svg_task("2x2TopLeft", DARK_OAK.shadow),
        paint_svg_task("borderShortDashes", DARK_OAK.color)
    )),
    Box::new(/*trapdoor*/ |_wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(/*door_bottom*/ |_wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("doorHingesBig", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(Wood::default_door_top),
    Box::new(/*leaves*/ |_wood| stack!(
        paint_svg_task("leaves3", DARK_OAK.leaves_shadow),
        paint_svg_task("leaves3a", DARK_OAK.leaves_highlight)
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("saplingStem", DARK_OAK.bark_color),
        paint_svg_task("bigCircle", c(0x005c00)),
        paint_svg_task("bigCircleTwoQuarters", c(0x57ad3f))
    )),
);}
lazy_static!{pub static ref JUNGLE: Wood = {
    let mut base = overworld_wood(
        "jungle",
        c(0x915431),
        c(0x795b4b),
        c(0x8A593A),
        c(0x483800),
        c(0x2B2000),
        c(0x8A593A),
        Box::new(/*door_common_layers*/ |_wood| stack!(
            paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW),
            paint_svg_task("doorHinges", ComparableColor::STONE)
        )),
        Box::new(/*trapdoor*/ |_wood, _door_common_layers| stack!(
            paint_svg_task("trapdoor2", JUNGLE.color),
            paint_svg_task("borderSolid", JUNGLE.shadow),
            paint_svg_task("borderShortDashes", JUNGLE.highlight),
            paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_SHADOW),
            paint_svg_task("trapdoorHinges", ComparableColor::STONE)
        )),
        Box::new(/*door_bottom*/ |_wood, door_common_layers| stack!(
            JUNGLE.planks(),
            door_common_layers
        )),
        Box::new(/*door_top*/ |_wood, _, door_common_layers| stack!(
            paint_svg_task("trapdoor2", JUNGLE.color),
            paint_svg_task("borderShortDashes", JUNGLE.highlight),
            door_common_layers,
            from_svg_task("doorKnob")
        )),
        Box::new(/*leaves*/ |_wood| stack!(
            paint_svg_task("leaves6", JUNGLE.leaves_highlight),
            paint_svg_task("leaves6a", JUNGLE.leaves_shadow)
        )),
        Box::new(/*sapling*/ |_wood| stack!(
            paint_svg_task("saplingStem", JUNGLE.bark_color),
            paint_svg_task("acaciaSapling", c(0x378020))
        )),
    );
    base.planks_shadow_strength = 1.0;
    base.planks_highlight_strength = 1.0;
    base
};}
lazy_static!{pub static ref MANGROVE: Wood = {
    let mut base = overworld_wood(
        "mangrove",
        c(0x773636),
        c(0x8A593A),
        c(0x500000),
        c(0x583838),
        c(0x624033),
        c(0x4a4a39),
    Box::new(/*door_common_layers*/ |_wood| stack!(
        paint_svg_task("rings2", MANGROVE.shadow),
        paint_svg_task("borderDotted", MANGROVE.highlight)
    )),
    Box::new(/*trapdoor*/ |_wood, door_common_layers| stack!(
        paint_svg_task("ringsHole", MANGROVE.color),
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_HIGHLIGHT),
        paint_svg_task("trapdoorHinges", ComparableColor::STONE_SHADOW)
    )),
    Box::new(/*door_bottom*/ |_wood, door_common_layers| stack_on!(MANGROVE.color,
        door_common_layers,
        paint_svg_task("doorHingesBig", ComparableColor::STONE_HIGHLIGHT),
        paint_svg_task("doorHinges", ComparableColor::STONE_SHADOW)
    )),
    Box::new(Wood::default_door_top),
    Box::new(/*leaves*/ |_wood| stack!(
        paint_svg_task("leaves5", MANGROVE.leaves_highlight),
        paint_svg_task("leaves5a", MANGROVE.leaves_color),
        paint_svg_task("leaves5b", MANGROVE.leaves_shadow)
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("mangrovePropagule", c(0x4aa54a)),
        paint_svg_task("flowerStemBottomBorder", c(0x748241))
    )),
    );
    base.sapling_synonym = "propagule";
    base
};}
lazy_static!{pub static ref SPRUCE: Wood = overworld_wood(
    "spruce",
    c(0x70583B),
    c(0x8A593A),
    c(0x624033),
    c(0x3b2700),
    c(0x624033),
    c(0x2b2000),
    empty_task(),
    Box::new(/*trapdoor*/ |_wood, _| stack_on!(SPRUCE.shadow,
        paint_svg_task("planksTopVertical", SPRUCE.color),
        paint_svg_task("borderSolidThick", SPRUCE.shadow),
        paint_svg_task("borderLongDashes", SPRUCE.highlight),
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE),
        paint_svg_task("trapdoorHinges", ComparableColor::STONE_SHADOW)
    )),
    Box::new(/*door_bottom*/ |_wood, _| stack!(
        SPRUCE.planks(),
        paint_svg_task("doorHingesBig", ComparableColor::STONE),
        paint_svg_task("doorHinges", ComparableColor::STONE_SHADOW)
    )),
    Box::new(Wood::default_door_top),
    Box::new(/*leaves*/ |_wood| stack!(
        paint_svg_task("leaves3", SPRUCE.leaves_highlight),
        paint_svg_task("leaves3b", SPRUCE.leaves_shadow)
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("saplingStem", SPRUCE.bark_highlight),
        paint_svg_task("spruceSapling", c(0x2e492e))
    )),
);}
lazy_static!{pub static ref OAK: Wood = overworld_wood(
    "oak",
    c(0xaf8f55),
    c(0xC29d62),
    c(0x70583B),
    c(0x70583B),
    c(0x987849),
    c(0x4a4a39),
    Box::new(/*door_common_layers*/ |_wood| stack!(
        stack!(
            paint_svg_task("borderSolidThick", OAK.color),
            paint_svg_task("borderSolid", OAK.highlight)
        ),
        paint_svg_task("cross", OAK.highlight),
        stack!(
            paint_svg_task("2x2TopLeft", OAK.shadow),
            paint_svg_task("borderShortDashes", OAK.color * 0.5)
        )
    )),
    Box::new(/*trapdoor*/ |_wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE),
        paint_svg_task("trapdoorHinges", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(/*door_bottom*/ |_wood, door_common_layers| stack_on!(OAK.color,
        paint_svg_task("waves", OAK.highlight),
        door_common_layers,
        stack!(
            paint_svg_task("doorHingesBig", ComparableColor::STONE),
            paint_svg_task("doorHinges", ComparableColor::STONE_HIGHLIGHT)
        )
    )),
    Box::new(/*door_top*/ |_wood, _, _| stack!(
            stack!(
                paint_svg_task("borderSolidThick", OAK.color),
                paint_svg_task("borderSolid", OAK.highlight)
            ),
            stack!(
                paint_svg_task("2x2TopLeft", OAK.shadow),
                paint_svg_task("borderShortDashes", OAK.color * 0.5)
            ),
            paint_stack!(OAK.shadow, "craftingSide", "cross"),
            from_svg_task("doorKnob"),
            stack!(
                paint_svg_task("doorHingesBig", ComparableColor::STONE),
                paint_svg_task("doorHinges", ComparableColor::STONE_HIGHLIGHT)
            )
    )),
    Box::new(/*leaves*/ |_wood| stack!(
        paint_svg_task("leaves4", OAK.leaves_shadow),
        paint_svg_task("leaves4a", OAK.leaves_highlight)
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("coalBorder", c(0x005c00)),
        paint_svg_task("saplingStem", OAK.bark_color),
        paint_svg_task("coal", c(0x57ad3f)),
        paint_svg_task("sunflowerPistil", c(0x005c00))
    ))
);}
const FUNGUS_SPOT_COLOR: ComparableColor = c(0xff6500);
lazy_static!{pub static ref CRIMSON: Wood = nether_fungus(
    "crimson",
    c(0x6a344b),
    c(0x4b2737),
    c(0x863e5a),
    c(0x4b2737),
    c(0x442929),
    c(0xb10000),
    c(0x7b0000),
    c(0x500000),
    c(0xac2020),
    Box::new(/*trapdoor*/ |_wood, _| stack!(
        paint_svg_task("borderSolidThick", CRIMSON.color),
        paint_svg_task("trapdoor1", CRIMSON.shadow),
        paint_svg_task("borderShortDashes", CRIMSON.highlight),
        paint_svg_task("zigzagSolid2", CRIMSON.highlight),
        paint_svg_task("zigzagSolid", CRIMSON.shadow),
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_HIGHLIGHT),
        paint_svg_task("trapdoorHinges", ComparableColor::STONE_SHADOW)
    )),
    Box::new(/*door_bottom*/ |_wood, _| stack_on!(CRIMSON.color,
        paint_svg_task("planksTopBorderVertical", CRIMSON.shadow),
        paint_svg_task("borderShortDashes", CRIMSON.highlight),
        paint_svg_task("zigzagSolid2", CRIMSON.bark_highlight),
        paint_svg_task("zigzagSolid", CRIMSON.shadow),
        paint_svg_task("doorHingesBig", ComparableColor::STONE_HIGHLIGHT),
        paint_svg_task("doorHinges", ComparableColor::STONE_SHADOW)
    )),
    Box::new(/*leaves*/ |_wood| stack_on!(CRIMSON.leaves_color,
        paint_svg_task("leaves6", CRIMSON.leaves_shadow),
        paint_stack!(CRIMSON.leaves_highlight, "leaves6a", "borderRoundDots")
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("mushroomStem", CRIMSON.bark_shadow),
        paint_svg_task("mushroomCapRed", CRIMSON.leaves_color),
        paint_svg_task("crimsonFungusSpots", FUNGUS_SPOT_COLOR)
    )),
);}
lazy_static!{pub static ref WARPED: Wood = nether_fungus(
    "warped",
    c(0x286c6c),
    c(0x3a8d8d),
    c(0x003939),
    c(0x583838),
    c(0x00956f),
    c(0x440031),
    c(0x008282),
    c(0x00b485),
    c(0x006565),
    Box::new(/*trapdoor*/ |_wood, _| stack!(
        paint_svg_task("trapdoor1", WARPED.highlight),
        paint_svg_task("borderSolidThick", WARPED.color),
        paint_svg_task("borderSolid", WARPED.highlight),
        paint_svg_task("borderShortDashes", WARPED.shadow),
        paint_svg_task("waves", WARPED.color),
        stack!(
            paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_SHADOW),
            paint_svg_task("trapdoorHinges", ComparableColor::STONE_HIGHLIGHT)
        )
    )),
    Box::new(/*door_bottom*/ |_wood, _| stack_on!(WARPED.color,
        paint_svg_task("planksTopBorderVertical", WARPED.shadow),
        paint_svg_task("borderShortDashes", WARPED.highlight),
        paint_svg_task("waves", WARPED.bark_highlight),
        stack!(
            paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW),
            paint_svg_task("doorHinges", ComparableColor::STONE_HIGHLIGHT)
        )
    )),
    Box::new(/*leaves*/ |_wood| stack_on!(WARPED.leaves_color,
        paint_stack!(WARPED.leaves_shadow, "leaves3", "borderSolid"),
        paint_stack!(WARPED.leaves_highlight, "leaves3a", "leaves3b", "borderShortDashes")
    )),
    Box::new(/*sapling*/ |_wood| stack!(
        paint_svg_task("mushroomStem", WARPED.bark_shadow),
        paint_svg_task("warpedFungusCap", WARPED.leaves_color),
        paint_svg_task("warpedFungusSpots", FUNGUS_SPOT_COLOR)
    ))
);}

impl Material for Wood {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        let door_common_layers: ToPixmapTaskSpec = (self.door_common_layers)(self);
        let door_bottom: ToPixmapTaskSpec = (self.door_bottom)(self, door_common_layers.to_owned());
        let stripped_log_side: ToPixmapTaskSpec = (self.stripped_log_side)(self);
        let stripped_log_top: ToPixmapTaskSpec = (self.stripped_log_top)(self);
        vec![
            out_task(&format!("block/{}_{}", self.name, self.log_synonym), (self.bark)(self)),
            out_task(&format!("block/stripped_{}_{}", self.name, self.log_synonym), stripped_log_side),
            out_task(&format!("block/stripped_{}_{}_top", self.name, self.log_synonym), stripped_log_top.to_owned()),
            out_task(&format!("block/{}_{}_top", self.name, self.log_synonym), (self.log_top)(self, stripped_log_top)),
            out_task(&format!("block/{}_trapdoor", self.name), (self.trapdoor)(self, door_common_layers.to_owned())),
            out_task(&format!("block/{}_door_top", self.name), (self.door_top)(self, door_bottom.to_owned(), door_common_layers)),
            out_task(&format!("block/{}_door_bottom", self.name), door_bottom),
            out_task(&format!("block/{}_{}", self.name, self.leaves_synonym), (self.leaves)(self)),
            out_task(&format!("block/{}_{}", self.name, self.sapling_synonym), (self.sapling)(self)),
            out_task(&format!("block/{}_planks", self.name), self.planks())
        ]
    }
}

group!(OVERWORLD_WOOD = ACACIA, BIRCH, DARK_OAK, JUNGLE, MANGROVE, SPRUCE, OAK);
group!(NETHER_FUNGUS = CRIMSON, WARPED);
group!(WOOD = OVERWORLD_WOOD, NETHER_FUNGUS);