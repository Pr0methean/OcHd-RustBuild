use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{
    out_task, paint_svg_task, FileOutputTaskSpec, ToPixmapTaskSpec,
};
use crate::materials::block::pickaxe::ore::GOLD;
use crate::texture_base::material::{ColorTriad, Material, TricolorMaterial};
use crate::{block_with_colors, group, paint_stack, single_texture_block, stack};

pub struct PolishableBlock {
    pub name: &'static str,
    pub colors: ColorTriad,
    texture: ToPixmapTaskSpec,
}

impl PolishableBlock {
    pub fn texture(&self) -> ToPixmapTaskSpec {
        self.texture.to_owned()
    }

    fn polished_texture(&self) -> ToPixmapTaskSpec {
        stack!(
            self.texture(),
            paint_svg_task("borderSolid", self.colors.shadow),
            paint_svg_task("borderSolidTopLeft", self.colors.highlight)
        )
    }
}

impl Material for PolishableBlock {
    fn get_output_tasks(&self) -> Box<[FileOutputTaskSpec]> {
        Box::new([
            out_task(format!("block/{}", self.name), self.texture()),
            out_task(
                format!("block/polished_{}", self.name),
                self.polished_texture(),
            ),
        ])
    }
}

impl TricolorMaterial for PolishableBlock {
    fn color(&self) -> ComparableColor {
        self.colors.color
    }

    fn shadow(&self) -> ComparableColor {
        self.colors.shadow
    }

    fn highlight(&self) -> ComparableColor {
        self.colors.highlight
    }
}

macro_rules! polishable {
    ($name:ident = $color:expr, $shadow:expr, $highlight:expr, $background:expr, $( $layers:expr ),* ) => {
        macro_rules! color {
            () => { $color }
        }
        macro_rules! shadow {
            () => { $shadow }
        }
        macro_rules! highlight {
            () => { $highlight }
        }
        pub static $name: once_cell::sync::Lazy<PolishableBlock> = once_cell::sync::Lazy::new(||
        PolishableBlock {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            colors: crate::texture_base::material::ColorTriad {
                color: color!(),
                shadow: shadow!(),
                highlight: highlight!()
            },
            texture: crate::stack_on!($background, $($layers),*).into()
        });
    }
}

polishable!(
    ANDESITE = ComparableColor::STONE,
    ComparableColor::STONE_SHADOW,
    ComparableColor::STONE_HIGHLIGHT,
    color!(),
    paint_svg_task("bigRingsBottomLeftTopRight", highlight!()),
    paint_svg_task("bigRingsTopLeftBottomRight", shadow!())
);

polishable!(
    DIORITE = ComparableColor::STONE_EXTREME_HIGHLIGHT,
    ComparableColor::STONE,
    ComparableColor::WHITE,
    color!(),
    paint_svg_task("bigRingsBottomLeftTopRight", shadow!()),
    paint_svg_task("bigRingsTopLeftBottomRight", highlight!())
);

polishable!(
    GRANITE = c(0x9f6b58),
    c(0x624033),
    c(0xFFCDB2),
    color!(),
    paint_svg_task("bigDotsBottomLeftTopRight", shadow!()),
    paint_stack!(
        highlight!(),
        "bigDotsTopLeftBottomRight",
        "bigDotsFillBottomLeftTopRight"
    ),
    paint_svg_task("bigDotsFillTopLeftBottomRight", shadow!())
);

polishable!(
    BLACKSTONE = c(0x2e2e36),
    ComparableColor::BLACK,
    ComparableColor::STONE_EXTREME_SHADOW,
    shadow!(),
    paint_svg_task("bigDotsBottomLeftTopRight", highlight!()),
    paint_svg_task("bigDotsTopLeftBottomRight", color!())
);

single_texture_block!(
    GILDED_BLACKSTONE = ComparableColor::TRANSPARENT,
    BLACKSTONE.polished_texture(),
    paint_svg_task("bigRingsBottomLeftTopRight", GOLD.color())
);

block_with_colors!(
    BLACKSTONE_TOP = ComparableColor::DEEPSLATE_SHADOW,
    ComparableColor::BLACK,
    ComparableColor::STONE_EXTREME_SHADOW,
    shadow!(),
    paint_svg_task("bigRingsBottomLeftTopRight", color!()),
    paint_svg_task("bigRingsTopLeftBottomRight", highlight!())
);

group!(
    POLISHABLE = ANDESITE,
    DIORITE,
    GRANITE,
    BLACKSTONE,
    GILDED_BLACKSTONE,
    BLACKSTONE_TOP
);
