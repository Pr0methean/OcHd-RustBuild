use anyhow::anyhow;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::{Transform};
use crate::image_tasks::color::ComparableColor;

pub fn stack(background: ComparableColor, mut layers: Box<dyn Iterator<Item=Pixmap>>) -> Result<Pixmap, anyhow::Error> {
    let first_layer = layers.next().ok_or(anyhow!("Tried to stack an empty list of layers"))?;
    let mut output = Pixmap::new(first_layer.width(), first_layer.height())
        .ok_or(anyhow!("Failed to create output Pixmap"))?;
    if background.alpha() > 0 {
        output.fill(background.into());
    }
    output.draw_pixmap(0, 0, first_layer.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None)
        .ok_or(anyhow!("Failed to render first layer while stacking"))?;
    drop(first_layer);
    for layer in layers {
        output.draw_pixmap(0, 0, layer.as_ref(), &PixmapPaint::default(),
                           Transform::default(), None)
            .ok_or(anyhow!("Failed to render a layer while stacking"))?;
        drop(layer);
    }
    return Ok(output);
}