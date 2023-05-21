mod simple_items;
mod music_disc;
mod compass;

use crate::group;
use crate::materials::item::compass::COMPASSES;
use crate::materials::item::music_disc::MUSIC_DISCS;
use crate::materials::item::simple_items::{SIMPLE_ITEMS};

group!(ALL_ITEMS = SIMPLE_ITEMS, MUSIC_DISCS, COMPASSES);