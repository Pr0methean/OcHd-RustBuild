use log::info;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::image_tasks::task_spec::{CloneableError, CloneableLazyTask, CloneableResult};
use tracing::instrument;
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};

#[instrument]
pub fn animate(background: &Pixmap, frames: Vec<CloneableLazyTask<MaybeFromPool<Pixmap>>>)
                     -> Result<Box<MaybeFromPool<Pixmap>>, CloneableError> {
    info!("Starting task: Animate");
    let frame_height = background.height();
    let mut out = allocate_pixmap(background.width(),
                              frame_height * (frames.len() as u32));
    for (index, frame) in frames.into_iter().enumerate() {
        let background = (*background).as_ref();
        out.draw_pixmap(0, (index as i32) * (frame_height as i32),
                        background,
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
        let frame_result: CloneableResult<MaybeFromPool<Pixmap>> = frame.into_result();
        let frame_pixmap = &*frame_result?;
        out.draw_pixmap(0, (index as i32) * (frame_height as i32),
                        (**frame_pixmap).as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
    }
    info!("Finishing task: Animate");
    Ok(Box::from(out))
}