use crate::anyhoo;
use crate::image_tasks::cloneable::CloneableError;
use crate::image_tasks::make_semitransparent::ALPHA_STACKING_TABLE;
use resvg::tiny_skia::{BlendMode, Color, Mask, Paint, Pixmap, PixmapPaint, Rect, Transform};

pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    background.draw_pixmap(
        0,
        0,
        foreground.as_ref(),
        &PixmapPaint::default(),
        Transform::default(),
        None,
    );
}

pub fn stack_layer_on_background(
    background: Color,
    foreground: &mut Pixmap,
) -> Result<(), CloneableError> {
    let mut paint = Paint::default();
    paint.set_color(background);
    paint.blend_mode = BlendMode::DestinationOver;
    foreground.fill_rect(
        Rect::from_xywh(
            0.0,
            0.0,
            foreground.width() as f32,
            foreground.height() as f32,
        )
        .ok_or(anyhoo!("Failed to allocate a rectangle"))?,
        &paint,
        Transform::default(),
        None,
    );
    Ok(())
}

pub(crate) fn stack_alpha_on_alpha(background: &mut Mask, foreground: &Mask) {
    let fg_data = foreground.data();
    let bg_data = background.data_mut();
    for (index, pixel) in fg_data.iter().enumerate() {
        bg_data[index] = ((255.0 - bg_data[index] as f32) * (*pixel as f32 / 255.0)
            + bg_data[index] as f32
            + 0.5) as u8;
    }
}

pub fn stack_alpha_on_background(background_alpha: u8, foreground: &mut Mask) {
    for pixel in foreground.data_mut() {
        *pixel = ALPHA_STACKING_TABLE[background_alpha as usize][*pixel as usize];
    }
}
