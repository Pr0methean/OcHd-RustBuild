use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::{group, paint_stack, single_texture_block};
use crate::texture_base::material::SingleTextureMaterial;
single_texture_block!(STONE = ComparableColor::STONE,
    paint_svg_task("checksQuarterCircles", ComparableColor::STONE_HIGHLIGHT),
    paint_svg_task("checksQuarterCircles2", ComparableColor::STONE_SHADOW),
    paint_svg_task("bigCircle", ComparableColor::STONE)
);

single_texture_block!(DEEPSLATE = ComparableColor::DEEPSLATE,
    paint_svg_task("diagonalChecksBottomLeftTopRight", ComparableColor::DEEPSLATE_HIGHLIGHT),
    paint_svg_task("diagonalChecksTopLeftBottomRight", ComparableColor::DEEPSLATE_SHADOW)
);

const NETHERRACK_COLOR: ComparableColor = c(0x723232);
single_texture_block!(NETHERRACK = NETHERRACK_COLOR,
    paint_svg_task("diagonalChecksTopLeftBottomRight", c(0x500000)),
    paint_svg_task("diagonalChecksBottomLeftTopRight", c(0x854242)),
    paint_stack!(NETHERRACK_COLOR, "diagonalChecksFillTopLeftBottomRight",
            "diagonalChecksFillBottomLeftTopRight")
);

group!(ORE_BASES = STONE, DEEPSLATE, NETHERRACK);

pub struct OreBase {
    pub block_name_prefix: &'static str,
    pub material: &'static SingleTextureMaterial,
}
lazy_static! {
    pub static ref STONE_BASE: OreBase = OreBase {block_name_prefix: "", material: &STONE };
    pub static ref DEEPSLATE_BASE: OreBase = OreBase {block_name_prefix: "deepslate_", material: &DEEPSLATE};
    pub static ref NETHERRACK_BASE: OreBase = OreBase {block_name_prefix: "nether_", material: &NETHERRACK};
}
