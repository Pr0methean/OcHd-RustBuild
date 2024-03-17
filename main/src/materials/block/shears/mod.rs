use crate::group;
use crate::materials::block::shears::simple_shear_block::SIMPLE_SHEAR_BLOCKS;

mod simple_shear_block;

group!(SHEAR_BLOCKS = SIMPLE_SHEAR_BLOCKS);
