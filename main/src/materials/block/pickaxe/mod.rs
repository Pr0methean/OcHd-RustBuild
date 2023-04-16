use crate::group;
use crate::materials::block::pickaxe::ore::ORES;
use crate::materials::block::pickaxe::ore_base::ORE_BASES;
use crate::materials::block::pickaxe::simple_pickaxe_block::SIMPLE_PICKAXE_BLOCKS;


mod ore_base;
mod simple_pickaxe_block;
mod ore;

group!(PICKAXE_BLOCKS = ORE_BASES, SIMPLE_PICKAXE_BLOCKS, ORES);