use crate::group;
use crate::materials::block::pickaxe::bone_block::BONE_BLOCK;
use crate::materials::block::pickaxe::concrete::CONCRETE;
use crate::materials::block::pickaxe::copper_oxide::COPPER_OXIDES;
use crate::materials::block::pickaxe::dyed_terracotta::TERRACOTTA;
use crate::materials::block::pickaxe::furnace::FURNACES;
use crate::materials::block::pickaxe::glass::GLASS_VARIANTS;
use crate::materials::block::pickaxe::misc_redstone::MISC_REDSTONE;
use crate::materials::block::pickaxe::nylium::NYLIUM;
use crate::materials::block::pickaxe::ore::ORES;
use crate::materials::block::pickaxe::ore_base::ORE_BASES;
use crate::materials::block::pickaxe::polishable::POLISHABLE;
use crate::materials::block::pickaxe::rail::RAILS;
use crate::materials::block::pickaxe::simple_pickaxe_block::SIMPLE_PICKAXE_BLOCKS;

mod bone_block;
mod concrete;
mod copper_oxide;
mod dyed_terracotta;
mod furnace;
mod glass;
mod misc_redstone;
mod nylium;
pub mod ore;
pub mod ore_base;
mod polishable;
mod rail;
pub mod simple_pickaxe_block;

group!(
    PICKAXE_BLOCKS = ORE_BASES,
    SIMPLE_PICKAXE_BLOCKS,
    ORES,
    RAILS,
    POLISHABLE,
    GLASS_VARIANTS,
    COPPER_OXIDES,
    TERRACOTTA,
    CONCRETE,
    NYLIUM,
    BONE_BLOCK,
    FURNACES,
    MISC_REDSTONE
);
