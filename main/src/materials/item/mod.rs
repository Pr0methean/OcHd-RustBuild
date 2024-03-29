mod clock;
mod compass;
mod music_disc;
mod simple_items;

use crate::group;
use crate::materials::item::clock::CLOCK;
use crate::materials::item::compass::COMPASSES;
use crate::materials::item::music_disc::MUSIC_DISCS;
use crate::materials::item::simple_items::SIMPLE_ITEMS;

group!(ALL_ITEMS = COMPASSES, CLOCK, MUSIC_DISCS, SIMPLE_ITEMS);
