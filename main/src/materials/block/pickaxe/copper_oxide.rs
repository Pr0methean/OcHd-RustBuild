use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{out_task, paint_svg_task, FileOutputTaskSpec};
use crate::texture_base::material::Material;
use crate::{group, paint_stack, stack, stack_on};

struct CopperOxide {
    name: &'static str,
    texture_name: &'static str,
    color: ComparableColor,
    shadow: ComparableColor,
    highlight: ComparableColor,
}

impl Material for CopperOxide {
    fn get_output_tasks(&self) -> Box<[FileOutputTaskSpec]> {
        let shared_layers = stack_on!(
            self.color,
            paint_svg_task("borderSolid", self.shadow),
            paint_stack!(self.highlight, "streaks", "borderSolidTopLeft")
        );
        Box::new([
            out_task(
                format!("block/{}_copper", self.name),
                stack!(
                    shared_layers.to_owned(),
                    paint_svg_task(self.texture_name, self.shadow)
                ),
            ),
            out_task(
                format!("block/cut_{}_copper", self.name),
                stack!(
                    shared_layers,
                    paint_svg_task("cutInQuarters2", self.highlight),
                    paint_svg_task("cutInQuarters1", self.shadow)
                ),
            ),
        ])
    }
}

const EXPOSED_COPPER: CopperOxide = CopperOxide {
    name: "exposed",
    texture_name: "copper2oxideOneThird",
    color: c(0xa87762),
    shadow: c(0x795B4B),
    highlight: c(0xce8888),
};
const WEATHERED_COPPER: CopperOxide = CopperOxide {
    name: "weathered",
    texture_name: "copper2oxideTwoThirds",
    color: c(0x64a077),
    shadow: c(0x647147),
    highlight: c(0x74BE9C),
};
const OXIDIZED_COPPER: CopperOxide = CopperOxide {
    name: "oxidized",
    texture_name: "copper2oxide",
    color: c(0x4fab90),
    shadow: c(0x3b5c5c),
    highlight: c(0x74BE9C),
};

group!(
    COPPER_OXIDES = EXPOSED_COPPER,
    WEATHERED_COPPER,
    OXIDIZED_COPPER
);
