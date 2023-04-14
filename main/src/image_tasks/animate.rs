use std::sync::{Arc, Mutex};
use futures::future::join_all;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError, TaskResult, TaskResultLazy};
use tracing::instrument;

#[instrument]
pub fn animate(background: TaskResultLazy, frames: Vec<TaskResultLazy>)
                                    -> TaskResult {
    let frame_count = frames.len() as u32;
    let background: Arc<Pixmap> = (&**background).try_into()?;
    let frame_height = background.height();
    let mut out = Pixmap::new(background.width(),
                              frame_height * frame_count)
                            .ok_or(anyhoo!("Failed to create output Pixmap"))?;
    for (index, frame) in frames.into_iter().enumerate() {
        let y_offset = (index as i32) * (frame_height as i32);
        out.draw_pixmap(0, y_offset,
                        (*background).as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))?;
        let frame_pixmap: Arc<Pixmap> = (&**frame).try_into()?;
        out.draw_pixmap(0, y_offset,
                        (*frame_pixmap).as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))?;
    }
    TaskResult::Pixmap { value: Arc::new(out) }
}