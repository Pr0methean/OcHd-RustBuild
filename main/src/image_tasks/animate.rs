use core::mem::size_of;
use std::cell::RefCell;
use std::mem;
use futures_util::FutureExt;
use resvg::tiny_skia::{Pixmap, PixmapMut, PixmapPaint, PremultipliedColorU8, Transform};
use tokio::task::JoinSet;
use tracing::instrument;

use crate::image_tasks::cloneable::{Arcow, SimpleArcow};
use crate::image_tasks::task_spec::BasicTask;
use crate::image_tasks::{allocate_pixmap_empty, allocate_pixmap_for_overwrite, MaybeFromPool};
use crate::join_all;

#[instrument(skip(background))]
pub async fn animate(
    background: &Pixmap,
    frames: Box<[BasicTask<MaybeFromPool<Pixmap>>]>,
    clear_output: bool,
) -> SimpleArcow<MaybeFromPool<Pixmap>> {
    let frame_height = background.height();
    let total_height = frame_height * (frames.len() as u32);
    let width = background.width();
    let out = RefCell::new(if clear_output {
        allocate_pixmap_empty(width, total_height)
    } else {
        allocate_pixmap_for_overwrite(background.width(), total_height)
    });
    let background = (*background).as_ref();
    // SAFETY: transmuted back to PixmapMut per frame by from_bytes
    let mut remainder: &mut [u8] = unsafe { mem::transmute(out.borrow_mut().pixels_mut()) };
    let mut join_set = JoinSet::new();
    for frame in frames.into_vec().into_iter() {
        let (frame_pixels, new_remainder)
            = remainder.split_at_mut(frame_height as usize * width as usize * size_of::<PremultipliedColorU8>());
        remainder = new_remainder;
        let mut frame_buffer = PixmapMut::from_bytes(frame_pixels, width, frame_height).unwrap();
        frame_buffer.draw_pixmap(
            0,
            0,
            background,
            &PixmapPaint::default(),
            Transform::default(),
            None,
        );
        join_set.spawn(frame.map(async move |frame_pixmap: SimpleArcow<MaybeFromPool<Pixmap>>| {
            frame_buffer.draw_pixmap(
                0,
                0,
                frame_pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::default(),
                None,
            );
        }));
    }
    join_all(join_set).await;
    Arcow::from_owned(out.into_inner())
}
