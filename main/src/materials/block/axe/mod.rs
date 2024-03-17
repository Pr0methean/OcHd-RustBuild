use crate::group;

pub mod giant_mushroom;
mod simple_axe_block;
pub mod wood;

group!(
    AXE_BLOCKS = giant_mushroom::GIANT_MUSHROOM,
    simple_axe_block::SIMPLE_AXE_BLOCK,
    wood::WOOD
);
