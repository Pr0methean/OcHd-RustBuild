use resvg::tiny_skia::{Mask};

const fn create_alpha_multiplication_table() -> [[u8; u8::MAX as usize + 1]; u8::MAX as usize + 1] {
    let mut table = [[0u8; u8::MAX as usize + 1]; u8::MAX as usize + 1];
    let mut x = 1;
    loop {
        let mut y = 1;
        loop {
            table[x as usize][y as usize] = (((x as u16) * (y as u16) + 128) / 255) as u8;
            if y == u8::MAX {
                break;
            } else {
                y += 1;
            }
        }
        if x == u8::MAX {
            return table;
        } else {
            x += 1;
        }
    }
}

pub const ALPHA_MULTIPLICATION_TABLE: [[u8; u8::MAX as usize + 1]; u8::MAX as usize + 1]
    = create_alpha_multiplication_table();

#[test]
fn test_alpha_multiplication_table() {
    for first in 0..=u8::MAX {
        assert_eq!(ALPHA_MULTIPLICATION_TABLE[0][first as usize], 0);
        assert_eq!(ALPHA_MULTIPLICATION_TABLE[u8::MAX as usize][first as usize], first);
        for second in first..=u8::MAX {
            assert_eq!(ALPHA_MULTIPLICATION_TABLE[first as usize][second as usize],
                       ALPHA_MULTIPLICATION_TABLE[second as usize][first as usize],)
        }
    }
}

const fn create_alpha_stacking_table() -> [[u8; u8::MAX as usize + 1]; u8::MAX as usize + 1] {
    let mut table = [[u8::MAX; u8::MAX as usize + 1]; u8::MAX as usize + 1];
    let mut x = 0;
    loop {
        let mut y = 0;
        loop {
            table[x as usize][y as usize] = x + ALPHA_MULTIPLICATION_TABLE[(u8::MAX - x) as usize][y as usize];
            if y == u8::MAX - 1 {
                break;
            } else {
                y += 1;
            }
        }
        if x == u8::MAX - 1 {
            return table;
        } else {
            x += 1;
        }
    }
}

pub const ALPHA_STACKING_TABLE: [[u8; u8::MAX as usize + 1]; u8::MAX as usize + 1]
    = create_alpha_stacking_table();

#[test]
fn test_alpha_stacking_table() {
    for first in 0..=u8::MAX {
        assert_eq!(ALPHA_STACKING_TABLE[0][first as usize], first);
        assert_eq!(ALPHA_STACKING_TABLE[first as usize][u8::MAX as usize], u8::MAX);
        for second in first..=u8::MAX {
            assert_eq!(ALPHA_STACKING_TABLE[first as usize][second as usize],
                       ALPHA_STACKING_TABLE[second as usize][first as usize]);
        }
    }
}

/// Multiplies the opacity of all pixels in the [input](given pixmap) by a given [alpha].
pub fn make_semitransparent(input: &mut Mask, alpha: u8) {
    let alpha_array = &ALPHA_MULTIPLICATION_TABLE[alpha as usize];
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

    let alpha = 128;
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
        let expected_alpha: u8 = (alpha as u16
            * pixmap_pixels[index].alpha() as u16 / u8::MAX as u16) as u8;
        assert!(semitransparent_pixels[index].alpha().abs_diff(expected_alpha) <= 1);
        if expected_alpha > 0 {
            assert!(semitransparent_pixels[index].red().abs_diff(expected_alpha) <= 1);
            // premultiplied
            assert_eq!(semitransparent_pixels[index].green(), 0);
            assert_eq!(semitransparent_pixels[index].blue(), 0);
        }
    }
}
