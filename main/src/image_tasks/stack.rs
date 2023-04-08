use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::TaskResult;

pub fn stack_layer_on_layer(background: Pixmap, foreground: Pixmap) -> TaskResult {
    let mut output = background.to_owned();
    drop(background);
    output.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    drop(foreground);
    return TaskResult::Pixmap {value: output};
}

pub fn stack_layer_on_background(background: &ComparableColor, foreground: Pixmap) -> TaskResult {
    let mut output = Pixmap::new(foreground.width(), foreground.height())
        .ok_or(anyhoo!("Failed to create background for stacking"))?;
    output.fill(background.to_owned().into());
    return stack_layer_on_layer(output, foreground);
}
