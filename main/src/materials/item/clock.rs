use std::sync::Arc;
use crate::image_tasks::task_spec::{FileOutputTaskSpec, from_svg_task, out_task};
use crate::stack;
use crate::texture_base::material::Material;

const CLOCK_ANGLES: usize = 64;

pub struct Clock {}

impl Material for Clock {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        let frame = from_svg_task("clockFrame");
        (0..CLOCK_ANGLES).map(|angle| {
            out_task(
                &format!("item/clock_{:0>2}", angle),
                stack!(
                    from_svg_task(&format!("clockDial{}", angle)),
                    frame.to_owned()
                )
            )
        }).collect()
    }
}

pub const CLOCK: Clock = Clock{};