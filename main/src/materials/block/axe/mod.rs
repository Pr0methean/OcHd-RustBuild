use crate::group;

pub mod giant_mushroom;
pub mod wood;
mod simple_axe_block;

group!(AXE_BLOCKS = giant_mushroom::GIANT_MUSHROOM, simple_axe_block::SIMPLE_AXE_BLOCK, wood::WOOD);
