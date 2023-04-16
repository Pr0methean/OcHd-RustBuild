use cached::proc_macro::cached;
use log::info;
use ordered_float::OrderedFloat;

use tracing::instrument;
use crate::image_tasks::repaint::{AlphaChannel};
use crate::image_tasks::{MaybeFromPool};



#[cached(sync_writes = true)]
pub(crate) fn create_alpha_array(out_alpha: OrderedFloat<f32>) -> [u8; 256] {
    return (0u16..256u16)
        .map (|alpha| (out_alpha.0 * f32::from(alpha)) as u8)
        .collect::<Vec<u8>>().try_into().unwrap();
}

#[instrument]
/// Multiplies the opacity of all pixels in the [input](given pixmap) by a given [alpha].
pub fn make_semitransparent(input: &mut MaybeFromPool<AlphaChannel>, alpha: f32) {
    info!("Starting task: make semitransparent with alpha {}", alpha);
    let alpha_array = create_alpha_array(alpha.into());
    let output_pixels = input.pixels_mut();
    for index in 0..output_pixels.len() {
        let pixel = output_pixels[index];
        output_pixels[index] = alpha_array[pixel as usize];
    }
    info!("Finishing task: make semitransparent with alpha {}", alpha);
}

#[test]
fn test_make_semitransparent() {
    use tiny_skia::{FillRule, Paint, Pixmap};
    use tiny_skia_path::{PathBuilder, Transform};
    use crate::image_tasks::color::ComparableColor;
    use crate::image_tasks::repaint::paint;
    use crate::image_tasks::repaint::to_alpha_channel;
    use crate::image_tasks::MaybeFromPool::NotFromPool;

    let alpha = 0.5;
    let alpha_multiplier = (alpha * f32::from(u8::MAX)) as u16;
    let side_length = 128;
    let mut pixmap = NotFromPool(Pixmap::new(side_length, side_length).unwrap());
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    let mut red_paint = Paint::default();
    red_paint.set_color(ComparableColor::RED.into());
    pixmap.as_mut().fill_path(&circle, &red_paint,
                     FillRule::EvenOdd, Transform::default(), None);
    let pixmap_pixels = pixmap.pixels().to_owned();
    let mut semitransparent_circle: MaybeFromPool<AlphaChannel> = to_alpha_channel(&pixmap);
    make_semitransparent(&mut semitransparent_circle, alpha);
    let semitransparent_red_circle: MaybeFromPool<Pixmap> =
        paint(&semitransparent_circle, &ComparableColor::RED);
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
