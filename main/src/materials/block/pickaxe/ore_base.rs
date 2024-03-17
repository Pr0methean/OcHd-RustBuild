use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::paint_svg_task;
use crate::texture_base::material::SingleTextureTricolorMaterial;
use crate::{block_with_colors, group, paint_stack};
use once_cell::sync::Lazy;
block_with_colors!(
    STONE = ComparableColor::STONE,
    ComparableColor::STONE_SHADOW,
    ComparableColor::STONE_HIGHLIGHT,
    color!(),
    paint_svg_task("checksQuarterCircles", highlight!()),
    paint_svg_task("checksQuarterCircles2", shadow!()),
    paint_svg_task("circle24", color!() * 0.75)
);

block_with_colors!(
    DEEPSLATE = ComparableColor::DEEPSLATE,
    ComparableColor::DEEPSLATE_SHADOW,
    ComparableColor::DEEPSLATE_HIGHLIGHT,
    color!(),
    paint_svg_task("diagonalChecksBottomLeftTopRight", highlight!()),
    paint_svg_task("diagonalChecksTopLeftBottomRight", shadow!())
);

block_with_colors!(
    NETHERRACK = c(0x723232),
    c(0x500000),
    c(0x854242),
    color!(),
    paint_svg_task("diagonalChecksTopLeftBottomRight", shadow!()),
    paint_svg_task("diagonalChecksBottomLeftTopRight", highlight!()),
    paint_stack!(
        color!(),
        "diagonalChecksFillTopLeftBottomRight",
        "diagonalChecksFillBottomLeftTopRight"
    )
);

group!(ORE_BASES = STONE, DEEPSLATE, NETHERRACK);

#[derive(Clone)]
pub struct OreBase {
    pub block_name_prefix: &'static str,
    pub material: SingleTextureTricolorMaterial,
}
pub static STONE_BASE: Lazy<OreBase> = Lazy::new(|| OreBase {
    block_name_prefix: "",
    material: STONE.to_owned(),
});
pub static DEEPSLATE_BASE: Lazy<OreBase> = Lazy::new(|| OreBase {
    block_name_prefix: "deepslate_",
    material: DEEPSLATE.to_owned(),
});
pub static NETHERRACK_BASE: Lazy<OreBase> = Lazy::new(|| OreBase {
    block_name_prefix: "nether_",
    material: NETHERRACK.to_owned(),
});
