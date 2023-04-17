use crate::group;
use crate::materials::block::shovel::concrete_powder::CONCRETE_POWDER;
use crate::materials::block::shovel::dirt_ground_cover::DIRT_GROUND_COVER;
use crate::materials::block::shovel::simple_soft_earth::SIMPLE_SOFT_EARTH;

mod simple_soft_earth;
mod dirt_ground_cover;
mod concrete_powder;

group!(SHOVEL_BLOCKS = SIMPLE_SOFT_EARTH, DIRT_GROUND_COVER, CONCRETE_POWDER);