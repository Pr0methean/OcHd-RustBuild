use std::sync::Arc;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::TaskResult;

#[instrument]
pub fn stack_layer_on_layer(mut background: Pixmap, foreground: &Pixmap) -> TaskResult {
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    TaskResult::Pixmap {value: Arc::new(background)}
}

#[instrument]
pub fn stack_layer_on_background(background: &ComparableColor, foreground: &Pixmap) -> TaskResult {
    let mut output = Pixmap::new(foreground.width(), foreground.height())
        .ok_or(anyhoo!("Failed to create background for stacking"))?;
    output.fill(background.to_owned().into());
    stack_layer_on_layer(output, foreground)
}
