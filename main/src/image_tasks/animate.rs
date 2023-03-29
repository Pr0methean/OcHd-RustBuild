use anyhow::anyhow;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

pub fn animate(background: Pixmap, frames: Box<dyn ExactSizeIterator<Item=Pixmap>>) -> Result<Pixmap, anyhow::Error> {
    let background_copyable = background.as_ref();
    let frame_count = frames.len() as u32;
    let frame_height = background.height();
    let mut out = Pixmap::new(background.width(),
                              frame_height * frame_count)
        .ok_or(anyhow!("Failed to create output Pixmap"))?;
    for i in 0..frame_count {
        out.draw_pixmap(0, (i * frame_height) as i32,
                        background_copyable,
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
    }
    drop(background_copyable);
    drop(background);
    let mut i = 0;
    for frame in frames {
        out.draw_pixmap(0, (i * frame_height) as i32,
                        frame.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
        drop(frame);
        i = i + 1;
    }
    return Ok(out);
}