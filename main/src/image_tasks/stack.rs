
use log::info;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::repaint::AlphaChannel;
use crate::image_tasks::task_spec::CloneableError;

#[instrument]
pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    info!("Starting task: stack_layer_on_layer");
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    info!("Finishing task: stack_layer_on_layer");
}

#[instrument]
pub fn stack_layer_on_background(background: &ComparableColor, foreground: &Pixmap)
        -> Result<Pixmap,CloneableError> {
    info!("Starting task: stack_layer_on_background (background: {})", background);
    let mut output = Pixmap::new(foreground.width(), foreground.height())
        .ok_or(anyhoo!("Failed to create background for stacking"))?;
    output.fill((*background).into());
    stack_layer_on_layer(&mut output, foreground);
    info!("Finishing task: stack_layer_on_background (background: {})", background);
    Ok(output)
}

pub fn stack_alpha_on_alpha(background: &mut AlphaChannel, foreground: &AlphaChannel)
        {
    let output_pixels = background.pixels_mut();
    for (index, &pixel) in foreground.pixels().iter().enumerate() {
        output_pixels[index] = (pixel as u16 +
            ((output_pixels[index] as u16) * ((u8::MAX - pixel) as u16) / (u8::MAX as u16))) as u8;
    }
}

pub fn stack_alpha_on_background(background_alpha: f32, foreground: &mut AlphaChannel)
{
    let background_alpha = (u8::MAX as f32 * background_alpha + 0.5) as u8;
    let output_pixels = foreground.pixels_mut();
    for pixel in output_pixels {
        *pixel = background_alpha + (
            ((*pixel as u16) * (u8::MAX - background_alpha) as u16) / u8::MAX as u16) as u8;
    }
}
