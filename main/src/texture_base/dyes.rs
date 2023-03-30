use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::color::c;
use crate::image_tasks::task_spec::TaskSpec;
use crate::image_tasks::task_spec::TaskSpec::PngOutput;
use std::path::{PathBuf};
use std::sync::Arc;

pub static DYES: &'static [(&str, ComparableColor)] = &[
    ("black",       ComparableColor::BLACK),
    ("red",         c(0xb0, 0x00, 0x00)),
    ("green",       c(0x00, 0x7c, 0x00)),
    ("brown",       c(0x83, 0x54, 0x00)),
    ("blue",        c(0x00, 0x00, 0xaa)),
    ("purple",      c(0x89, 0x00, 0xb8)),
    ("cyan",        c(0x00, 0x9c, 0x9c)),
    ("light_gray",  c(0xaa, 0xaa, 0xaa)),
    ("gray",        c(0x51, 0x51, 0x51)),
    ("pink",        c(0xff, 0x9a, 0x9a)),
    ("lime",        c(0x80, 0xff, 0x00)),
    ("yellow",      c(0xff, 0xff, 0x00)),
    ("light_blue",  c(0x77, 0x77, 0xff)),
    ("magenta",     c(0xff, 0x4e, 0xff)),
    ("orange",      c(0xff, 0x80, 0x00)),
    ("white",       ComparableColor::WHITE)
];

pub fn dyed_block(name: String,
                  create_dyed_texture: Box<dyn Fn(String, ComparableColor) -> TaskSpec>)
        -> Vec<Arc<TaskSpec>>{
    let mut out: Vec<Arc<TaskSpec>> = vec!();
    for (dye_name, dye_color) in DYES {
        out.push(Arc::new(PngOutput {
            base: Arc::new(create_dyed_texture(dye_name.to_string(), *dye_color)),
            destinations: Arc::new(vec!(PathBuf::from(format!("blocks/{}_{}", dye_name, name))))
        }));
    }
    return out;
}
