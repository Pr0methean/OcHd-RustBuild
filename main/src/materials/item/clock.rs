use crate::image_tasks::task_spec::{from_svg_task, out_task, FileOutputTaskSpec};
use crate::stack;
use crate::texture_base::material::Material;

const CLOCK_ANGLES: usize = 64;

pub struct Clock {}

impl Material for Clock {
    fn get_output_tasks(&self) -> Box<[FileOutputTaskSpec]> {
        let frame = from_svg_task("clockFrame");
        (0..CLOCK_ANGLES)
            .map(|angle| {
                out_task(
                    format!("item/clock_{:0>2}", angle),
                    stack!(
                        from_svg_task(format!("clockDial{}", angle)),
                        frame.to_owned()
                    ),
                )
            })
            .collect()
    }
}

pub const CLOCK: Clock = Clock {};
