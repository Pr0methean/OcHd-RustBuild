use std::future::Future;
use anyhow::anyhow;
use tiny_skia::{Pixmap, PixmapPaint};
use tiny_skia_path::Transform;

pub fn animate(background: Pixmap, frames: Vec<Box<dyn Future<Output=Pixmap>>>)
        -> Result<Pixmap, anyhow::Error> {
    let frame_count = frames.len() as u32;
    let frame_height = background.height().to_owned();
    let mut out = Pixmap::new(background.width(),
                              frame_height * frame_count)
        .ok_or(anyhow!("Failed to create output Pixmap"))?;
    for i in 0..frame_count {
        out.draw_pixmap(0, (i * frame_height) as i32,
                        background.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
    }
    drop(background);
    let mut i: u32 = 0;
    for frame in frames {
        out.draw_pixmap(0, (i * frame_height) as i32,
                        frame.as_ref(),
                        &PixmapPaint::default(),
                        Transform::default(),
                        None);
        drop(frame);
        i += 1;
    }
    return Ok(out);
}