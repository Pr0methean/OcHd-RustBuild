use std::sync::Arc;
use log::info;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::TaskResult;

#[instrument]
pub fn stack_layer_on_layer(background: &mut Pixmap, foreground: &Pixmap) {
    info!("Starting task: stack_layer_on_layer");
    background.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    info!("Finishing task: stack_layer_on_layer");
}

#[instrument]
pub fn stack_layer_on_background(background: &ComparableColor, foreground: &Pixmap) -> TaskResult {
    info!("Starting task: stack_layer_on_background (background: {})", background);
    let mut output = Pixmap::new(foreground.width(), foreground.height())
        .ok_or(anyhoo!("Failed to create background for stacking"))?;
    output.fill(background.to_owned().into());
    stack_layer_on_layer(&mut output, foreground);
    info!("Finishing task: stack_layer_on_background (background: {})", background);
    TaskResult::Pixmap {value: Arc::new(output)}
}
