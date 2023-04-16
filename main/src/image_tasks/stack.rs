
use log::info;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::repaint::AlphaChannel;
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};
use crate::image_tasks::task_spec::CloneableError;

#[instrument]
pub fn stack_layer_on_layer(background: &mut MaybeFromPool<Pixmap>, foreground: &MaybeFromPool<Pixmap>) {
    info!("Starting task: stack_layer_on_layer");
    let mut output = background.as_mut();
    output.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    info!("Finishing task: stack_layer_on_layer");
}

#[instrument]
pub fn stack_layer_on_background(background: &ComparableColor, foreground: &MaybeFromPool<Pixmap>)
        -> Result<MaybeFromPool<Pixmap>,CloneableError> {
    info!("Starting task: stack_layer_on_background (background: {})", background);
    let mut output = allocate_pixmap(foreground.width(), foreground.height());
    output.fill((*background).into());
    stack_layer_on_layer(&mut output, foreground);
    info!("Finishing task: stack_layer_on_background (background: {})", background);
    Ok(output)
}

pub fn stack_alpha_on_alpha(background: &mut MaybeFromPool<AlphaChannel>, foreground: &MaybeFromPool<AlphaChannel>)
        {
    let output_pixels = background.pixels_mut();
    for (index, &pixel) in foreground.pixels().iter().enumerate() {
        output_pixels[index] = (pixel as u16 +
            ((output_pixels[index] as u16) * ((u8::MAX - pixel) as u16) / (u8::MAX as u16))) as u8;
    }
}
