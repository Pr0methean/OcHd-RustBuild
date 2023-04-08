use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use async_std::future::IntoFuture;
use futures::future::{BoxFuture, Shared};
use futures::TryFutureExt;
use tiny_skia::{Pixmap, PixmapPaint, PixmapRef};
use tiny_skia_path::Transform;

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError, CloneableFutureWrapper, TaskResult, TaskSpec};

pub async fn animate<'a>(background: Pixmap, frames: Vec<CloneableFutureWrapper<'a, TaskResult>>)
                         -> TaskResult {
    let frame_count = frames.len() as u32;
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
        let frame_pixmap: Pixmap = frame.await.clone().try_into()?;
        out.draw_pixmap(0, (i * frame_height) as i32,
                        frame_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
        i += 1;
    }
    return TaskResult::Pixmap { value: out };
}