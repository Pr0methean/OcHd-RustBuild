use std::sync::Arc;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::repaint::AlphaChannel;
use crate::image_tasks::task_spec::TaskResult;

#[instrument]
pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
}

#[instrument]
pub fn stack_layer_on_background(background: &ComparableColor, foreground: &Pixmap) -> TaskResult {
    let mut output = Pixmap::new(foreground.width(), foreground.height())
        .ok_or(anyhoo!("Failed to create background for stacking"))?;
    output.fill(background.to_owned().into());
    stack_layer_on_layer(&mut output, foreground);
    TaskResult::Pixmap {value: Arc::new(output)}
}

pub fn stack_alpha_on_alpha(background: &mut AlphaChannel, foreground: &AlphaChannel) {
    let output_pixels = background.pixels_mut();
    for (index, &pixel) in foreground.pixels().iter().enumerate() {
        output_pixels[index] = (pixel as u16 +
            ((output_pixels[index] as u16) * ((u8::MAX - pixel) as u16) / (u8::MAX as u16))) as u8;
    }
}
