use lazy_static::lazy_static;
use crate::image_tasks::color::{ComparableColor, c};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::{block_with_colors, group, paint_stack};
use crate::texture_base::material::{SingleTextureTricolorMaterial};
block_with_colors!(STONE = ComparableColor::STONE, ComparableColor::STONE_SHADOW, ComparableColor::STONE_HIGHLIGHT,
    color!(),
    paint_svg_task("checksQuarterCircles", highlight!()),
    paint_svg_task("checksQuarterCircles2", shadow!()),
    paint_svg_task("circle24", color!() * 0.75)
);

block_with_colors!(DEEPSLATE = ComparableColor::DEEPSLATE, ComparableColor::DEEPSLATE_SHADOW,
        ComparableColor::DEEPSLATE_HIGHLIGHT,
    color!(),
    paint_svg_task("diagonalChecksBottomLeftTopRight", highlight!()),
    paint_svg_task("diagonalChecksTopLeftBottomRight", shadow!())
);

block_with_colors!(NETHERRACK = c(0x723232), c(0x500000), c(0x854242),
    color!(),
    paint_svg_task("diagonalChecksTopLeftBottomRight", shadow!()),
    paint_svg_task("diagonalChecksBottomLeftTopRight", highlight!()),
    paint_stack!(color!(), "diagonalChecksFillTopLeftBottomRight",
            "diagonalChecksFillBottomLeftTopRight")
);

group!(ORE_BASES = STONE, DEEPSLATE, NETHERRACK);

pub struct OreBase {
    pub block_name_prefix: &'static str,
    pub material: &'static SingleTextureTricolorMaterial,
}
lazy_static! {
    pub static ref STONE_BASE: OreBase = OreBase {block_name_prefix: "", material: &STONE };
    pub static ref DEEPSLATE_BASE: OreBase = OreBase {block_name_prefix: "deepslate_", material: &DEEPSLATE};
    pub static ref NETHERRACK_BASE: OreBase = OreBase {block_name_prefix: "nether_", material: &NETHERRACK};
}
