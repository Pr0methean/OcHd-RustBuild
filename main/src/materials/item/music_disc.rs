use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor};
use crate::image_tasks::task_spec::{FileOutputTaskSpec, out_task, paint_svg_task};
use crate::{group, single_texture_item, stack};
use crate::texture_base::dyes::DYES;
use crate::texture_base::material::{Material, MaterialGroup};

const DISC_NAMES_AND_COLORS: &[(&str, &str)] = &[
    ("far", "red"),
    ("wait", "green"),
    ("strad", "brown"),
    ("mall", "blue"),
    ("cat", "purple"),
    ("pigstep", "cyan"),
    ("mellohi", "light_gray"),
    ("13", "pink"),
    ("blocks", "lime"),
    ("stal", "yellow"),
    ("ward", "light_blue"),
    ("5", "magenta"),
    ("otherside", "orange"),
    ("chirp", "gray")
];

struct MusicDisc {
    name: &'static str,
    color: ComparableColor
}

impl Material for MusicDisc {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![out_task(&format!("item/music_disc_{}", self.name), stack!(
            paint_svg_task("musicDisc", ComparableColor::STONE_EXTREME_SHADOW),
            paint_svg_task("musicDiscGroove", ComparableColor::DARKEST_GRAY),
            paint_svg_task("musicDiscLabel", self.color)
        ))]
    }
}

single_texture_item!(MUSIC_DISC_11 =
    paint_svg_task("musicDiscBroken", ComparableColor::DARKEST_GRAY),
    paint_svg_task("musicDiscGrooveBroken", ComparableColor::STONE_EXTREME_SHADOW)
);

lazy_static! {
    static ref NORMAL_MUSIC_DISCS: MaterialGroup = MaterialGroup {
        tasks: DISC_NAMES_AND_COLORS.iter().map(
                |(name, color_name)| {
            for (dye_name, dye_color) in DYES {
                if dye_name == color_name {
                    return MusicDisc {
                        name,
                        color: dye_color.to_owned()
                    };
                }
            }
            panic!("No dye named {} found", color_name)
        }).flat_map(|disc| disc.get_output_tasks().into_iter()).collect()
    };
}

group!(MUSIC_DISCS = NORMAL_MUSIC_DISCS, MUSIC_DISC_11);
