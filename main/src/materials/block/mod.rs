use crate::group;
use crate::materials::block::indestructible::INDESTRUCTIBLE_BLOCKS;

mod axe;
mod bare_hand;
mod hoe;
mod indestructible;
mod liquid;
mod pickaxe;
mod shears;
mod shovel;

group!(ALL_BLOCKS = axe::AXE_BLOCKS, pickaxe::PICKAXE_BLOCKS, shovel::SHOVEL_BLOCKS,
        shears::SHEAR_BLOCKS, hoe::HOE_BLOCKS, INDESTRUCTIBLE_BLOCKS);