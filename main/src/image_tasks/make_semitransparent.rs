use tiny_skia::{Pixmap};
use crate::image_tasks::color::ComparableColor;
use cached::proc_macro::cached;
use ordered_float::OrderedFloat;

#[cached(sync_writes = true)]
fn create_alpha_array(out_alpha: OrderedFloat<f32>) -> [u8; 256] {
    return (0u16..256u16).into_iter()
        .map (|alpha| (out_alpha.0 * f32::from(alpha)) as u8)
        .collect::<Vec<u8>>().try_into().unwrap();
}

pub fn make_semitransparent(input: Pixmap, alpha: f32) -> Result<Pixmap, anyhow::Error> {
    let mut output = input.clone();
    let output_pixels = output.pixels_mut();
    for mut pixel in output_pixels {
        pixel = &mut((ComparableColor::from(*pixel) * alpha).into());
    }
    return Ok(output);
}