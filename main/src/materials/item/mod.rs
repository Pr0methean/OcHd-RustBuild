mod remains;
mod music_disc;
mod amethyst;

use crate::group;
use crate::materials::item::amethyst::AMETHYST_SHARD;
use crate::materials::item::music_disc::MUSIC_DISCS;
use crate::materials::item::remains::REMAINS;

group!(ALL_ITEMS = REMAINS, MUSIC_DISCS, AMETHYST_SHARD);