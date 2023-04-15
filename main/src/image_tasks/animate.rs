use std::sync::{Arc, Mutex};
use futures::future::join_all;
use log::info;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError, CloneableResult, TaskResultFuture};
use tracing::instrument;

#[instrument]
pub async fn animate<'bg_input, 'fg_input>(background: &'bg_input Pixmap, frames: Vec<TaskResultFuture<'fg_input, Pixmap>>)
                     -> Result<Pixmap,CloneableError> {
    info!("Starting task: Animate");
    let frame_height = background.height();
    let out = Mutex::new(Pixmap::new(background.width(),
                              frame_height * (frames.len() as u32))
                            .ok_or(anyhoo!("Failed to create output Pixmap"))?);
    let results = join_all(frames.into_iter().enumerate().map(|(index, frame)| {
        let background = (*background).as_ref();
        let out = &out;
        async move || -> Result<(), CloneableError>  {
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            background,
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))?;
            let frame_result: CloneableResult<Pixmap> = frame.await;
            let frame_pixmap: &Pixmap = &*frame_result?;
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            frame_pixmap.as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))
        }
    }()));
    for result in results.await {
        result?;
    }
    info!("Finishing task: Animate");
    out.into_inner().map_err(|e| anyhoo!(e))
}