use lazy_static::lazy_static;
use ordered_float::OrderedFloat;
use crate::{dyed_block, group, paint_stack, single_texture_block, stack_alpha};
use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task, paint_task, ToAlphaChannelTaskSpec};
use crate::texture_base::material::SingleLayerMaterial;

lazy_static! {
    static ref GLASS_PANE_TOP: SingleLayerMaterial = SingleLayerMaterial {
        name: "block/glass_pane_top",
        layer_name: "paneTop",
        color: c(0xa8d5d5)
    };

    static ref STAINED_GLASS_BASE: ToAlphaChannelTaskSpec = ToAlphaChannelTaskSpec::StackAlphaOnBackground {
        background: OrderedFloat(0.25),
        foreground: Box::new(stack_alpha!(
            from_svg_task("borderSolid"),
            from_svg_task("streaks")
        ))
    };
}

dyed_block!(STAINED_GLASS = paint_task(STAINED_GLASS_BASE.to_owned(), color!()));

dyed_block!(STAINED_GLASS_PANE_TOP = paint_task(from_svg_task("paneTop").into(), color!()));

single_texture_block!(GLASS =
    ComparableColor::TRANSPARENT,
    paint_svg_task("borderSolid", c(0x515151)),
    paint_stack!(ComparableColor::WHITE, "borderSolidTopLeft", "streaks")
);

single_texture_block!(TINTED_GLASS =
    ComparableColor::BLACK * 0.25,
    paint_stack!(ComparableColor::WHITE * 0.25, "borderSolid", "streaks")
);

group!(GLASS_VARIANTS = GLASS_PANE_TOP, GLASS, TINTED_GLASS, STAINED_GLASS_FRONT, STAINED_GLASS_TOP);
