use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{FileOutputTaskSpec, out_task, paint_svg_task, ToPixmapTaskSpec};
use crate::materials::block::axe::wood::CRIMSON;
use crate::{group, stack};
use crate::texture_base::material::{Material, TextureSupplier};

const VEG_LEAVES_SHADOW: ComparableColor = c(0x256325);
const VEG_LEAVES_HIGHLIGHT: ComparableColor = c(0x55ff2d);

type CropTextureSupplier = Box<dyn Fn(&Crop, u8) -> ToPixmapTaskSpec + Send + Sync>;

struct Crop {
    name: &'static str,
    stages: u8,
    color: ComparableColor,
    create_texture_for_growing_stage: CropTextureSupplier,
    create_texture_for_final_stage: TextureSupplier<Crop>
}

impl Material for Crop {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        let mut output = Vec::with_capacity(self.stages as usize);
        for stage in 0..(self.stages - 1) {
            output.push(out_task(
                &format!("block/{}_stage{}", self.name, stage),
                (self.create_texture_for_growing_stage)(self, stage)
            ));
        }
        output.push(out_task(
            &format!("block/{}_stage{}", self.name, self.stages - 1),
            (self.create_texture_for_final_stage)(self)
        ));
        output
    }
}

fn basic_texture_for_growing_stage(crop: &Crop, stage: u8) -> ToPixmapTaskSpec {
    paint_svg_task(&format!("{}{}", crop.name, stage), VEG_LEAVES_SHADOW)
}

fn root_veg_texture_for_final_stage(crop: &Crop) -> ToPixmapTaskSpec {
    stack!(
        paint_svg_task(
            &format!("{}{}Stems", crop.name, crop.stages - 1), VEG_LEAVES_HIGHLIGHT),
        paint_svg_task("rootVeg", crop.color)
    )
}

lazy_static!{
    static ref NETHER_WART: Crop = Crop {
        name: "nether_wart",
        stages: 3,
        color: CRIMSON.leaves_shadow,
        create_texture_for_growing_stage: Box::new(|crop, stage| {
            paint_svg_task(&format!("wart{}", stage), crop.color)
        }),
        create_texture_for_final_stage: Box::new(|crop| {
            stack!(
                paint_svg_task("wart2", crop.color),
                paint_svg_task("wart2a", CRIMSON.leaves_highlight)
            )
        })
    };
    static ref CARROTS: Crop = Crop {
        name: "carrots",
        stages: 4,
        color: c(0xff8000),
        create_texture_for_growing_stage: Box::new(basic_texture_for_growing_stage),
        create_texture_for_final_stage: Box::new(root_veg_texture_for_final_stage)
    };
    static ref BEETROOTS: Crop = Crop {
        name: "beetroots",
        stages: 4,
        color: c(0xbf2727),
        create_texture_for_growing_stage: Box::new(basic_texture_for_growing_stage),
        create_texture_for_final_stage: Box::new(root_veg_texture_for_final_stage)
    };
    static ref POTATOES: Crop = Crop {
        name: "potatoes",
        stages: 4,
        color: c(0xd97b30),
        create_texture_for_growing_stage: Box::new(basic_texture_for_growing_stage),
        create_texture_for_final_stage: Box::new(|crop| {
            stack!(
                paint_svg_task("flowerStemShort", VEG_LEAVES_HIGHLIGHT),
                paint_svg_task("potato", crop.color)
            )
        })
    };
    static ref WHEAT: Crop = Crop {
        name: "wheat",
        stages: 8,
        color: c(0x888836),
        create_texture_for_growing_stage: Box::new(|crop, stage| {
            stack!(
                paint_svg_task(&format!("wheat{}", stage), c(0x636300)),
                paint_svg_task(&format!("wheatTexture{}", stage), crop.color)
            )
        }),
        create_texture_for_final_stage: Box::new(|crop| {
            stack!(
                paint_svg_task("wheat7", c(0xdcbb65)),
                paint_svg_task("wheatTexture7", crop.color)
            )
        })
    };
}

group!(CROPS = NETHER_WART, CARROTS, BEETROOTS, POTATOES, WHEAT);
