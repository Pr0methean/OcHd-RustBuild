use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{TaskResult, TaskResultFuture};
use tracing::instrument;

#[instrument]
pub async fn animate<'a>(background: &Pixmap, frames: Vec<TaskResultFuture<'a>>)
                         -> TaskResult {
    let frame_count = frames.len() as u32;
    let frame_height = background.height();
    let mut out = Pixmap::new(background.width(),
                              frame_height * frame_count)
        .ok_or(anyhoo!("Failed to create output Pixmap"))?;
    for i in 0..frame_count {
        out.draw_pixmap(0, (i * frame_height) as i32,
                        background.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
    }
    let mut i: u32 = 0;
    for frame in frames {
        let frame_pixmap: Pixmap = frame.await.try_into()?;
        out.draw_pixmap(0, (i * frame_height) as i32,
                        frame_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
        i += 1;
    }
    return TaskResult::Pixmap { value: out };
}