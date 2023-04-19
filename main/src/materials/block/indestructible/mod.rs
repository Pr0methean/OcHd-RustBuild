use crate::group;
use crate::materials::block::indestructible::command_block::COMMAND_BLOCKS;
use crate::materials::block::indestructible::simple_indestructible_block::SIMPLE_INDESTRUCTIBLE_BLOCKS;
use crate::materials::block::indestructible::structure_jigsaw::{JIGSAW_BLOCKS, STRUCTURE_BLOCKS};

mod command_block;
mod structure_jigsaw;
mod simple_indestructible_block;

group!(INDESTRUCTIBLE_BLOCKS = COMMAND_BLOCKS, STRUCTURE_BLOCKS, JIGSAW_BLOCKS,
        SIMPLE_INDESTRUCTIBLE_BLOCKS);