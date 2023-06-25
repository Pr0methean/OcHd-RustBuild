use std::sync::Arc;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{FileOutputTaskSpec, out_task, paint_svg_task, ToPixmapTaskSpec};
use crate::materials::block::axe::wood::{CRIMSON_LEAVES_HIGHLIGHT, CRIMSON_LEAVES_SHADOW};
use crate::{group, stack};
use crate::texture_base::material::{Material};

const VEG_LEAVES_SHADOW: ComparableColor = c(0x256325);
const VEG_LEAVES_HIGHLIGHT: ComparableColor = c(0x55ff2d);

pub struct Crop<T = fn(u8) -> ToPixmapTaskSpec, U = fn() -> ToPixmapTaskSpec>
where T: Fn(u8) -> ToPixmapTaskSpec, U: Fn() -> ToPixmapTaskSpec
{
    name: &'static str,
    stages: u8,
    color: ComparableColor,
    create_texture_for_growing_stage: T,
    create_texture_for_final_stage: U
}

impl Material for Crop {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        let mut output = Vec::with_capacity(self.stages as usize);
        for stage in 0..(self.stages - 1) {
            output.push(out_task(
                &format!("block/{}_stage{}", self.name, stage),
                (self.create_texture_for_growing_stage)(stage)
            ));
        }
        output.push(out_task(
            &format!("block/{}_stage{}", self.name, self.stages - 1),
            (self.create_texture_for_final_stage)()
        ));
        output.into()
    }
}

fn basic_texture_for_growing_stage(name: &str, stage: u8) -> ToPixmapTaskSpec {
    paint_svg_task(&format!("{}{}", name, stage), VEG_LEAVES_SHADOW)
}

fn root_veg_texture_for_final_stage(crop: &Crop) -> ToPixmapTaskSpec {
    stack!(
        paint_svg_task(
            &format!("{}{}Stems", crop.name, crop.stages - 1), VEG_LEAVES_HIGHLIGHT),
        paint_svg_task("rootVeg", crop.color)
    )
}

pub const NETHER_WART: Crop = Crop {
    name: "nether_wart",
    stages: 3,
    color: CRIMSON_LEAVES_SHADOW,
    create_texture_for_growing_stage: |stage|
        paint_svg_task(&format!("wart{}", stage), NETHER_WART.color)
    ,
    create_texture_for_final_stage: || stack!(
        paint_svg_task("wart2", NETHER_WART.color),
        paint_svg_task("wart2a", CRIMSON_LEAVES_HIGHLIGHT)
    )
};
pub const CARROTS: Crop = Crop {
    name: "carrots",
    stages: 4,
    color: c(0xff8000),
    create_texture_for_growing_stage: |stage| basic_texture_for_growing_stage("carrots", stage),
    create_texture_for_final_stage: || root_veg_texture_for_final_stage(&CARROTS)
};
pub const BEETROOTS: Crop = Crop {
    name: "beetroots",
    stages: 4,
    color: c(0xbf2727),
    create_texture_for_growing_stage: |stage| basic_texture_for_growing_stage("beetroots", stage),
    create_texture_for_final_stage: || root_veg_texture_for_final_stage(&BEETROOTS)
};
pub const POTATOES: Crop = Crop {
    name: "potatoes",
    stages: 4,
    color: c(0xd97b30),
    create_texture_for_growing_stage: |stage| basic_texture_for_growing_stage("potatoes", stage),
    create_texture_for_final_stage: || {
        stack!(
            paint_svg_task("flowerStemShort", VEG_LEAVES_HIGHLIGHT),
            paint_svg_task("potato", POTATOES.color)
        )
    }
};
pub const WHEAT: Crop = Crop {
    name: "wheat",
    stages: 8,
    color: c(0x888836),
    create_texture_for_growing_stage: |stage| {
        stack!(
            paint_svg_task(&format!("wheat{}", stage), c(0x636300)),
            paint_svg_task(&format!("wheatTexture{}", stage), WHEAT.color)
        )
    },
    create_texture_for_final_stage: || stack!(
        paint_svg_task("wheat7", c(0xdcbb65)),
        paint_svg_task("wheatTexture7", WHEAT.color)
    )
};

group!(CROPS = NETHER_WART, CARROTS, BEETROOTS, POTATOES, WHEAT);
