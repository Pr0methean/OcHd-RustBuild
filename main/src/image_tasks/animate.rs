use resvg::tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::image_tasks::task_spec::{CloneableError, CloneableLazyTask, CloneableResult};
use crate::image_tasks::{allocate_pixmap_empty, allocate_pixmap_for_overwrite, MaybeFromPool};

pub fn animate(background: &Pixmap, frames: Vec<CloneableLazyTask<MaybeFromPool<Pixmap>>>, clear_output: bool)
               -> Result<Box<MaybeFromPool<Pixmap>>, CloneableError> {
    let frame_height = background.height();
    let total_height = frame_height * (frames.len() as u32);
    let mut out = if clear_output {
        allocate_pixmap_empty(background.width(), total_height)
    } else {
        allocate_pixmap_for_overwrite(background.width(), total_height)
    };
    let background = (*background).as_ref();
    for index in 0..frames.len() {
        out.draw_pixmap(0, (index as i32) * (frame_height as i32),
                        background,
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
    }
    for (index, frame) in frames.into_iter().enumerate() {
        let frame_result: CloneableResult<MaybeFromPool<Pixmap>> = frame.into_result();
        let frame_pixmap = &*frame_result?;
        out.draw_pixmap(0, (index as i32) * (frame_height as i32),
                        (**frame_pixmap).as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
    }
    Ok(Box::from(out))
}