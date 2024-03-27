use futures_util::{TryFutureExt};
use resvg::tiny_skia::{Pixmap, PixmapPaint, Transform};
use tokio::task::JoinSet;
use tracing::instrument;

use crate::image_tasks::cloneable::{Arcow, SimpleArcow};
use crate::image_tasks::task_spec::BasicTask;
use crate::image_tasks::{allocate_pixmap_empty, allocate_pixmap_for_overwrite, MaybeFromPool};

pub async fn animate(
    background: &Pixmap,
    frames: Vec<BasicTask<MaybeFromPool<Pixmap>>>,
    clear_output: bool,
) -> SimpleArcow<MaybeFromPool<Pixmap>> {
    let frame_height = background.height();
    let total_height = frame_height * (frames.len() as u32);
    let mut out = if clear_output {
        allocate_pixmap_empty(background.width(), total_height)
    } else {
        allocate_pixmap_for_overwrite(background.width(), total_height)
    };
    let background = (*background).as_ref();
    for index in 0..frames.len() {
        out.draw_pixmap(
            0,
            (index as i32) * (frame_height as i32),
            background,
            &PixmapPaint::default(),
            Transform::default(),
            None,
        );
    }
    let mut join_set = JoinSet::new();
    for (index, frame) in frames.into_iter().enumerate() {
        join_set.spawn(frame.and_then(async move |frame_pixmap| {
            out.draw_pixmap(
                0,
                (index as i32) * (frame_height as i32),
                frame_pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::default(),
                None,
            );
        }));
    }
    while join_set.join_next().await.is_some() {}
    Arcow::from_owned(out)
}
