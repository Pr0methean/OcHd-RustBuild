use std::sync::{Arc, Mutex};
use anyhow::Error;
use futures::future::join_all;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError, TaskResult, TaskResultFuture};
use tracing::instrument;

#[instrument]
pub async fn animate<'input>(background: TaskResultFuture<'input>, frames: Vec<TaskResultFuture<'input>>)
                                    -> TaskResult {
    let frame_count = frames.len() as u32;
    let background: Arc<Pixmap> = background.await.try_into()?;
    let frame_height = background.height();
    let out = Mutex::new(Pixmap::new(background.width(),
                              frame_height * frame_count)
                            .ok_or(anyhoo!("Failed to create output Pixmap"))?);
    let results = join_all(frames.into_iter().enumerate().map(|(index, frame)| {
        let out = &out;
        let background = background.to_owned();
        async move || -> Result<(), CloneableError>  {
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            background.as_ref().as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))?;
            let frame_pixmap: Result<Arc<Pixmap>, CloneableError> = frame.await.try_into();
            let frame_pixmap = frame_pixmap?;
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            frame_pixmap.as_ref().as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))
        }
    }()));
    for result in results.await {
        result?;
    }
    return TaskResult::Pixmap { value: Arc::new(out.into_inner().unwrap()) };
}