use crate::image_tasks::color::c;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{out_task, FileOutputTaskSpec, ToPixmapTaskSpec};
use crate::texture_base::material::Material;

pub static DYES: &[(&str, ComparableColor)] = &[
    ("black",       ComparableColor::BLACK),
    ("red",         c(0xba0000)),
    ("green",       c(0x007c00)),
    ("brown",       c(0x835400)),
    ("blue",        c(0x0000aa)),
    ("purple",      c(0x8900b8)),
    ("cyan",        c(0x009c9c)),
    ("light_gray",  ComparableColor::STONE_HIGHLIGHT),
    ("gray",        ComparableColor::STONE_EXTREME_SHADOW),
    ("pink",        c(0xff9a9a)),
    ("lime",        c(0x80ff00)),
    ("yellow",      ComparableColor::YELLOW),
    ("light_blue",  c(0x7777ff)),
    ("magenta",     c(0xff4eff)),
    ("orange",      c(0xff8000)),
    ("white",       ComparableColor::WHITE)
];

pub struct DyedBlock {
    pub name: &'static str,
    pub create_dyed_texture: Box<dyn Fn(ComparableColor) -> ToPixmapTaskSpec + Send + Sync>
}

impl Material for DyedBlock {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        let mut out  = Vec::with_capacity(DYES.len());
        for (dye_name, dye_color) in DYES {
            out.push(out_task(&format!("block/{}_{}", dye_name, self.name),
                (self.create_dyed_texture)(*dye_color)
            ));
        }
        out
    }
}

#[macro_export]
macro_rules! dyed_block {
    ($name:ident = $create_dyed_texture:expr) => {
        lazy_static::lazy_static! {
            pub static ref $name: $crate::texture_base::dyes::DyedBlock =
            $crate::texture_base::dyes::DyedBlock {
                name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
                create_dyed_texture: Box::new(|color| {
                    macro_rules! color {
                        () => { color }
                    }
                    $create_dyed_texture
                })
            };
        }
    }
}
