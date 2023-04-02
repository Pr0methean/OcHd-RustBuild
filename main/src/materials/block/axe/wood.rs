/*
    JUNGLE(
        color = c(0x915431),
        shadow = c(0x795b4b),
        highlight = c(0x8A593A),
        bark_color = c(0x483800),
        bark_shadow = c(0x2B2000),
        bark_highlight = c(0x8A593A)
    ) {
        override fun LayerListBuilder.doorCommonLayers() {
            layer("doorHingesBig", STONE.shadow)
            layer("doorHinges", STONE.color)
        }

        override fun LayerListBuilder.trapdoor(commonLayers: AbstractImageTask) {
            layer("trapdoor2", color)
            layer("borderSolid", shadow)
            layer("borderShortDashes", highlight)
            layer("trapdoorHingesBig", STONE.shadow)
            layer("trapdoorHinges", STONE.color)
        }

        override fun LayerListBuilder.doorTop(doorBottom: AbstractImageTask, commonLayers: AbstractImageTask) {
            layer("trapdoor2", color)
            layer("borderShortDashes", highlight)
            copy(commonLayers)
            layer("doorKnob")
        }

        override fun LayerListBuilder.doorBottom(commonLayers: AbstractImageTask) {
            background(color)
            layer("waves", highlight)
            layer("planksTopBorderVertical", shadow)
            layer("borderSolid", color)
            layer("borderShortDashes", highlight)
            copy(commonLayers)
        }

        override fun LayerListBuilder.leaves() {
            layer("leaves6", leaves_highlight)
            layer("leaves6a", leaves_shadow)
        }

        override fun LayerListBuilder.sapling() {
            layer("saplingStem", bark_color)
            layer("acaciaSapling", c(0x378020))
        }
    },
    MANGROVE(
        color = c(0x773636),
        highlight = c(0x8A593A),
        shadow = c(0x500000),
        bark_color = c(0x583838),
        bark_highlight = c(0x624033),
        bark_shadow = c(0x4a4a39)
    ) {
        override val sapling_synonym: &str = "propagule"

        override fun LayerListBuilder.doorCommonLayers() {
            layer("rings2", shadow)
            layer("borderDotted", highlight)
        }

        override fun LayerListBuilder.trapdoor(commonLayers: AbstractImageTask) {
            layer("ringsHole", color)
            copy(commonLayers)
            layer("trapdoorHingesBig", STONE.highlight)
            layer("trapdoorHinges", STONE.shadow)
        }

        override fun LayerListBuilder.doorBottom(commonLayers: AbstractImageTask) {
            background(color)
            copy(commonLayers)
            layer("doorHingesBig", STONE.highlight)
            layer("doorHinges", STONE.shadow)
        }

        override fun LayerListBuilder.leaves() {
            layer("leaves5", leaves_highlight)
            layer("leaves5a", leaves_color)
            layer("leaves5b", leaves_shadow)
        }

        override fun LayerListBuilder.sapling() {
            layer("mangrovePropagule", c(0x4aa54a))
            layer("flowerStemBottomBorder", c(0x748241))
        }
    },
    SPRUCE(
        color = c(0x70583B),
        highlight = c(0x8A593A),
        shadow = c(0x624033),
        bark_color = c(0x3b2700),
        bark_highlight = c(0x624033),
        bark_shadow = c(0x2b2000)
    ) {
        override fun LayerListBuilder.doorCommonLayers(): Unit = copy(InvalidTask)

        override fun LayerListBuilder.trapdoor(commonLayers: AbstractImageTask) {
            background(shadow)
            layer("planksTopVertical", color)
            layer("borderSolidThick", shadow)
            layer("borderLongDashes", highlight)
            layer("trapdoorHingesBig", STONE.color)
            layer("trapdoorHinges", STONE.shadow)
        }

        override fun LayerListBuilder.doorBottom(commonLayers: AbstractImageTask) {
            copy {
                planks()
            }
            layer("doorHingesBig", STONE.color)
            layer("doorHinges", STONE.shadow)
        }

        override fun LayerListBuilder.leaves() {
            layer("leaves3", leaves_highlight)
            layer("leaves3b", leaves_shadow)
        }

        override fun LayerListBuilder.sapling() {
            layer("saplingStem", bark_highlight)
            layer("spruceSapling", c(0x2e492e))
        }
    },
    OAK(
        color = c(0xaf8f55),
        highlight = c(0xC29d62),
        shadow = c(0x70583B),
        bark_color = c(0x70583B),
        bark_highlight = c(0x987849),
        bark_shadow = c(0x4a4a39)
    ) {
        override fun LayerListBuilder.doorCommonLayers() {
            copy {
                layer("borderSolidThick", color)
                layer("borderSolid", highlight)
            }
            layer("cross", highlight)
            copy {
                layer("2x2TopLeft", shadow)
                layer("borderShortDashes", color, 0.5)
            }
        }

        override fun LayerListBuilder.trapdoor(commonLayers: AbstractImageTask) {
            copy(commonLayers)
            layer("trapdoorHingesBig", STONE.color)
            layer("trapdoorHinges", STONE.highlight)
        }

        override fun LayerListBuilder.doorTop(doorBottom: AbstractImageTask, commonLayers: AbstractImageTask) {
            copy {
                layer("borderSolidThick", color)
                layer("borderSolid", highlight)
            }
            copy {
                layer("2x2TopLeft", shadow)
                layer("borderShortDashes", color, 0.5)
            }
            layer("craftingSide", shadow)
            layer("cross", shadow)
            layer("doorKnob")
            copy {
                layer("doorHingesBig", STONE.color)
                layer("doorHinges", STONE.highlight)
            }
        }

        override fun LayerListBuilder.doorBottom(commonLayers: AbstractImageTask) {
            background(color)
            layer("waves", highlight)
            copy(commonLayers)
            copy {
                layer("doorHingesBig", STONE.color)
                layer("doorHinges", STONE.highlight)
            }
        }

        override fun LayerListBuilder.leaves() {
            layer("leaves4", leaves_shadow)
            layer("leaves4a", leaves_highlight)
        }

        override fun LayerListBuilder.sapling() {
            layer("coalBorder", c(0x005c00))
            layer("saplingStem", bark_color)
            layer("coal", c(0x57ad3f))
            layer("sunflowerPistil", c(0x005c00))
        }
    };


    override fun LayerListBuilder.bark() {
        background(bark_color)
        layer("borderSolid", bark_shadow)
        layer("borderDotted", bark_highlight)
        layer("zigzagSolid", bark_shadow)
        layer("zigzagSolid2", bark_highlight)
    }

    override fun LayerListBuilder.strippedLogSide() {
        background(color)
        layer("borderSolid", shadow)
        layer("borderShortDashes", highlight)
    }

    override fun LayerListBuilder.logTop(strippedLogTop: AbstractImageTask) {
        copy(strippedLogTop)
        layer("borderSolid", bark_color)
        layer("borderDotted", bark_shadow)
    }

    override fun LayerListBuilder.strippedLogTop(strippedLogSide: AbstractImageTask) {
        copy(strippedLogSide)
        layer("ringsCentralBullseye", highlight)
        layer("rings", shadow)
    }

    override val log_synonym: &str = "log"
    override val leaves_synonym: &str = "leaves"
    override val sapling_synonym: &str = "sapling"

    // Like grass, leaves are stored as gray and colorized in real time based on the biome
    override val leaves_color: Paint = DirtGroundCover.GRASS_BLOCK.color
    override val leaves_highlight: Paint = DirtGroundCover.GRASS_BLOCK.highlight
    override val leaves_shadow: Paint = DirtGroundCover.GRASS_BLOCK.shadow
}

private val fungusSpotColor = c(0xff6500)
@Suppress("unused", "LongParameterList")
enum class Fungus(
        override val color: Paint,
        override val highlight: Paint,
        override val shadow: Paint,
        override val bark_color: Paint,
        override val bark_highlight: Paint,
        override val bark_shadow: Paint,
        override val leaves_color: Paint,
        override val leaves_highlight: Paint,
        override val leaves_shadow: Paint)
    : Wood {
        CRIMSON(
            color = c(0x6a344b),
            shadow = c(0x4b2737),
            highlight = c(0x863e5a),
            bark_color = c(0x4b2737),
            bark_shadow = c(0x442929),
            bark_highlight = c(0xb10000),
            leaves_color = c(0x7b0000),
            leaves_shadow = c(0x500000),
            leaves_highlight = c(0xac2020),
        ) {
            override fun LayerListBuilder.doorCommonLayers(): Unit = copy(InvalidTask)

            override fun LayerListBuilder.trapdoor(commonLayers: AbstractImageTask) {
                layer("borderSolidThick", color)
                layer("trapdoor1", shadow)
                layer("borderShortDashes", highlight)
                layer("zigzagSolid2", highlight)
                layer("zigzagSolid", shadow)
                layer("trapdoorHingesBig", STONE.highlight)
                layer("trapdoorHinges", STONE.shadow)
            }

            override fun LayerListBuilder.doorBottom(commonLayers: AbstractImageTask) {
                background(color)
                layer("planksTopBorderVertical", shadow)
                layer("borderShortDashes", highlight)
                layer("zigzagSolid2", bark_highlight)
                layer("zigzagSolid", shadow)
                layer("doorHingesBig", STONE.highlight)
                layer("doorHinges", STONE.shadow)
            }

            override fun LayerListBuilder.leaves() {
                background(leaves_color)
                layer("leaves6", leaves_shadow)
                layer("leaves6a", leaves_highlight)
                layer("borderRoundDots", leaves_highlight)
            }
            override fun LayerListBuilder.sapling() {
                layer("mushroomStem", bark_shadow)
                layer("mushroomCapRed", leaves_color)
                layer("crimsonFungusSpots", fungusSpotColor)
            }
        }, WARPED(
            color = c(0x286c6c),
            shadow = c(0x003939),
            highlight = c(0x3a8d8d),
            bark_color = c(0x583838),
            bark_shadow = c(0x440031),
            bark_highlight = c(0x00956f),
            leaves_color = c(0x008282),
            leaves_highlight = c(0x00b485),
            leaves_shadow = c(0x006565),
        ) {
        override fun LayerListBuilder.doorCommonLayers(): Unit = copy(InvalidTask)

        override fun LayerListBuilder.trapdoor(commonLayers: AbstractImageTask) {
            layer("trapdoor1", highlight)
            layer("borderSolidThick", color)
            layer("borderSolid", highlight)
            layer("borderShortDashes", shadow)
            layer("waves", color)
            copy {
                layer("trapdoorHingesBig", STONE.shadow)
                layer("trapdoorHinges", STONE.highlight)
            }
        }

        override fun LayerListBuilder.doorBottom(commonLayers: AbstractImageTask) {
            background(color)
            layer("planksTopBorderVertical", shadow)
            layer("borderShortDashes", highlight)
            layer("waves", bark_highlight)
            layer("doorHingesBig", STONE.shadow)
            layer("doorHinges", STONE.highlight)
        }

        override fun LayerListBuilder.leaves() {
            background(leaves_color)
            layer("leaves3", leaves_shadow)
            layer("borderSolid", leaves_shadow)
            layer("leaves3a", leaves_highlight)
            layer("leaves3b", leaves_highlight)
            layer("borderShortDashes", leaves_highlight)
        }

        override fun LayerListBuilder.sapling() {
            layer("mushroomStem", bark_shadow)
            layer("warpedFungusCap", leaves_color)
            layer("warpedFungusSpots", fungusSpotColor)
        }
    };

    override fun LayerListBuilder.bark() {
        background(bark_color)
        layer("borderSolid", bark_shadow)
        layer("waves", bark_highlight)
    }

    override fun LayerListBuilder.strippedLogSide() {
        background(color)
        layer("borderSolid", shadow)
        layer("borderDotted", highlight)
    }

    override fun LayerListBuilder.logTop(strippedLogTop: AbstractImageTask) {
        background(color)
        copy {
            layer("ringsCentralBullseye", shadow)
            layer("rings2", highlight)
        }
        layer("borderSolid", bark_color)
        layer("borderShortDashes", bark_shadow)
    }

    override fun LayerListBuilder.strippedLogTop(strippedLogSide: AbstractImageTask) {
        copy(strippedLogSide)
        copy {
            layer("ringsCentralBullseye", shadow)
            layer("rings2", highlight)
        }
    }

    override val log_synonym: &str = "stem"
    override val leaves_synonym: &str = "wart_block"
    override val sapling_synonym: &str = "fungus"
}
 */

use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{paint_svg_task, from_svg_task, TaskSpec, out_task};
use crate::texture_base::material::Material;
use crate::{group, repaint_stack, stack, stack_on};
use std::sync::Arc;

struct Wood {
    color: ComparableColor,
    highlight: ComparableColor,
    shadow: ComparableColor,
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
    bark: Box<dyn (Fn(&Self) -> Arc<TaskSpec>) + Sync + Send>,
    stripped_log_side: Box<dyn (Fn(&Self) -> Arc<TaskSpec>) + Sync + Send>,
    log_top: Box<dyn (Fn(&Self, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
    stripped_log_top: Box<dyn (Fn(&Self, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
    trapdoor: Box<dyn (Fn(&Self, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
    door_top: Box<dyn (Fn(&Self, Arc<TaskSpec>, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
    door_bottom: Box<dyn (Fn(&Self, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
    leaves: Box<dyn (Fn(&Self) -> Arc<TaskSpec>) + Sync + Send>,
    sapling: Box<dyn (Fn(&Self) -> Arc<TaskSpec>) + Sync + Send>,
    door_common_layers: Box<dyn (Fn(&Self) -> Arc<TaskSpec>) + Sync + Send>,
}

impl Wood {
    pub fn planks(&self) -> Arc<TaskSpec> {
        return stack!(
            stack_on!(self.color,
                paint_svg_task("waves2", self.highlight),
                repaint_stack!(self.shadow, from_svg_task("waves"), from_svg_task("planksTopBorder"))
            ),
            paint_svg_task("borderShortDashes", self.highlight),
        );
    }

    pub fn overworld_bark(&self) -> Arc<TaskSpec> {
        return stack_on!(self.bark_color,
            paint_svg_task("borderSolid", self.bark_shadow),
            paint_svg_task("borderDotted", self.bark_highlight),
            paint_svg_task("zigzagSolid", self.bark_shadow),
            paint_svg_task("zigzagSolid2", self.bark_highlight)
        );
    }

    pub fn overworld_stripped_log_side(&self) -> Arc<TaskSpec> {
        return stack_on!(self.color,
            paint_svg_task("borderSolid", self.shadow),
            paint_svg_task("borderShortDashes", self.highlight)
        );
    }

    pub fn overworld_log_top(&self, stripped_log_top: Arc<TaskSpec>) -> Arc<TaskSpec> {
        return stack!(
            stripped_log_top,
            paint_svg_task("borderSolid", self.barkColor),
            paint_svg_task("borderDotted", self.barkShadow),
        )
    }

    pub fn overworld_stripped_log_top(&self, stripped_log_side: Arc<TaskSpec>) -> Arc<TaskSpec> {
        return stack!(
            stripped_log_side,
            paint_svg_task("ringsCentralBullseye", self.highlight),
            paint_svg_task("rings", self.shadow),
        )
    }

    pub fn default_door_top(&self, door_bottom: Arc<TaskSpec>, _: Arc<TaskSpec>) -> Arc<TaskSpec> {
        return stack!(
            door_bottom,
            from_svg_task("doorKnob")
        )
    }
}

pub fn overworld_wood(name: &str, color: ComparableColor,
                      highlight: ComparableColor, shadow: ComparableColor,
                      bark_color: ComparableColor, bark_highlight: ComparableColor,
                      bark_shadow: ComparableColor,
                      door_common_layers: Box<dyn (Fn(&Wood) -> Arc<TaskSpec>) + Sync + Send>,
                      trapdoor: Box<dyn (Fn(&Wood, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
                      door_bottom: Box<dyn (Fn(&Wood, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
                      door_top: Box<dyn (Fn(&Wood, Arc<TaskSpec>, Arc<TaskSpec>) -> Arc<TaskSpec>) + Sync + Send>,
                      leaves: Box<dyn (Fn(&Wood) -> Arc<TaskSpec>) + Sync + Send>,
                      sapling: Box<dyn (Fn(&Wood) -> Arc<TaskSpec>) + Sync + Send>) -> Wood {
    return Wood {
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
    };
}

pub const ACACIA: Wood = overworld_wood(
    "acacia",
    c(0xad5d32),
    c(0xc26d3f),
    c(0x915431),
    c(0x70583B),
    c(0x898977),
    c(0x4a4a39),
    Box::new(/*door_common_layers*/ |&wood| stack!(
        paint_svg_task("borderSolidThick", ACACIA.color),
        paint_svg_task("borderSolid", ACACIA.highlight),
        paint_svg_task("bigDiamond", ACACIA.shadow)
    )),
    Box::new(/*trapdoor*/ |&wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_SHADOW),
        paint_svg_task("trapdoorHinges", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(/*door_bottom*/ |&wood, door_common_layers| stack!(
        door_common_layers,
        repaint_stack!(ACACIA.color,
            from_svg_task("strokeBottomLeftTopRight"),
            from_svg_task("strokeTopLeftBottomRight")
        ),
        paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW),
        paint_svg_task("doorHinges", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(Wood::default_door_top),
    Box::new(/*leaves*/ |&wood| stack!(
        paint_svg_task("leaves1", ACACIA.leaves_shadow),
        paint_svg_task("leaves1a", ACACIA.leaves_highlight)
    )),
    Box::new(/*sapling*/ |&wood| stack!(
        paint_svg_task("saplingStem", ACACIA.bark_color),
        paint_svg_task("acaciaSapling", c(0x6c9e38)),
        paint_svg_task("acaciaSapling2", c(0xc9d7a5))
    )),
);
pub const BIRCH: Wood = overworld_wood(
    "birch",
    c(0xc8b77a),
    c(0xD7C187),
    c(0x915431),
    c(0xeeffea),
    ComparableColor::WHITE,
    c(0x5f5f4f),
    Box::new(/*door_common_layers*/ |&wood| stack_on!(BIRCH.bark_highlight,
        paint_svg_task("borderSolidExtraThick", BIRCH.color),
        paint_svg_task("craftingGridSquare", BIRCH.highlight),
        paint_svg_task("craftingGridSpaces", BIRCH.bark_highlight),
        paint_svg_task("borderSolid", BIRCH.shadow)
    )),
    Box::new(/*trapdoor*/ |&wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_SHADOW)
    )),
    Box::new(/*door_bottom*/ |&wood, door_common_layers| stack_on!(BIRCH.highlight,
            paint_svg_task("borderSolidExtraThick", BIRCH.color),
            paint_svg_task("craftingGridSquare", BIRCH.shadow),
            paint_svg_task("borderSolid", BIRCH.shadow),
            paint_svg_task("craftingGridSpaces", BIRCH.highlight),
            paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW)
    )),
    Box::new(/*door_top*/ |&wood, _, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("doorHingesBig", ComparableColor::STONE_SHADOW),
        from_svg_task("doorKnob")
    )),
    Box::new(/*leaves*/ |&wood| stack!(
        paint_svg_task("leaves2", BIRCH.leaves_highlight),
        paint_svg_task("leaves2a", BIRCH.leaves_shadow)
    )),
    Box::new(/*sapling*/ |&wood| stack!(
        paint_svg_task("saplingStem", BIRCH.bark_color),
        paint_svg_task("flowerStemBottomBorder", BIRCH.bark_shadow),
        paint_svg_task("saplingLeaves", c(0x6c9e38))
    )),
);
pub const DARK_OAK: Wood = overworld_wood(
    "dark_oak",
    c(0x3f2d23),
    c(0x3a2400),
    c(0x4a4a39),
    c(0x483800),
    c(0x2b2000),
    c(0x624033),
    Box::new(/*door_common_layers*/ |&wood| stack_on!(DARK_OAK.color,
        repaint_stack!(DARK_OAK.highlight, from_svg_task("borderSolid"), from_svg_task("cross")),
        paint_svg_task("2x2TopLeft", DARK_OAK.shadow),
        paint_svg_task("borderShortDashes", DARK_OAK.color)
    )),
    Box::new(/*trapdoor*/ |&wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("trapdoorHingesBig", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(/*door_bottom*/ |&wood, door_common_layers| stack!(
        door_common_layers,
        paint_svg_task("doorHingesBig", ComparableColor::STONE_HIGHLIGHT)
    )),
    Box::new(Wood::default_door_top),
    Box::new(/*leaves*/ |&wood| stack!(
        paint_svg_task("leaves3", DARK_OAK.leaves_shadow),
        paint_svg_task("leaves3a", DARK_OAK.leaves_highlight)
    )),
    Box::new(/*sapling*/ |&wood| stack!(
        paint_svg_task("saplingStem", DARK_OAK.bark_color),
        paint_svg_task("bigCircle", c(0x005c00)),
        paint_svg_task("bigCircleTwoQuarters", c(0x57ad3f))
    )),
);

impl Material for Wood {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        let door_common_layers = (self.door_common_layers)(self);
        let door_bottom = (self.door_bottom)(self, door_common_layers);
        let stripped_log_side = (self.stripped_log_side)(self);
        let stripped_log_top = (self.stripped_log_top)(self, stripped_log_side);
        return vec![
            out_task(&*format!("block/{}_{}", self.name, self.log_synonym), (self.bark)(self)),
            out_task(&*format!("block/stripped_{}_{}", self.name, self.log_synonym), stripped_log_side),
            out_task(&*format!("block/stripped_{}_{}_top", self.name, self.log_synonym), stripped_log_top),
            out_task(&*format!("block/{}_{}_top", self.name, self.log_synonym), (self.log_top)(self, stripped_log_top)),
            out_task(&*format!("block/{}_trapdoor", self.name), (self.trapdoor)(self, door_common_layers)),
            out_task(&*format!("block/{}_door_top", self.name), (self.door_top)(self, door_bottom, door_common_layers)),
            out_task(&*format!("block/{}_door_bottom", self.name), door_bottom),
            out_task(&*format!("block/{}_{}", self.name, self.leaves_synonym), (self.leaves)(self)),
            out_task(&*format!("block/{}_{}", self.name, self.sapling_synonym), (self.sapling)(self)),
            out_task(&*format!("block/{}_planks", self.name), self.planks())
        ];
    }
}

group!(OVERWORLD_WOOD = ACACIA, BIRCH, DARK_OAK);