use lazy_static::lazy_static;

use crate::{copy_block, group};
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task};
use crate::materials::block::axe::wood::{BIRCH, DARK_OAK, OAK, SPRUCE};
use crate::materials::block::bare_hand::simple_bare_hand_block::{HONEYCOMB_BORDER, HONEYCOMB_CELLS};
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
copy_block!(CRAFTING_TABLE_FRONT = CRAFTING_TABLE_SIDE, "");
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
single_texture_block!(CHISELED_BOOKSHELF_EMPTY = OAK.color,
    from_svg_task("bookShelvesChiseledEmpty")
);
single_texture_block!(CHISELED_BOOKSHELF = OAK.color,
    from_svg_task("bookShelvesChiseled")
);
single_texture_block!(JUKEBOX_TOP = OAK.color,
    paint_svg_task("borderSolidThick", OAK.highlight),
    paint_svg_task("borderDotted", OAK.shadow),
    from_svg_task("thirdRail")
);
single_texture_block!(JUKEBOX_SIDE = OAK.highlight,
    paint_svg_task("borderSolidThick", OAK.color),
    paint_svg_task("borderSolid", OAK.highlight),
    paint_stack!(OAK.shadow, "strokeTopLeftBottomRight4", "strokeBottomLeftTopRight4xorBorder")
);
single_texture_block!(NOTE_BLOCK = ComparableColor::TRANSPARENT,
    JUKEBOX_SIDE.texture(),
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
single_texture_block!(BEEHIVE_END = ComparableColor::TRANSPARENT,
    BIRCH.grain(),
    paint_svg_task("honeycomb", BIRCH.shadow)
);
single_texture_block!(BEEHIVE_SIDE = ComparableColor::TRANSPARENT,
    BEEHIVE_END.texture(),
    paint_svg_task("topPart", BIRCH.shadow),
    paint_svg_task("topStripeThick", BIRCH.color)
);
single_texture_block!(BEEHIVE_FRONT = ComparableColor::TRANSPARENT,
    BEEHIVE_SIDE.texture(),
    from_svg_task("beehiveEntrance")
);
single_texture_block!(BEEHIVE_FRONT_HONEY = ComparableColor::TRANSPARENT,
    BEEHIVE_SIDE.texture(),
    paint_svg_task("beehiveEntrance", HONEYCOMB_BORDER)
);
single_texture_block!(BEE_NEST_SIDE = HONEYCOMB_BORDER,
    paint_svg_task("planksTopBorder", c(0x624831))
);
single_texture_block!(BEE_NEST_FRONT = HONEYCOMB_BORDER,
    paint_stack!(c(0x624831), "planksTopBorder", "honeycombBorder"),
    from_svg_task("honeycombNoHalfCells")
);
single_texture_block!(BEE_NEST_FRONT_HONEY = HONEYCOMB_BORDER,
    paint_stack!(c(0x624831), "planksTopBorder", "honeycombBorder"),
    paint_svg_task("honeycombNoHalfCells", ComparableColor::YELLOW)
);
single_texture_block!(BEE_NEST_TOP = HONEYCOMB_BORDER,
    paint_svg_task("honeycomb", c(0x624831)),
    paint_svg_task("ringsCentralBullseye", HONEYCOMB_CELLS * 0.5)
);
single_texture_block!(BEE_NEST_BOTTOM = BIRCH.color,
    paint_svg_task("honeycomb", BIRCH.shadow),
    paint_svg_task("ringsCentralBullseye", BIRCH.highlight * 0.5)
);
single_texture_block!(BARREL_SIDE = SPRUCE.color,
    paint_svg_task("planksTopBorderVertical", SPRUCE.shadow),
    from_svg_task("barrelSlats")
);
single_texture_block!(BARREL_BOTTOM = ComparableColor::TRANSPARENT,
    SPRUCE.planks(),
    paint_svg_task("borderSolidExtraThick", SPRUCE.shadow),
    paint_svg_task("borderSolid", SPRUCE.highlight)
);
single_texture_block!(BARREL_TOP = ComparableColor::TRANSPARENT,
    BARREL_BOTTOM.texture(),
    paint_svg_task("bigCircle", ComparableColor::WHITE * 0.25)
);
single_texture_block!(BARREL_TOP_OPEN = ComparableColor::TRANSPARENT,
    BARREL_BOTTOM.texture(),
    from_svg_task("bigCircle")
);

group!(SIMPLE_AXE_BLOCK = CRAFTING_TABLE_SIDE, CRAFTING_TABLE_TOP, CRAFTING_TABLE_FRONT,
    LADDER, BOOKSHELF, CHISELED_BOOKSHELF_EMPTY, CHISELED_BOOKSHELF,
    JUKEBOX_TOP, JUKEBOX_SIDE, NOTE_BLOCK,
    COMPOSTER_BOTTOM, COMPOSTER_TOP, COMPOSTER_SIDE,
    BEEHIVE_END, BEEHIVE_SIDE, BEEHIVE_FRONT, BEEHIVE_FRONT_HONEY,
    BEE_NEST_SIDE, BEE_NEST_FRONT, BEE_NEST_FRONT_HONEY, BEE_NEST_TOP, BEE_NEST_BOTTOM,
    BARREL_SIDE, BARREL_BOTTOM, BARREL_TOP, BARREL_TOP_OPEN);