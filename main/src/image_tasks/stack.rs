use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;
use tracing::instrument;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};

#[instrument]
pub fn stack_layer_on_layer<'a,'b>(background: &mut MaybeFromPool<Pixmap>, foreground: &MaybeFromPool<Pixmap>) {
    let output = background.deref_mut();
    output.draw_pixmap(0, 0, foreground.as_ref(), &PixmapPaint::default(),
                       Transform::default(), None);
}

#[instrument]
pub fn stack_layer_on_background<'a,'b>(background: &ComparableColor, foreground: &MaybeFromPool<'a, Pixmap>) -> MaybeFromPool<'b, Pixmap> {
    let mut output = allocate_pixmap(foreground.width(), foreground.height());
    output.fill(background.to_owned().into());
    stack_layer_on_layer(&mut output, foreground);
    return output;
}
