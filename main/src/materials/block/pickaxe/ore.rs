use std::borrow::ToOwned;
use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, gray, c};
use crate::image_tasks::task_spec::{out_task, paint_svg_task, SinkTaskSpec, ToPixmapTaskSpec};
use crate::{group, paint_stack, stack, stack_on};
use crate::materials::block::pickaxe::ore_base::{DEEPSLATE, DEEPSLATE_BASE, NETHERRACK_BASE, OreBase, STONE, STONE_BASE};
use crate::texture_base::material::{AbstractTextureSupplier, AbstractTextureUnaryFunc, ColorTriad, Material};

lazy_static! {
    static ref OVERWORLD_SUBSTRATES: Vec<&'static OreBase> = vec![
        &STONE_BASE, &DEEPSLATE_BASE
    ];

    static ref ALL_SUBSTRATES: Vec<&'static OreBase> = vec![
        &STONE_BASE, &DEEPSLATE_BASE, &NETHERRACK_BASE
    ];
}

pub struct Ore {
    pub name: &'static str,
    pub colors: ColorTriad,
    substrates: Vec<&'static OreBase>,
    needs_refining: bool,
    svg_name: &'static str,
    item_name: &'static str,
    pub refined_colors: ColorTriad,
    pub raw_item: AbstractTextureSupplier<Ore>,
    pub refined_block: AbstractTextureSupplier<Ore>,
    pub refined_item: AbstractTextureSupplier<Ore>,
    pub raw_block: AbstractTextureSupplier<Ore>,
    pub ore_block_for_substrate: AbstractTextureUnaryFunc<Ore>
}

impl Ore {
    fn default_single_layer_item(&self) -> Box<ToPixmapTaskSpec> {
        paint_svg_task(self.svg_name, self.colors.color)
    }

    fn basic_refined_block(&self) -> Box<ToPixmapTaskSpec> {
        stack_on!(self.refined_colors.color,
            paint_svg_task("streaks", self.refined_colors.highlight),
            paint_stack!(self.refined_colors.shadow, self.svg_name, "borderSolid"),
            paint_svg_task("borderSolidTopLeft", self.refined_colors.highlight)
        )
    }

    fn basic_ingot(&self) -> Box<ToPixmapTaskSpec> {
        stack!(
            paint_svg_task("ingotMask", self.refined_colors.color),
            paint_stack!(self.refined_colors.shadow, "ingotBorder", self.svg_name),
            paint_svg_task("ingotBorderTopLeft", self.refined_colors.highlight)
        )
    }

    fn basic_raw_ore(&self) -> Box<ToPixmapTaskSpec> {
        stack!(
            paint_svg_task("bigCircle", self.colors.shadow),
            paint_svg_task("bigCircleTwoQuarters", self.colors.color),
            paint_svg_task(self.svg_name, self.colors.highlight)
        )
    }

    fn basic_raw_block(&self) -> Box<ToPixmapTaskSpec> {
        stack_on!(self.colors.color,
            paint_svg_task("checksSmall", self.colors.highlight),
            paint_svg_task(self.svg_name, self.colors.shadow)
        )
    }

    fn basic_ore_block_for_substrate(&self, substrate: Box<ToPixmapTaskSpec>) -> Box<ToPixmapTaskSpec> {
        stack!(
            substrate.to_owned(),
            self.default_single_layer_item().to_owned()
        )
    }

    pub fn new(
        name: &'static str,
        color: ComparableColor,
        shadow: ComparableColor,
        highlight: ComparableColor
    ) -> Ore {
        Ore {
            name,
            colors: ColorTriad {color, shadow, highlight},
            substrates: OVERWORLD_SUBSTRATES.to_owned(),
            needs_refining: false,
            svg_name: name,
            item_name: name,
            refined_colors: ColorTriad {color, shadow, highlight},
            raw_item: Box::new(Ore::basic_raw_ore),
            refined_block: Box::new(Ore::basic_refined_block),
            refined_item: Box::new(Ore::basic_ingot),
            raw_block: Box::new(Ore::basic_raw_block),
            ore_block_for_substrate: Box::new(Ore::basic_ore_block_for_substrate)
        }
    }
}

impl Material for Ore {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        /*
                substrates.forEach { oreBase ->
            out("block/${oreBase.orePrefix}${this@Ore.name}_ore", oreBlock(this@outputTasks, oreBase))
        }
        out("block/${this@Ore.name}_block") { block() }
        if (needsRefining) {
            out("block/raw_${this@Ore.name}_block") { rawBlock() }
            out("item/raw_${this@Ore.name}") { rawOre() }
            out("item/${this@Ore.name}_ingot") { ingot() }
        } else {
            out("item/${itemNameOverride ?: this@Ore.name}") { itemForOutput() }
        }
         */
        let mut output = Vec::with_capacity(7);
        for substrate in &self.substrates {
            output.push(out_task(
                &*format!("block/{}{}_ore", substrate.block_name_prefix, self.name),
                (self.ore_block_for_substrate)(self, substrate.material.texture.to_owned())));
        }
        if self.name != "quartz" {
            // quartz textures are defined separately in simple_pickaxe_block.rs
            output.push(out_task(
                &*format!("block/{}_block", self.name), (self.refined_block)(self)
            ));
        }
        if self.needs_refining {
            output.push(out_task(
                &*format!("block/raw_{}_block", self.name), (self.raw_block)(self)
            ));
            output.push(out_task(
                &*format!("item/raw_{}", self.name), (self.raw_item)(self)
            ));
            output.push(out_task(
                &*format!("item/{}_ingot", self.name), (self.refined_item)(self)
            ));
        } else {
            output.push(out_task(
                &*format!("item/{}", self.item_name), (self.raw_item)(self)
            ));
        }
        output
    }
}

lazy_static! {
    pub static ref COAL: Ore = {
        let mut coal = Ore::new("coal",
                                gray(0x2f),
                                ComparableColor::BLACK,
                                ComparableColor::STONE_EXTREME_SHADOW);
        coal.ore_block_for_substrate = Box::new(|deferred_self, ore_base| {
            if ore_base == DEEPSLATE.texture {
                stack!(
                    DEEPSLATE.texture.to_owned(),
                    paint_svg_task("coalBorder", deferred_self.colors.highlight),
                    deferred_self.default_single_layer_item()
                )
            } else {
                deferred_self.basic_ore_block_for_substrate(ore_base)
            }
        });

        coal.refined_block = Box::new(|deferred_self| stack_on!(deferred_self.colors.color,
            paint_stack!(deferred_self.refined_colors.highlight, "streaks", "coalBorder"),
            paint_stack!(deferred_self.refined_colors.shadow, "coal", "borderSolid"),
            paint_svg_task("borderSolidTopLeft", deferred_self.refined_colors.highlight)
        ));
        coal
    };
    pub static ref COPPER: Ore = {
        let mut copper = Ore::new("copper",
                                  c(0xe0734d),
                                  c(0x915431),
                                  c(0xff8268));
        copper.needs_refining = true;
        copper
    };
    pub static ref IRON: Ore = {
        let mut iron = Ore::new("iron",
                            c(0xd8af93),
                            c(0xaf8e77),
                            c(0xFFCDB2));
        iron.needs_refining = true;
        iron.refined_colors = ColorTriad {
            color: ComparableColor::LIGHTEST_GRAY,
            highlight: ComparableColor::WHITE,
            shadow: ComparableColor::STONE_HIGHLIGHT,
        };
        iron
    };
    pub static ref REDSTONE: Ore = {
        let mut redstone = Ore::new("redstone",
                            ComparableColor::RED,
                            c(0xca0000),
                            c(0xff5e5e));
        redstone.raw_item = Box::new(Ore::basic_raw_ore);
        redstone.ore_block_for_substrate = Box::new(|_, substrate| {
            if substrate == STONE.texture {
                stack!(
                    STONE.texture.to_owned(),
                    paint_svg_task("redstone", REDSTONE.colors.shadow)
                )
            } else {
                REDSTONE.basic_ore_block_for_substrate(substrate)
            }
        });
        redstone
    };
    pub static ref GOLD: Ore = {
        let mut gold = Ore::new("gold",
                                    ComparableColor::YELLOW,
                                    c(0xeb9d00),
                                    c(0xffffb5));
        gold.needs_refining = true;
        gold.substrates = ALL_SUBSTRATES.to_owned();
        gold.raw_item = Box::new(|_| stack!(
            paint_svg_task("bigCircle", GOLD.colors.highlight),
            paint_svg_task("bigCircleTwoQuarters", GOLD.colors.color),
            paint_svg_task("gold", GOLD.colors.shadow)
        ));
        gold
    };
    pub static ref QUARTZ: Ore = {
        let mut quartz = Ore::new("quartz",
                                  c(0xe8e8de),
                                  c(0xb6a48e),
                                  ComparableColor::WHITE);
        quartz.substrates = vec![&*NETHERRACK_BASE];
        quartz
    };
}

// TODO: emerald, lapis, diamond
group!(ORES = COPPER, COAL, IRON, REDSTONE, GOLD, QUARTZ);