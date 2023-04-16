use crate::group;

mod axe;
mod barehand;
mod hoe;
mod indestructible;
mod liquid;
mod pickaxe;
mod shears;
mod shovel;

group!(ALL_BLOCKS = axe::AXE_BLOCKS, pickaxe::PICKAXE_BLOCKS);