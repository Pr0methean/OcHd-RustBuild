use lazy_static::lazy_static;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{FileOutputTaskSpec, from_svg_task, out_task, paint_svg_task};
use crate::{group, stack};
use crate::texture_base::material::Material;

const COMPASS_ANGLES: usize = 32;

pub struct Compass {
    rim_color: ComparableColor,
    dial_color: ComparableColor,
    needle_color: ComparableColor,
    base_name: &'static str
}

impl Material for Compass {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        let base = stack!(
            paint_svg_task("circle32", self.rim_color),
            paint_svg_task("circle28", self.dial_color),
            from_svg_task("compassRim")
        );
        let mut output_tasks = Vec::with_capacity(COMPASS_ANGLES);
        for angle in 0..COMPASS_ANGLES {
            output_tasks.push(out_task(
                &format!("item/{}_{:0>2}", self.base_name, angle),
                stack!(
                    base.to_owned(),
                    paint_svg_task(&format!("compass{}", angle), self.needle_color)
                )
            ));
        }
        output_tasks
    }
}

lazy_static!(
    pub static ref COMPASS: Compass = Compass {
        rim_color: ComparableColor::WHITE,
        dial_color: ComparableColor::DEEPSLATE_SHADOW,
        needle_color: ComparableColor::RED,
        base_name: "compass"
    };

    pub static ref RECOVERY_COMPASS: Compass = Compass {
        rim_color: ComparableColor::CYAN,
        dial_color: ComparableColor::BLACK,
        needle_color: ComparableColor::CYAN,
        base_name: "compass"
    };
);

group!(COMPASSES = COMPASS, RECOVERY_COMPASS);