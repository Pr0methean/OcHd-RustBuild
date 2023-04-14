use lazy_static::lazy_static;

use crate::{group};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::materials::block::axe::wood::{DARK_OAK, OAK};
use crate::paint_stack;
use crate::single_texture_block;
use crate::stack_on;
use crate::texture_base::material::SingleLayerMaterial;

single_texture_block!(CRAFTING_TABLE_SIDE = ComparableColor::TRANSPARENT,
    stack_on!(OAK.color,
        paint_svg_task("waves2", OAK.highlight),
        paint_stack!(OAK.shadow, "waves", "planksTopBorder")
    ),
    paint_svg_task("borderSolid", OAK.highlight),
    paint_svg_task("craftingSide", DARK_OAK.color)
);
single_texture_block!(CRAFTING_TABLE_TOP = OAK.color,
    paint_svg_task("waves", OAK.highlight),
    paint_svg_task("waves2", OAK.shadow * 0.5),
    paint_svg_task("craftingGridSquare", OAK.shadow),
    paint_svg_task("craftingGridSpaces", OAK.color),
    paint_svg_task("borderSolid", DARK_OAK.color),
    paint_svg_task("cornersTri", OAK.highlight)
);
single_texture_block!(LADDER = ComparableColor::TRANSPARENT,
    paint_svg_task("rail", OAK.color),
    paint_svg_task("railTies", OAK.highlight)
);
single_texture_block!(BOOKSHELF = OAK.color,
    from_svg_task("bookShelves")
);
single_texture_block!(JUKEBOX_TOP = OAK.color,
    paint_svg_task("borderSolidThick", OAK.highlight),
    paint_svg_task("borderDotted", OAK.shadow),
    from_svg_task("thirdRail")
);
single_texture_block!(JUKEBOX_SIDE = OAK.highlight,
    paint_stack!(OAK.shadow, "strokeTopLeftBottomRight4", "strokeBottomLeftTopRight4"),
    paint_svg_task("borderSolidThick", OAK.color),
    paint_svg_task("borderSolid", OAK.highlight),
    paint_svg_task("borderDotted", OAK.shadow)
);
single_texture_block!(NOTE_BLOCK = ComparableColor::TRANSPARENT,
    JUKEBOX_SIDE.texture.to_owned(),
    paint_svg_task("note", DARK_OAK.shadow)
);
// Compost textures are part of DirtGroundCover.PODZOL
single_texture_block!(COMPOSTER_BOTTOM = OAK.shadow,
    paint_svg_task("planksTopVertical", OAK.color),
    paint_svg_task("borderSolidThick", OAK.shadow),
    paint_svg_task("borderSolid", OAK.color)
);
lazy_static! {pub static ref COMPOSTER_TOP: SingleLayerMaterial = SingleLayerMaterial{
    name: "block/composter_top",
    layer_name: "borderSolidThick",
    color: OAK.color
};}
single_texture_block!(COMPOSTER_SIDE = OAK.color,
    paint_svg_task("stripesThick", OAK.shadow),
    paint_svg_task("borderDotted", OAK.highlight)
);
group!(SIMPLE_AXE_BLOCK = CRAFTING_TABLE_SIDE, CRAFTING_TABLE_TOP, LADDER, BOOKSHELF, JUKEBOX_TOP,
        JUKEBOX_SIDE, NOTE_BLOCK, COMPOSTER_BOTTOM, COMPOSTER_TOP, COMPOSTER_SIDE);