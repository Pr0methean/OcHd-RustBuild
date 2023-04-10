use cached::proc_macro::cached;
use ordered_float::OrderedFloat;
use tiny_skia::{ColorU8, Pixmap};
use tracing::instrument;
use crate::image_tasks::color::ComparableColor;

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

#[test]
fn test_make_semitransparent() {
    use tiny_skia::{FillRule, Paint};
    use tiny_skia_path::{PathBuilder, Transform};

    let alpha = 0.5;
    let alpha_multiplier = (alpha * f32::from(u8::MAX)) as u16;
    let side_length = 128;
    let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    let mut red_paint = Paint::default();
    red_paint.set_color(ComparableColor::RED.into());
    pixmap.fill_path(&circle, &red_paint,
                     FillRule::EvenOdd, Transform::default(), None);
    let pixmap_pixels = pixmap.pixels();
    let semitransparent_red_circle: Pixmap
        = make_semitransparent(&pixmap, alpha).try_into().unwrap();
    let semitransparent_pixels = semitransparent_red_circle.pixels();
    for index in 0usize..((side_length * side_length) as usize) {
        let expected_alpha: u8 = (u16::from(alpha_multiplier
            * u16::from(pixmap_pixels[index].alpha()) / 0xff)) as u8;
        assert_eq!(semitransparent_pixels[index].alpha(), expected_alpha);
        if expected_alpha > 0 {
            assert_eq!(semitransparent_pixels[index].red(), expected_alpha); // premultiplied
            assert_eq!(semitransparent_pixels[index].green(), 0);
            assert_eq!(semitransparent_pixels[index].blue(), 0);
        }
    }
}
