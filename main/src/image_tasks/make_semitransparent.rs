use cached::proc_macro::cached;
use ordered_float::OrderedFloat;
use resvg::tiny_skia::{Mask};

#[cached(sync_writes = true)]
pub(crate) fn create_alpha_array(out_alpha: OrderedFloat<f32>) -> [u8; 256] {
    let mut alpha_array = [0u8; 256];
    for in_alpha in 1..256u16 {
        alpha_array[in_alpha as usize] = (*out_alpha * f32::from(in_alpha) + 0.5) as u8
    }
    alpha_array
}

/// Multiplies the opacity of all pixels in the [input](given pixmap) by a given [alpha].
pub fn make_semitransparent(input: &mut Mask, alpha: f32) {

    let alpha_array = create_alpha_array(alpha.into());
    let pixels = input.data_mut();
    for pixel in pixels {
        *pixel = alpha_array[*pixel as usize];
    }
}

#[test]
fn test_make_semitransparent() {
    use resvg::tiny_skia::{Color, FillRule, Paint, Pixmap};
    use tiny_skia_path::{PathBuilder, Transform};
    use crate::image_tasks::MaybeFromPool;
    use crate::image_tasks::repaint::paint;
    use crate::image_tasks::repaint::pixmap_to_mask;
    use crate::image_tasks::color::ComparableColor;

    let alpha = 0.5;
    let alpha_multiplier = (alpha * f32::from(u8::MAX)) as u16;
    let side_length = 128;
    let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    let mut red_paint = Paint::default();
    let red = Color::from_rgba8(255, 0, 0, 255);
    red_paint.set_color(red);
    pixmap.fill_path(&circle, &red_paint,
                     FillRule::EvenOdd, Transform::default(), None);
    let pixmap_pixels = pixmap.pixels().to_owned();
    let mut semitransparent_circle: MaybeFromPool<Mask> = pixmap_to_mask(pixmap);
    make_semitransparent(&mut semitransparent_circle, alpha);
    let semitransparent_red_circle: Box<MaybeFromPool<Pixmap>> =
        paint(&semitransparent_circle, ComparableColor::RED).unwrap();
    let semitransparent_pixels = semitransparent_red_circle.pixels();
    for index in 0usize..((side_length * side_length) as usize) {
        let expected_alpha: u8 = (u16::from(alpha_multiplier
            * u16::from(pixmap_pixels[index].alpha()) / 0xff)) as u8;
        assert!(semitransparent_pixels[index].alpha().abs_diff(expected_alpha) <= 1);
        if expected_alpha > 0 {
            assert!(semitransparent_pixels[index].red().abs_diff(expected_alpha) <= 1);
            // premultiplied
            assert_eq!(semitransparent_pixels[index].green(), 0);
            assert_eq!(semitransparent_pixels[index].blue(), 0);
        }
    }
}
