
use log::info;
use tiny_skia::{BlendMode, Paint, Pixmap, PixmapPaint, Shader};
use tiny_skia_path::{Rect, Transform};
use tracing::instrument;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::repaint::AlphaChannel;
use crate::image_tasks::MaybeFromPool;

#[instrument]
pub fn stack_layer_on_layer(background: &mut MaybeFromPool<Pixmap>, foreground: &MaybeFromPool<Pixmap>) {
    info!("Starting task: stack_layer_on_layer");
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    info!("Finishing task: stack_layer_on_layer");
}

#[instrument]
pub fn stack_layer_on_background(background: &ComparableColor, foreground: &mut Pixmap) {
    info!("Starting task: stack_layer_on_background (background: {})", background);
    foreground.fill_rect(Rect::from_xywh(0.0, 0.0, foreground.width() as f32, foreground.height() as f32).unwrap(),
                         &Paint {
                             shader: Shader::SolidColor((*background).into()),
                             blend_mode: BlendMode::DestinationOver,
                             anti_alias: true,
                             force_hq_pipeline: false
                         }, Transform::default(), None);
    info!("Finishing task: stack_layer_on_background (background: {})", background);
}

pub(crate) fn stack_alpha_on_alpha(background: &mut MaybeFromPool<AlphaChannel>, foreground: &MaybeFromPool<AlphaChannel>)
        {
    let output_pixels = background.pixels_mut();
    for (index, &pixel) in foreground.pixels().iter().enumerate() {
        output_pixels[index] = (pixel as u16 +
            ((output_pixels[index] as u16) * ((u8::MAX - pixel) as u16) / (u8::MAX as u16))) as u8;
    }
}

pub fn stack_alpha_on_background(background_alpha: f32, foreground: &mut MaybeFromPool<AlphaChannel>)
{
    let background_alpha = (u8::MAX as f32 * background_alpha + 0.5) as u8;
    let output_pixels = foreground.pixels_mut();
    for pixel in output_pixels {
        *pixel = background_alpha + (
            ((*pixel as u16) * (u8::MAX - background_alpha) as u16) / u8::MAX as u16) as u8;
    }
}
