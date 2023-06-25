use std::sync::Arc;
use crate::image_tasks::color::c;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{out_task, FileOutputTaskSpec, ToPixmapTaskSpec};
use crate::texture_base::material::Material;

macro_rules! dyes {
    ($($name:tt = $color:expr),+) => {
        $(pub const $name: (&str, ComparableColor) = (
            const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            $color
        );)+
        pub const DYES: &[(&str, ComparableColor)] = &[
            $($name),+
        ];
    }
}

dyes!(
    BLACK = ComparableColor::BLACK,
    RED = c(0xba0000),
    GREEN = c(0x007c00),
    BROWN = c(0x835400),
    BLUE = c(0x0000aa),
    PURPLE = c(0x8900b8),
    CYAN = c(0x009c9c),
    LIGHT_GRAY = ComparableColor::STONE_HIGHLIGHT,
    GRAY = ComparableColor::STONE_EXTREME_SHADOW,
    PINK = c(0xff9a9a),
    LIME = c(0x80ff00),
    YELLOW = ComparableColor::YELLOW,
    LIGHT_BLUE = c(0x7777ff),
    MAGENTA = c(0xff4eff),
    ORANGE = c(0xff8000),
    WHITE = ComparableColor::WHITE
);

pub struct DyedBlock<T = fn(ComparableColor) -> ToPixmapTaskSpec>
    where T: Fn(ComparableColor) -> ToPixmapTaskSpec {
    pub name: &'static str,
    pub create_dyed_texture: T
}

impl <T> Material for DyedBlock<T>
    where T: Fn(ComparableColor) -> ToPixmapTaskSpec {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        let mut out  = Vec::with_capacity(DYES.len());
        for (dye_name, dye_color) in DYES {
            out.push(out_task(&format!("block/{}_{}", dye_name, self.name),
                (self.create_dyed_texture)(*dye_color)
            ));
        }
        out.into()
    }
}

#[macro_export]
macro_rules! dyed_block {
    ($name:ident = $create_dyed_texture:expr) => {
        pub const $name: $crate::texture_base::dyes::DyedBlock = $crate::texture_base::dyes::DyedBlock {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            create_dyed_texture: |color| {
                macro_rules! color {
                    () => { color }
                }
                $create_dyed_texture
            }
        };
    }
}
