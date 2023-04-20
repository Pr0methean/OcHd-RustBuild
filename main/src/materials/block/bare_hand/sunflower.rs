use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::{group, material, stack};
use crate::texture_base::material::DoubleTallBlock;

const STEM_COLOR: ComparableColor = c(0x4a8f28);
const STEM_SHADOW: ComparableColor = c(0x256325);
const STEM_HIGHLIGHT: ComparableColor = c(0x55ab2d);

lazy_static!{
    static ref SUNFLOWER_BASE: DoubleTallBlock = DoubleTallBlock {
        name: "sunflower",
        bottom: stack!(
            paint_svg_task("flowerStemTall", STEM_COLOR),
            paint_svg_task("flowerStemTallBorder", STEM_HIGHLIGHT),
            paint_svg_task("flowerStemBottomBorder", STEM_SHADOW)
        ),
        top: stack!(
            paint_svg_task("flowerStemShort", STEM_COLOR),
            paint_svg_task("flowerStemShortBorder", STEM_HIGHLIGHT),
            paint_svg_task("flowerStemBottomBorder", STEM_SHADOW)
        )
    };
}

material!(SUNFLOWER_BACK = "block", from_svg_task("sunflowerPetals"));
material!(SUNFLOWER_FRONT = "block", stack!(
    paint_svg_task("sunflowerPetals", ComparableColor::YELLOW),
    from_svg_task("sunflowerPistil")
));

group!(SUNFLOWER = SUNFLOWER_BASE, SUNFLOWER_BACK, SUNFLOWER_FRONT);