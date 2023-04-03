use anyhow::anyhow;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::{Transform};
use crate::image_tasks::color::ComparableColor;

pub fn stack_layer_on_layer(background: Pixmap, foreground: Pixmap) -> Result<Pixmap, anyhow::Error> {
    let mut output = background.to_owned();
    output.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    return Ok(output);
}

pub fn stack_layer_on_background(background: ComparableColor, foreground: Pixmap) -> Result<Pixmap, anyhow::Error> {
    let mut output = Pixmap::new(foreground.width(), foreground.height())
        .ok_or(anyhow!("Failed to create background for stacking"))?;
    output.fill(background.into());
    return stack_layer_on_layer(output, foreground);
}
