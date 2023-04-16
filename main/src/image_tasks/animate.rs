use log::info;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};
use crate::image_tasks::task_spec::{CloneableError, CloneableLazyTask, CloneableResult};
use tracing::instrument;

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
                        None).ok_or(anyhoo!("draw_pixmap failed"))?;
        let frame_result: CloneableResult<MaybeFromPool<Pixmap>> = frame.into_result();
        let frame_pixmap: &Pixmap = &*frame_result?;
        out.draw_pixmap(0, (index as i32) * (frame_height as i32),
                        frame_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None).ok_or(anyhoo!("draw_pixmap failed"))?;
    }
    info!("Finishing task: Animate");
    Ok(Box::from(out))
}