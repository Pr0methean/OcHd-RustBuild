use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task, ToPixmapTaskSpec};
use crate::{group, material, single_texture_block, stack};

const VINE_SHADOW: ComparableColor = c(0x4f3200);
const VINE_HIGHLIGHT: ComparableColor = c(0x70922d);

lazy_static! {
    static ref VINE_PLANT_TASK: ToPixmapTaskSpec = stack!(
        paint_svg_task("wavyVines", VINE_SHADOW),
        paint_svg_task("waves", VINE_HIGHLIGHT)
    );
    static ref VINE_TASK: ToPixmapTaskSpec = stack!(
        paint_svg_task("wavyVinesBottom", VINE_SHADOW),
        paint_svg_task("wavesBottom", VINE_HIGHLIGHT)
    );
}

material!(CAVE_VINES_PLANT = "block", VINE_PLANT_TASK.to_owned());

single_texture_block!(CAVE_VINES_PLANT_LIT = ComparableColor::TRANSPARENT,
    VINE_PLANT_TASK.to_owned(),
    from_svg_task("vineBerries")
);

material!(CAVE_VINES = "block", VINE_TASK.to_owned());

single_texture_block!(CAVE_VINES_LIT = ComparableColor::TRANSPARENT,
    VINE_TASK.to_owned(),
    from_svg_task("vineBerries")
);

group!(CAVE_VINE_VARIANTS = CAVE_VINES, CAVE_VINES_LIT, CAVE_VINES_PLANT, CAVE_VINES_PLANT_LIT);