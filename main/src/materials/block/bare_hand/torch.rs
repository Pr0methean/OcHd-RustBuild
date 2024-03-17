use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{from_svg_task, paint_svg_task, ToPixmapTaskSpec};
use crate::materials::block::axe::wood::OAK;
use crate::{group, single_texture_block, stack};
use once_cell::sync::Lazy;
pub static TORCH_BASE: Lazy<ToPixmapTaskSpec> = Lazy::new(|| {
    stack!(
        paint_svg_task("torchBase", OAK.highlight),
        paint_svg_task("torchShadow", OAK.shadow)
    )
});
single_texture_block!(
    TORCH = ComparableColor::TRANSPARENT,
    TORCH_BASE.to_owned(),
    from_svg_task("torchFlameSmall")
);
single_texture_block!(
    SOUL_TORCH = ComparableColor::TRANSPARENT,
    TORCH_BASE.to_owned(),
    from_svg_task("soulTorchFlameSmall")
);
single_texture_block!(
    REDSTONE_TORCH_OFF = ComparableColor::TRANSPARENT,
    TORCH_BASE.to_owned(),
    paint_svg_task("torchRedstoneHead", ComparableColor::BLACK)
);
single_texture_block!(
    REDSTONE_TORCH = ComparableColor::TRANSPARENT,
    TORCH_BASE.to_owned(),
    from_svg_task("torchRedstoneHead")
);
group!(
    TORCHES = TORCH,
    SOUL_TORCH,
    REDSTONE_TORCH_OFF,
    REDSTONE_TORCH
);
