use crate::{group, single_layer_item};
use crate::image_tasks::color::c;

single_layer_item!(BONE = "boneBottomLeftTopRight", c(0xeaead0));
single_layer_item!(BONE_MEAL = "bonemealSmall");

// TODO: Rotten flesh

group!(REMAINS = BONE, BONE_MEAL);