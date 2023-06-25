use once_cell::sync::Lazy;
use crate::{group, material, single_texture_block, stack};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task, ToPixmapTaskSpec};

single_texture_block!(FURNACE_SIDE =
    ComparableColor::STONE,
    paint_svg_task("bottomHalf", ComparableColor::STONE_HIGHLIGHT),
    paint_svg_task("borderSolid", ComparableColor::STONE_EXTREME_SHADOW)
);
single_texture_block!(FURNACE_FRONT =
    ComparableColor::TRANSPARENT,
    FURNACE_SIDE.texture(),
    paint_svg_task("furnaceFrontLit", ComparableColor::BLACK)
);
single_texture_block!(FURNACE_FRONT_ON =
    ComparableColor::TRANSPARENT,
    FURNACE_SIDE.texture(),
    from_svg_task("furnaceFrontLit")
);
single_texture_block!(BLAST_FURNACE_TOP =
    ComparableColor::STONE_EXTREME_SHADOW,
    paint_svg_task("cornerCrosshairs", ComparableColor::STONE_EXTREME_HIGHLIGHT)
);
single_texture_block!(BLAST_FURNACE =
    ComparableColor::STONE_EXTREME_SHADOW,
    paint_svg_task("bottomHalf", ComparableColor::STONE),
    paint_svg_task("cornerCrosshairs", ComparableColor::STONE_EXTREME_HIGHLIGHT)
);

static BLAST_FURNACE_FRONT_BASE: Lazy<ToPixmapTaskSpec> = Lazy::new(|| stack!(
        BLAST_FURNACE.texture(),
        paint_svg_task("craftingGridSquare", ComparableColor::STONE_EXTREME_HIGHLIGHT)
    )
);

single_texture_block!(BLAST_FURNACE_FRONT =
    ComparableColor::TRANSPARENT,
    BLAST_FURNACE_FRONT_BASE.to_owned(),
    paint_svg_task("blastFurnaceHolesLit", ComparableColor::BLACK)
);

material!(BLAST_FURNACE_FRONT_ON = "block",
    ToPixmapTaskSpec::Animate {
        background: Box::new(BLAST_FURNACE_FRONT_BASE.to_owned()),
        frames: Box::new([
            from_svg_task("blastFurnaceHolesLit"),
            from_svg_task("blastFurnaceHolesLit1")
        ])
    }
);

group!(FURNACES = FURNACE_FRONT, FURNACE_FRONT_ON, FURNACE_SIDE, BLAST_FURNACE_FRONT,
        BLAST_FURNACE_FRONT_ON, BLAST_FURNACE, BLAST_FURNACE_TOP);