use std::sync::{Arc, Mutex};
use futures::future::join_all;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError, TaskResult, TaskResultFuture};
use tracing::instrument;
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};

#[instrument]
pub async fn animate<'input,'output>(background: TaskResultFuture<'input>, frames: Vec<TaskResultFuture<'input>>)
                                    -> TaskResult<'output> where 'output : 'input {
    let frame_count = frames.len() as u32;
    let background: Arc<MaybeFromPool<Pixmap>> = (*background.await).to_owned().try_into_pixmap()?;
    let frame_height = background.height();
    let out = Mutex::new(allocate_pixmap(background.width(),
                              frame_height * frame_count));
    let results = join_all(frames.into_iter().enumerate().map(|(index, frame)| {
        let background = (*background).as_ref();
        let out = &out;
        async move || -> Result<(), CloneableError>  {
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            background,
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))?;
            let frame_pixmap: Arc<MaybeFromPool<Pixmap>>
                = (*frame.await).to_owned().try_into_pixmap()?;
            out.lock().unwrap().draw_pixmap(0, (index as i32) * (frame_height as i32),
                            (*frame_pixmap).as_ref(),
                            &PixmapPaint::default(),
                            Transform::default(),
                            None).ok_or(anyhoo!("draw_pixmap failed"))
        }
    }()));
    for result in results.await {
        result?;
    }
    TaskResult::Pixmap { value: Arc::new(out.into_inner().unwrap()) }
}