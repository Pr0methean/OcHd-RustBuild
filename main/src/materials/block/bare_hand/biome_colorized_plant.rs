use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::paint_svg_task;
use crate::{group, make_tricolor_block_macro};

make_tricolor_block_macro!(
    biome_colorized,
    ComparableColor::MEDIUM_BIOME_COLORABLE,
    ComparableColor::DARK_BIOME_COLORABLE,
    ComparableColor::LIGHT_BIOME_COLORABLE
);

biome_colorized!(
    LILY_PAD = ComparableColor::TRANSPARENT,
    paint_svg_task("lilyPad", shadow!()),
    paint_svg_task("lilyPadInterior", highlight!())
);

biome_colorized!(
    TALL_GRASS = ComparableColor::TRANSPARENT,
    paint_svg_task("bottomHalf", shadow!()),
    paint_svg_task("grassTall", color!())
);

biome_colorized!(
    TALL_GRASS_TOP = ComparableColor::TRANSPARENT,
    paint_svg_task("grassVeryShort", color!())
);

biome_colorized!(
    GRASS = ComparableColor::TRANSPARENT,
    paint_svg_task("grassShort", color!())
);

group!(
    BIOME_COLORIZED = LILY_PAD,
    TALL_GRASS,
    TALL_GRASS_TOP,
    GRASS
);
