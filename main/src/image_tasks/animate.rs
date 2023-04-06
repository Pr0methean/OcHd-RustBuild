use std::future::Future;
use std::ops::Deref;
use anyhow::{anyhow, Error};
use async_std::future::IntoFuture;
use futures::future::{BoxFuture, Shared};
use futures::TryFutureExt;
use tiny_skia::{Pixmap, PixmapPaint, PixmapRef};
use tiny_skia_path::Transform;
use crate::image_tasks::task_spec::{CloneableResult, SharedResultFuture};
use crate::anyhoo;

pub async fn animate(background: SharedResultFuture<Pixmap>, frames: Vec<&SharedResultFuture<Pixmap>>)
                     -> CloneableResult<Pixmap> {
    let frame_count = frames.len() as u32;
    let background_result: CloneableResult<Pixmap> = background.into().await;
    let background = background_result?;
    let frame_height = background.height().to_owned();
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
        let frame_result: CloneableResult<Pixmap> = frame.into().await;
        let frame_pixmap = frame_result?;
        out.draw_pixmap(0, (i * frame_height) as i32,
                        frame_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
        i += 1;
    }
    return Ok(out);
}