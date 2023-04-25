
use log::info;
use tiny_skia::{Mask, Paint, Pixmap, PixmapPaint};
use tiny_skia_path::{Rect, Transform};
use tracing::instrument;

use crate::image_tasks::color::ComparableColor;

#[instrument]
pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    info!("Starting task: stack_layer_on_layer");
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    info!("Finishing task: stack_layer_on_layer");
}

#[instrument]
pub fn stack_layer_on_background(background: ComparableColor, foreground: &mut Pixmap) {
    info!("Starting task: stack_layer_on_background (background: {})", background);
    let mut paint = Paint::default();
    paint.set_color(background.into());
    foreground.fill_rect(Rect::from_xywh(0.0, 0.0, foreground.width() as f32, foreground.height() as f32).unwrap(),
                         &paint, Transform::default(), None);
    info!("Finishing task: stack_layer_on_background (background: {})", background);
}

pub(crate) fn stack_alpha_on_alpha(background: &mut Mask, foreground: &Mask) {
    let fg_data = foreground.data();
    for (index, &mut mut pixel) in background.data_mut().iter_mut().enumerate() {
        pixel = (pixel as u16 +
            (fg_data[index] as u16) * ((u8::MAX - pixel) as u16) / (u8::MAX as u16)) as u8;
    }
}

pub fn stack_alpha_on_background(background_alpha: f32, foreground: &mut Mask)
{
    let background_alpha = (u8::MAX as f32 * background_alpha + 0.5) as u8;
    let output_pixels = foreground.data_mut();
    for pixel in output_pixels {
        *pixel = background_alpha + (
            ((*pixel as u16) * (u8::MAX - background_alpha) as u16) / u8::MAX as u16) as u8;
    }
}
