use crate::group;
use crate::materials::block::bare_hand::biome_colorized_plant::BIOME_COLORIZED;
use crate::materials::block::bare_hand::cave_vines::CAVE_VINE_VARIANTS;
use crate::materials::block::bare_hand::crop::CROPS;
use crate::materials::block::bare_hand::simple_bare_hand_block::SIMPLE_BARE_HAND_BLOCKS;
use crate::materials::block::bare_hand::sunflower::SUNFLOWER;
use crate::materials::block::bare_hand::tnt::TNT;
use crate::materials::block::bare_hand::torch::TORCHES;
use crate::materials::block::bare_hand::wool::WOOL;

mod biome_colorized_plant;
mod cave_vines;
mod crop;
pub mod simple_bare_hand_block;
mod sunflower;
mod tnt;
mod torch;
mod wool;

group!(
    BARE_HAND_BLOCKS = BIOME_COLORIZED,
    CAVE_VINE_VARIANTS,
    CROPS,
    SUNFLOWER,
    SIMPLE_BARE_HAND_BLOCKS,
    TNT,
    TORCHES,
    WOOL
);
