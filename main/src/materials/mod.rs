use crate::group;

mod block;
mod item;
mod particle;

group!(ALL_MATERIALS = block::ALL_BLOCKS, item::ALL_ITEMS, particle::ALL_PARTICLES);