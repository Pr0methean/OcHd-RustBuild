use std::sync::Arc;
use once_cell::sync::Lazy;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{out_task, paint_svg_task, FileOutputTaskSpec, ToPixmapTaskSpec};
use crate::{group, paint_stack, stack, stack_on};
use crate::materials::block::pickaxe::ore_base::{DEEPSLATE, DEEPSLATE_BASE, NETHERRACK_BASE, OreBase, STONE_BASE};
use crate::texture_base::material::{TextureSupplier, TextureUnaryFunc, ColorTriad, Material, TricolorMaterial, REDSTONE_ON};

pub static OVERWORLD_SUBSTRATES: Lazy<Vec<OreBase>> = Lazy::new(|| vec![
    STONE_BASE.to_owned(), DEEPSLATE_BASE.to_owned()
]);

pub static ALL_SUBSTRATES: Lazy<Vec<OreBase>> = Lazy::new(|| vec![
    STONE_BASE.to_owned(), DEEPSLATE_BASE.to_owned(), NETHERRACK_BASE.to_owned()
]);

pub struct Ore {
    pub name: &'static str,
    pub colors: ColorTriad,
    substrates: Vec<OreBase>,
    needs_refining: bool,
    svg_name: &'static str,
    item_name: &'static str,
    pub refined_colors: ColorTriad,
    pub raw_item: TextureSupplier<Ore>,
    pub refined_block: TextureSupplier<Ore>,
    pub refined_item: TextureSupplier<Ore>,
    pub raw_block: TextureSupplier<Ore>,
    pub ore_block_for_substrate: TextureUnaryFunc<Ore>
}

impl Ore {
    fn default_single_layer_item(&self) -> ToPixmapTaskSpec {
        paint_svg_task(self.svg_name, self.colors.color)
    }

    fn basic_refined_block(&self) -> ToPixmapTaskSpec {
        stack_on!(self.refined_colors.color,
            paint_svg_task("streaks", self.refined_colors.highlight),
            paint_stack!(self.refined_colors.shadow, self.svg_name, "borderSolid"),
            paint_svg_task("borderSolidTopLeft", self.refined_colors.highlight)
        )
    }

    fn basic_ingot(&self) -> ToPixmapTaskSpec {
        stack!(
            paint_svg_task("ingotMask", self.refined_colors.color),
            paint_stack!(self.refined_colors.shadow, "ingotBorder", self.svg_name),
            paint_svg_task("ingotBorderTopLeft", self.refined_colors.highlight)
        )
    }

    fn basic_raw_ore(&self) -> ToPixmapTaskSpec {
        stack!(
            paint_svg_task("circle32BottomLeftTopRight", self.colors.shadow),
            paint_svg_task("circle32TopLeftBottomRight", self.colors.color),
            paint_svg_task(self.svg_name, self.colors.highlight)
        )
    }

    fn basic_raw_block(&self) -> ToPixmapTaskSpec {
        stack_on!(self.colors.color,
            paint_svg_task("checksSmall", self.colors.highlight),
            paint_svg_task(self.svg_name, self.colors.shadow)
        )
    }

    fn basic_ore_block_for_substrate(&self, substrate: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
        stack!(
            substrate,
            self.default_single_layer_item()
        )
    }

    fn raw_item_based_ore_block_for_substrate(&self, substrate: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
        stack!(
            substrate,
            (self.raw_item)(self)
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
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        let mut output = Vec::with_capacity(7);
        for substrate in &self.substrates {
            output.push(out_task(
                format!("block/{}{}_ore", substrate.block_name_prefix, self.name),
                (self.ore_block_for_substrate)(self,
                                               substrate.material.material.texture())));
        }
        if self.name != "quartz" {
            // quartz textures are defined separately in simple_pickaxe_block.rs
            output.push(out_task(
                format!("block/{}_block", self.name), (self.refined_block)(self)
            ));
        }
        if self.needs_refining {
            output.push(out_task(
                format!("block/raw_{}_block", self.name), (self.raw_block)(self)
            ));
            output.push(out_task(
                format!("item/raw_{}", self.item_name), (self.raw_item)(self)
            ));
            output.push(out_task(
                format!("item/{}_ingot", self.name), (self.refined_item)(self)
            ));
        } else {
            output.push(out_task(
                format!("item/{}", self.item_name), (self.raw_item)(self)
            ));
        }
        output.into()
    }
}

impl TricolorMaterial for Ore {
    fn color(&self) -> ComparableColor {
        self.colors.color
    }

    fn shadow(&self) -> ComparableColor {
        self.colors.shadow
    }

    fn highlight(&self) -> ComparableColor {
        self.colors.highlight
    }
}

pub static COAL: Lazy<Ore> = Lazy::new(|| {
    let mut coal = Ore::new("coal",
                            ComparableColor::DARKEST_GRAY,
                            ComparableColor::BLACK,
                            ComparableColor::STONE_EXTREME_SHADOW);
    coal.ore_block_for_substrate = Box::new(|deferred_self, ore_base| {
        if ore_base == DEEPSLATE.material.texture() {
            stack!(
                DEEPSLATE.material.texture(),
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
});
pub static COPPER: Lazy<Ore> = Lazy::new(|| {
    let mut copper = Ore::new("copper",
                              c(0xff8000),
                              c(0x915400),
                              c(0xffa268));
    copper.needs_refining = true;
    copper.refined_colors = ColorTriad {
        color: c(0xe0734d),
        shadow: c(0x915431),
        highlight: c(0xff8268)
    };
    copper
});
pub static IRON: Lazy<Ore> = Lazy::new(|| {
    let mut iron = Ore::new("iron",
                        c(0xffaf93),
                        c(0xaf8e77),
                        c(0xFFCDB2));
    iron.needs_refining = true;
    iron.refined_colors = ColorTriad {
        color: ComparableColor::LIGHTEST_GRAY,
        highlight: ComparableColor::WHITE,
        shadow: ComparableColor::STONE_HIGHLIGHT,
    };
    iron
});
pub static REDSTONE: Lazy<Ore> = Lazy::new(|| {
    let mut redstone = Ore::new("redstone",
                        ComparableColor::RED,
                        c(0xba0000),
                        REDSTONE_ON);
    redstone.raw_item = Box::new(Ore::basic_raw_ore);
    redstone.ore_block_for_substrate = Box::new(|_, substrate| {
        stack!(
            substrate,
            paint_svg_task("redstoneShadows", REDSTONE.colors.shadow),
            paint_svg_task("redstoneHighlights", REDSTONE.colors.highlight)
        )
    });
    redstone
});
pub static LAPIS: Lazy<Ore> = Lazy::new(|| {
    let mut lapis = Ore::new("lapis", c(0x0055bd), c(0x0000aa), c(0x6995ff));
    lapis.item_name = "lapis_lazuli";
    lapis.raw_item = Box::new(|_| stack!(
        paint_svg_task("lapis", LAPIS.colors.color),
        paint_svg_task("lapisHighlight", LAPIS.colors.highlight),
        paint_svg_task("lapisShadow", LAPIS.colors.shadow)
    ));
    lapis.refined_block = Box::new(|_| stack_on!(
        LAPIS.colors.highlight,
        paint_svg_task("checksLarge", LAPIS.colors.shadow),
        paint_svg_task("checksSmall", LAPIS.colors.color),
        paint_svg_task("borderSolid", LAPIS.colors.shadow),
        paint_svg_task("borderSolidTopLeft", LAPIS.colors.highlight)
    ));
    lapis.ore_block_for_substrate = Box::new(Ore::raw_item_based_ore_block_for_substrate);
    lapis
});
pub static DIAMOND: Lazy<Ore> = Lazy::new(|| {
    let mut diamond = Ore::new("diamond", c(0x20d3d3), c(0x209797), c(0x77e7d1));
    let extreme_highlight = c(0xd5ffff);
    diamond.raw_item = Box::new(move |_| stack!(
        paint_svg_task("diamond1", extreme_highlight),
        paint_svg_task("diamond2", DIAMOND.colors.shadow)
    ));
    diamond.refined_block = Box::new(move |deferred_self| stack_on!(
        DIAMOND.colors.color,
        paint_svg_task("streaks", DIAMOND.colors.highlight),
        (DIAMOND.raw_item)(deferred_self),
        paint_svg_task("borderSolid", DIAMOND.colors.shadow),
        paint_svg_task("borderSolidTopLeft", extreme_highlight)
    ));
    diamond.ore_block_for_substrate = Box::new(Ore::raw_item_based_ore_block_for_substrate);
    diamond
});
pub static GOLD: Lazy<Ore> = Lazy::new(|| {
    let mut gold = Ore::new("gold",
                                ComparableColor::YELLOW,
                                c(0xeb9d00),
                                c(0xffffb5));
    gold.needs_refining = true;
    gold.substrates = ALL_SUBSTRATES.to_owned();
    gold.raw_item = Box::new(|_| stack!(
        paint_svg_task("circle32BottomLeftTopRight", GOLD.colors.highlight),
        paint_svg_task("circle32TopLeftBottomRight", GOLD.colors.color),
        paint_svg_task("gold", GOLD.colors.shadow)
    ));
    gold
});
pub static QUARTZ: Lazy<Ore> = Lazy::new(|| {
    let mut quartz = Ore::new("quartz",
                              c(0xe8e8de),
                              c(0xb6a48e),
                              ComparableColor::WHITE);
    quartz.substrates = vec![NETHERRACK_BASE.to_owned()];
    quartz.raw_item = Box::new(|_| stack!(
        paint_svg_task("quartzChunk", QUARTZ.colors.color),
        paint_stack!(QUARTZ.colors.highlight, "diamond2", "quartzChunkTopLeftBottomRight"),
        paint_svg_task("quartz", QUARTZ.colors.shadow)
    ));
    quartz
});
pub static EMERALD: Lazy<Ore> = Lazy::new(|| {
    let mut emerald = Ore::new("emerald", c(0x009829), c(0x007b18), c(0x00dd62));
    let extreme_highlight = c(0xd9ffeb);
    emerald.raw_item = Box::new(|_| stack!(
        paint_svg_task("emerald", EMERALD.colors.shadow),
        paint_svg_task("emeraldTopLeft", EMERALD.colors.highlight),
    ));
    emerald.refined_block = Box::new(move |_| stack_on!(
        EMERALD.colors.highlight,
        paint_svg_task("emerald", EMERALD.colors.shadow),
        paint_svg_task("borderSolid", EMERALD.colors.color),
        paint_stack!(extreme_highlight, "emeraldTopLeft", "borderSolidTopLeft")
    ));
    emerald.ore_block_for_substrate = Box::new(Ore::raw_item_based_ore_block_for_substrate);
    emerald
});

group!(ORES = COPPER, COAL, IRON, REDSTONE, LAPIS, GOLD, QUARTZ, DIAMOND, EMERALD);