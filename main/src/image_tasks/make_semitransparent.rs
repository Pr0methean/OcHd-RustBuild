use cached::proc_macro::cached;
use log::info;
use ordered_float::OrderedFloat;
use tiny_skia::{Mask};
use crate::image_tasks::task_spec::CloneableError;

#[cached(sync_writes = true)]
pub(crate) fn create_alpha_array(out_alpha: OrderedFloat<f32>) -> Result<[u8; 256], CloneableError> {
    (0u16..256u16)
        .map (|alpha| (out_alpha.0 * f32::from(alpha) + 0.5) as u8)
        .collect::<Vec<u8>>().try_into()?
}

/// Multiplies the opacity of all pixels in the [input](given pixmap) by a given [alpha].
pub fn make_semitransparent(input: &mut Mask, alpha: f32) {

    info!("Starting task: make semitransparent with alpha {}", alpha);
    let alpha_array = create_alpha_array(alpha.into())?;
    let pixels = input.data_mut();
    for pixel in pixels {
        *pixel = alpha_array[*pixel as usize];
    }
    info!("Finishing task: make semitransparent with alpha {}", alpha);
}

#[test]
fn test_make_semitransparent() {
    use tiny_skia::{Color, FillRule, Paint, Pixmap};
    use tiny_skia_path::{PathBuilder, Transform};
    use crate::image_tasks::MaybeFromPool;
    use crate::image_tasks::repaint::paint;
    use crate::image_tasks::repaint::pixmap_to_mask;

    let alpha = 0.5;
    let alpha_multiplier = (alpha * f32::from(u8::MAX)) as u16;
    let side_length = 128;
    let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    let mut red_paint = Paint::default();
    let red = Color::from_rgba(1.0, 0.0, 0.0, 1.0).unwrap();
    red_paint.set_color(red);
    pixmap.fill_path(&circle, &red_paint,
                     FillRule::EvenOdd, Transform::default(), None);
    let pixmap_pixels = pixmap.pixels().to_owned();
    let mut semitransparent_circle: MaybeFromPool<Mask> = pixmap_to_mask(pixmap);
    make_semitransparent(&mut semitransparent_circle, alpha);
    let semitransparent_red_circle: Box<MaybeFromPool<Pixmap>> =
        paint(&semitransparent_circle, red).unwrap();
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
