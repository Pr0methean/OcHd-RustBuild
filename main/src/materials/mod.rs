use crate::materials::block::ALL_BLOCKS;
use crate::materials::item::ALL_ITEMS;
use crate::materials::particle::ALL_PARTICLES;
use crate::texture_base::material::MaterialGroup;
use std::sync::Arc;

mod block;
mod item;
mod particle;

pub static ALL_MATERIALS: MaterialGroup = MaterialGroup {
    members: vec!(Arc::new(ALL_BLOCKS), Arc::new(ALL_ITEMS), Arc::new(ALL_PARTICLES))
};