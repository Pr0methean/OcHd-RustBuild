// This file includes both the structure and jigsaw blocks since they're visually similar.
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{paint_svg_task};
use crate::{group, make_tricolor_block_macro};

make_tricolor_block_macro!(sj, c(0xb493b4), c(0x26002a), c(0xd7c2d7));

sj!(JIGSAW_BOTTOM = shadow!(),
        paint_svg_task("borderDotted", color!()));

macro_rules! sjs {
    ($name:ident = $layer_name:expr) => {
        sj!($name =
            ComparableColor::TRANSPARENT,
            JIGSAW_BOTTOM.material.texture.to_owned(),
            paint_svg_task($layer_name, highlight!())
        );
    }
}

sjs!(JIGSAW_TOP = "jigsaw");
sjs!(JIGSAW_SIDE = "arrowUp");
sjs!(JIGSAW_LOCK = "jigsawLock");
sjs!(STRUCTURE_BLOCK = "bigCircle");
sjs!(STRUCTURE_BLOCK_CORNER = "cornerCrosshairs");
sjs!(STRUCTURE_BLOCK_DATA = "data");
sj!(STRUCTURE_BLOCK_LOAD =
    ComparableColor::TRANSPARENT,
    JIGSAW_BOTTOM.material.texture.to_owned(),
    paint_svg_task("folder", color!()),
    paint_svg_task("loadArrow", color!())
);
sj!(STRUCTURE_BLOCK_SAVE =
    ComparableColor::TRANSPARENT,
    JIGSAW_BOTTOM.material.texture.to_owned(),
    paint_svg_task("folder", color!()),
    paint_svg_task("saveArrow", color!())
);

group!(JIGSAW_BLOCKS = JIGSAW_BOTTOM, JIGSAW_TOP, JIGSAW_SIDE, JIGSAW_LOCK);
group!(STRUCTURE_BLOCKS = STRUCTURE_BLOCK, STRUCTURE_BLOCK_CORNER, STRUCTURE_BLOCK_DATA,
        STRUCTURE_BLOCK_LOAD, STRUCTURE_BLOCK_SAVE);
