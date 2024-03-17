use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::texture_base::material::DoubleTallBlock;
use crate::{group, material, stack};
use once_cell::sync::Lazy;

const STEM_COLOR: ComparableColor = c(0x4a8f28);
const STEM_SHADOW: ComparableColor = c(0x256325);
const STEM_HIGHLIGHT: ComparableColor = c(0x55ab2d);

static SUNFLOWER_BASE: Lazy<DoubleTallBlock> = Lazy::new(|| DoubleTallBlock {
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
    ),
});

material!(SUNFLOWER_BACK = "block", from_svg_task("sunflowerPetals"));
material!(
    SUNFLOWER_FRONT = "block",
    stack!(
        paint_svg_task("sunflowerPetals", ComparableColor::YELLOW),
        from_svg_task("sunflowerPistil")
    )
);

group!(SUNFLOWER = SUNFLOWER_BASE, SUNFLOWER_BACK, SUNFLOWER_FRONT);
