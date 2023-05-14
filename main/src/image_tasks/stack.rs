use log::info;
use tiny_skia::{BlendMode, Color, Mask, Paint, Pixmap, PixmapPaint};
use tiny_skia_path::{Rect, Transform};
use crate::anyhoo;
use crate::image_tasks::task_spec::CloneableError;

pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    info!("Starting task: stack_layer_on_layer");
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    info!("Finishing task: stack_layer_on_layer");
}

pub fn stack_layer_on_background(background: Color, foreground: &mut Pixmap) -> Result<(), CloneableError> {
    info!("Starting task: stack_layer_on_background (background: {:?})", background);
    let mut paint = Paint::default();
    paint.set_color(background);
    paint.blend_mode = BlendMode::DestinationOver;
    foreground.fill_rect(Rect::from_xywh(0.0, 0.0, foreground.width() as f32, foreground.height() as f32)
                             .ok_or(anyhoo!("Failed to allocate a rectangle"))?,
                         &paint, Transform::default(), None);
    info!("Finishing task: stack_layer_on_background (background: {:?})", background);
    Ok(())
}

pub(crate) fn stack_alpha_on_alpha(background: &mut Mask, foreground: &Mask) {
    info!("Starting task: stack_alpha_on_alpha");
    let fg_data = foreground.data();
    let bg_data = background.data_mut();
    for (index, pixel) in fg_data.iter().enumerate() {
        bg_data[index] = (*pixel as u16 +
            (bg_data[index] as u16) * ((u8::MAX - pixel) as u16) / (u8::MAX as u16)) as u8;
    }
    info!("Finishing task: stack_alpha_on_alpha");
}

pub fn stack_alpha_on_background(background_alpha: f32, foreground: &mut Mask)
{
    info!("Starting task: stack_alpha_on_background (background: {})", background_alpha);
    let background_alpha = (u8::MAX as f32 * background_alpha + 0.5) as u8;
    let output_pixels = foreground.data_mut();
    for pixel in output_pixels {
        *pixel = background_alpha + (
            ((*pixel as u16) * (u8::MAX - background_alpha) as u16) / u8::MAX as u16) as u8;
    }
    info!("Finishing task: stack_alpha_on_background (background: {})", background_alpha);
}
