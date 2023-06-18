use crate::group;

mod block;
mod item;
mod particle;

group!(ALL_MATERIALS = item::ALL_ITEMS, block::ALL_BLOCKS, particle::ALL_PARTICLES);