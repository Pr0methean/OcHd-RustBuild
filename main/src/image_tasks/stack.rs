use std::ops::DerefMut;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};
use crate::image_tasks::task_spec::TaskResult;

#[instrument]
pub fn stack_layer_on_layer<'a,'b>(mut background: MaybeFromPool<'a, Pixmap>, foreground: MaybeFromPool<Pixmap>) -> TaskResult<'a> {
    let output = background.deref_mut();
    output.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
    return TaskResult::Pixmap {value: Arc::new(background)};
}

#[instrument]
pub fn stack_layer_on_background<'a>(background: &ComparableColor, foreground: MaybeFromPool<Pixmap>) -> TaskResult<'a> {
    let mut output = allocate_pixmap(foreground.width(), foreground.height());
    output.fill(background.to_owned().into());
    return stack_layer_on_layer(output, foreground);
}
