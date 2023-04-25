use log::info;
use tiny_skia::{Mask, Paint, Pixmap};
use tiny_skia_path::{Rect, Transform};
use tracing::instrument;
use crate::anyhoo;


use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::{CloneableError};

/// Applies the given [color] to the given [input](alpha channel).
#[instrument]
pub fn paint(input: &Mask, color: ComparableColor) -> Result<Box<Pixmap>, CloneableError> {
    info!("Starting task: paint with color {}", color);
    let mut output = Pixmap::new(input.width(), input.height())
        .ok_or(anyhoo!("Failed to create output Pixmap"))?;
    let mut paint = Paint::default();
    paint.set_color(color.into());
    output.fill_rect(Rect::from_ltrb(0.0, 0.0, input.width() as f32, input.height() as f32)
                         .ok_or(anyhoo!("Failed to create rectangle for paint()"))?,
                     &paint, Transform::default(),
                     Some(input));
    info!("Finishing task: paint with color {}", color);
    Ok(Box::new(output))
}