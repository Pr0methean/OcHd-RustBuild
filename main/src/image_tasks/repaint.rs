use log::info;
use tiny_skia::{Mask, MaskType, Paint, Pixmap};
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

#[test]
fn test_paint() {
    use tiny_skia::{ColorU8, FillRule, Paint};
    use tiny_skia_path::{PathBuilder, Transform};


    let side_length = 128;
    let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    pixmap.fill_path(&circle, &Paint::default(),
                     FillRule::EvenOdd, Transform::default(), None);
    let alpha_channel = Mask::from_pixmap(pixmap.as_ref(), MaskType::Alpha);
    let repainted_alpha: u8 = 0xcf;
    let red = ColorU8::from_rgba(0xff, 0, 0, repainted_alpha);
    let repainted_red: Box<Pixmap> = paint(&alpha_channel, ComparableColor::from(red))
        .unwrap();
    let pixmap_pixels = pixmap.pixels();
    let repainted_pixels = repainted_red.pixels();
    for index in 0usize..((side_length * side_length) as usize) {
        let expected_alpha: u8 = (u16::from(repainted_alpha)
            * u16::from(pixmap_pixels[index].alpha()) / 0xff) as u8;
        assert!(repainted_pixels[index].alpha().abs_diff(expected_alpha) <= 1);
        if expected_alpha > 0 {
            // premultiplied
            assert_eq!(repainted_pixels[index].red(), repainted_pixels[index].alpha());

            assert_eq!(repainted_pixels[index].green(), 0);
            assert_eq!(repainted_pixels[index].blue(), 0);
        }
    }
}