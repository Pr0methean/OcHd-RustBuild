use crate::image_tasks::color::ComparableColor;
use crate::{group, single_layer_particle};

single_layer_particle!(NOTE = "note", ComparableColor::STONE_EXTREME_HIGHLIGHT);

group!(SIMPLE_PARTICLES = NOTE);