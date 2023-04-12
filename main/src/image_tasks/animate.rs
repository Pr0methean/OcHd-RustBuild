use std::sync::Mutex;
use futures::future::join_all;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{TaskResult, TaskResultFuture};
use tracing::instrument;

#[instrument]
pub async fn animate<'input>(background: TaskResultFuture<'input>, frames: Vec<TaskResultFuture<'input>>)
                                    -> TaskResult {
    let frame_count = frames.len() as u32;
    let background: &Pixmap = &(background.await.try_into()?);
    let frame_height = background.height();
    let out = Mutex::new(Pixmap::new(background.width(),
                              frame_height * frame_count)
                            .ok_or(anyhoo!("Failed to create output Pixmap"))?);
    join_all(frames.into_iter().enumerate().map(|(index, frame)| {
        let out = &out;
        let background = background.to_owned();
        async move {
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            background.as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None);
            let frame_pixmap: &Pixmap = &(frame.to_owned().await.try_into().unwrap());
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            frame_pixmap.as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None);
        }
    })).await;
    return TaskResult::Pixmap { value: out.into_inner().unwrap() };
}