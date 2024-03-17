use crate::group;
use crate::materials::block::indestructible::command_block::CommandBlocks::CommandBlocks;
use crate::materials::block::indestructible::simple_indestructible_block::SIMPLE_INDESTRUCTIBLE_BLOCKS;
use crate::materials::block::indestructible::structure_jigsaw::{JIGSAW_BLOCKS, STRUCTURE_BLOCKS};

mod command_block;
mod simple_indestructible_block;
mod structure_jigsaw;

group!(
    INDESTRUCTIBLE_BLOCKS = CommandBlocks,
    STRUCTURE_BLOCKS,
    JIGSAW_BLOCKS,
    SIMPLE_INDESTRUCTIBLE_BLOCKS
);
