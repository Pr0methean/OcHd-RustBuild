use crate::group;

mod axe;
pub(crate) mod bare_hand;
mod hoe;
mod indestructible;
mod liquid;
pub(crate) mod pickaxe;
mod shears;
mod shovel;

group!(
    ALL_BLOCKS = indestructible::INDESTRUCTIBLE_BLOCKS,
    axe::AXE_BLOCKS,
    liquid::LIQUID_BLOCKS,
    shears::SHEAR_BLOCKS,
    shovel::SHOVEL_BLOCKS,
    pickaxe::PICKAXE_BLOCKS,
    hoe::HOE_BLOCKS,
    bare_hand::BARE_HAND_BLOCKS
);
