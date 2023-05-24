use crate::image_tasks::task_spec::{FileOutputTaskSpec, from_svg_task, out_task};
use crate::stack;
use crate::texture_base::material::Material;

const CLOCK_ANGLES: usize = 64;

pub struct Clock {}

impl Material for Clock {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        let frame = from_svg_task("clockFrame");
        let mut output_tasks = Vec::with_capacity(CLOCK_ANGLES);
        for angle in 0..CLOCK_ANGLES {
            output_tasks.push(out_task(
                &format!("item/clock_{:0>2}", angle),
                stack!(
                    from_svg_task(&format!("clockDial{}", angle)),
                    frame.to_owned()
                )
            ));
        }
        output_tasks
    }
}

pub const CLOCK: Clock = Clock{};