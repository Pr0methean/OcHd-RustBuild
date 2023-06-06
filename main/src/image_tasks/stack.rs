use resvg::tiny_skia::{BlendMode, Color, Mask, Paint, Pixmap, PixmapPaint, Rect, Transform};
use crate::anyhoo;
use crate::image_tasks::make_semitransparent::ALPHA_MULTIPLICATION_TABLE;
use crate::image_tasks::task_spec::CloneableError;

pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
}

pub fn stack_layer_on_background(background: Color, foreground: &mut Pixmap) -> Result<(), CloneableError> {
    let mut paint = Paint::default();
    paint.set_color(background);
    paint.blend_mode = BlendMode::DestinationOver;
    foreground.fill_rect(Rect::from_xywh(0.0, 0.0, foreground.width() as f32, foreground.height() as f32)
                             .ok_or(anyhoo!("Failed to allocate a rectangle"))?,
                         &paint, Transform::default(), None);
    Ok(())
}

pub(crate) fn stack_alpha_on_alpha(background: &mut Mask, foreground: &Mask) {
    let fg_data = foreground.data();
    let bg_data = background.data_mut();
    for (index, pixel) in fg_data.iter().enumerate() {
        bg_data[index] = ((255.0 - bg_data[index] as f32) * (*pixel as f32 / 255.0)
            + bg_data[index] as f32 + 0.5) as u8;
    }
}

pub fn stack_alpha_on_background(background_alpha: u8, foreground: &mut Mask)
{
    for pixel in foreground.data_mut() {
        *pixel = stack_alpha_pixel(background_alpha, *pixel);
    }
}

pub fn stack_alpha_pixel(background: u8, foreground: u8) -> u8 {
    background + ALPHA_MULTIPLICATION_TABLE[(u8::MAX - background) as usize][foreground as usize]
}
