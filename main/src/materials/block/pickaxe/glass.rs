use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{
    from_svg_task, paint_svg_task, paint_task, stack_alpha, svg_alpha_task, ToAlphaChannelTaskSpec,
};
use crate::texture_base::material::SingleLayerMaterial;
use crate::{dyed_block, group, paint_stack, single_texture_block};
use once_cell::sync::Lazy;

const GLASS_PANE_TOP: SingleLayerMaterial = SingleLayerMaterial {
    name: "block/glass_pane_top",
    layer_name: "paneTop",
    color: Some(c(0xa8d5d5)),
};

static STAINED_GLASS_BASE: Lazy<ToAlphaChannelTaskSpec> =
    Lazy::new(|| ToAlphaChannelTaskSpec::StackAlphaOnBackground {
        background: 0x40,
        foreground: Box::new(stack_alpha(vec![
            svg_alpha_task("borderSolid"),
            svg_alpha_task("streaks"),
        ])),
    });

dyed_block!(STAINED_GLASS = paint_task(STAINED_GLASS_BASE.to_owned(), color!()));

dyed_block!(STAINED_GLASS_PANE_TOP = paint_task(from_svg_task("paneTop").into(), color!()));

single_texture_block!(
    GLASS = ComparableColor::TRANSPARENT,
    paint_svg_task("borderSolid", ComparableColor::STONE_EXTREME_SHADOW),
    paint_stack!(ComparableColor::WHITE, "borderSolidTopLeft", "streaks")
);

single_texture_block!(
    TINTED_GLASS = ComparableColor::BLACK * 0.25,
    paint_stack!(ComparableColor::WHITE * 0.25, "borderSolid", "streaks")
);

group!(
    GLASS_VARIANTS = GLASS_PANE_TOP,
    GLASS,
    TINTED_GLASS,
    STAINED_GLASS,
    STAINED_GLASS_PANE_TOP
);
