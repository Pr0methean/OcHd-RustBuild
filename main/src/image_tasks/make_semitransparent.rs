use cached::proc_macro::cached;
use ordered_float::OrderedFloat;
use tiny_skia::{ColorU8, Pixmap};
use tracing::instrument;

use crate::image_tasks::task_spec::TaskResult;

#[cached(sync_writes = true)]
pub(crate) fn create_alpha_array(out_alpha: OrderedFloat<f32>) -> [u8; 256] {
    return (0u16..256u16).into_iter()
        .map (|alpha| (out_alpha.0 * f32::from(alpha)) as u8)
        .collect::<Vec<u8>>().try_into().unwrap();
}

#[instrument]
/// Multiplies the opacity of all pixels in the [input](given pixmap) by a given [alpha].
pub fn make_semitransparent(input: &Pixmap, alpha: f32) -> TaskResult {
    let alpha_array = create_alpha_array(alpha.into());
    let mut output = input.to_owned();
    let output_pixels = output.pixels_mut();
    for index in 0..output_pixels.len() {
        let pixel = output_pixels[index].demultiply();
        output_pixels[index] = ColorU8::from_rgba(pixel.red(), pixel.green(), pixel.blue(),
                alpha_array[pixel.alpha() as usize]).premultiply();
    }
    return TaskResult::Pixmap {value: output};
}