use crate::image_tasks::color::{gray, rgb};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{out_task, SinkTaskSpec, ToPixmapTaskSpec};
use crate::texture_base::material::Material;

pub static DYES: &[(&str, ComparableColor)] = &[
    ("black",       ComparableColor::BLACK),
    ("red",         rgb(0xb0, 0x00, 0x00)),
    ("green",       rgb(0x00, 0x7c, 0x00)),
    ("brown",       rgb(0x83, 0x54, 0x00)),
    ("blue",        rgb(0x00, 0x00, 0xaa)),
    ("purple",      rgb(0x89, 0x00, 0xb8)),
    ("cyan",        rgb(0x00, 0x9c, 0x9c)),
    ("light_gray",  gray(0xaa)),
    ("gray",        gray(0x51)),
    ("pink",        rgb(0xff, 0x9a, 0x9a)),
    ("lime",        rgb(0x80, 0xff, 0x00)),
    ("yellow",      rgb(0xff, 0xff, 0x00)),
    ("light_blue",  rgb(0x77, 0x77, 0xff)),
    ("magenta",     rgb(0xff, 0x4e, 0xff)),
    ("orange",      rgb(0xff, 0x80, 0x00)),
    ("white",       ComparableColor::WHITE)
];

pub struct DyedBlock {
    pub name: &'static str,
    pub create_dyed_texture: Box<dyn Fn(ComparableColor) -> ToPixmapTaskSpec + Send + Sync>
}

impl Material for DyedBlock {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        let mut out  = Vec::with_capacity(DYES.len());
        for (dye_name, dye_color) in DYES {
            out.push(out_task(&*format!("block/{}_{}", dye_name, self.name),
                (self.create_dyed_texture)(*dye_color),
            ));
        }
        out
    }
}

#[macro_export]
macro_rules! dyed_block {
    ($name:ident = $create_dyed_texture:expr) => {
        lazy_static::lazy_static! {
            pub static ref $name: crate::texture_base::dyes::DyedBlock =
            crate::texture_base::dyes::DyedBlock {
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
