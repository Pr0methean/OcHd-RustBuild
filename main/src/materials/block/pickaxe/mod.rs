use crate::group;
use crate::materials::block::pickaxe::copper_oxide::COPPER_OXIDES;
use crate::materials::block::pickaxe::glass::GLASS_VARIANTS;
use crate::materials::block::pickaxe::ore::ORES;
use crate::materials::block::pickaxe::ore_base::ORE_BASES;
use crate::materials::block::pickaxe::polishable::POLISHABLE;
use crate::materials::block::pickaxe::rail::RAILS;
use crate::materials::block::pickaxe::simple_pickaxe_block::SIMPLE_PICKAXE_BLOCKS;

pub mod ore_base;
mod simple_pickaxe_block;
pub mod ore;
mod rail;
mod polishable;
mod glass;
mod copper_oxide;

group!(PICKAXE_BLOCKS = ORE_BASES, SIMPLE_PICKAXE_BLOCKS, ORES, RAILS, POLISHABLE,
    GLASS_VARIANTS, COPPER_OXIDES);