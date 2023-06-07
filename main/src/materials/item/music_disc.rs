use crate::image_tasks::color::{ComparableColor};
use crate::image_tasks::task_spec::{FileOutputTaskSpec, out_task, paint_svg_task};
use crate::{group, single_texture_item, stack};
use crate::texture_base::dyes::*;
use crate::texture_base::material::{Material};

macro_rules! discs {
    ($($name:ident = $dye:expr),+) => {
        $(pub const $name: MusicDisc = MusicDisc {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            color: &$dye.1
        };)+
        crate::group!(NORMAL_MUSIC_DISCS = $($name),+);
    }
}

pub struct MusicDisc {
    name: &'static str,
    color: &'static ComparableColor
}

discs!(
    FAR = RED,
    WAIT = GREEN,
    STRAD = BROWN,
    MALL = BLUE,
    CAT = PURPLE,
    PIGSTEP = CYAN,
    MELLOHI = LIGHT_GRAY,
    BLOCKS = LIME,
    STAL = YELLOW,
    WARD = LIGHT_BLUE,
    OTHERSIDE = ORANGE,
    CHIRP = GRAY
);

pub const MUSIC_DISC_13: MusicDisc = MusicDisc {name: "13", color: &PINK.1 };
pub const MUSIC_DISC_5: MusicDisc = MusicDisc {name: "5", color: &MAGENTA.1 };

impl Material for MusicDisc {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![out_task(&format!("item/music_disc_{}", self.name), stack!(
            paint_svg_task("musicDisc", ComparableColor::STONE_EXTREME_SHADOW),
            paint_svg_task("musicDiscGroove", ComparableColor::DARKEST_GRAY),
            paint_svg_task("musicDiscLabel", *self.color)
        ))]
    }
}

single_texture_item!(MUSIC_DISC_11 =
    paint_svg_task("musicDiscBroken", ComparableColor::DARKEST_GRAY),
    paint_svg_task("musicDiscGrooveBroken", ComparableColor::STONE_EXTREME_SHADOW)
);

group!(MUSIC_DISCS = NORMAL_MUSIC_DISCS, MUSIC_DISC_13, MUSIC_DISC_5, MUSIC_DISC_11);