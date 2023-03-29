use anyhow::anyhow;
use tiny_skia::{Pixmap, PremultipliedColorU8};
use crate::image_tasks::color::ComparableColor;
use cached::proc_macro::cached;

#[derive(Clone, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct AlphaChannel {
    pixels: Vec<u8>,
    width: u32,
    height: u32
}

impl AlphaChannel {
    fn pixels(&self) -> &[u8] {
        return self.pixels.as_slice();
    }
}

impl From<&Pixmap> for AlphaChannel {
    fn from(value: &Pixmap) -> Self {
        let pixels =
            value.pixels().into_iter()
                .map(|pixel| pixel.alpha())
                .collect::<Vec<u8>>();
        return AlphaChannel {
            width: value.width(),
            height: value.height(),
            pixels
        }
    }
}

#[cached(sync_writes = true)]
fn create_paint_array(color: ComparableColor) -> [PremultipliedColorU8; 256] {
    return (0u16..256u16).into_iter()
        .map (|alpha| {
            let alpha_fraction = f32::from(alpha) / 255.0;
            let color_with_alpha: PremultipliedColorU8 = (color * alpha_fraction).into();
            return color_with_alpha;
        })
        .collect::<Vec<PremultipliedColorU8>>().try_into().unwrap();
}

pub fn paint(input: AlphaChannel, color: ComparableColor) -> Result<Pixmap, anyhow::Error> {
    let paint_array = create_paint_array(color);
    let input_pixels = input.pixels;
    let mut output = Pixmap::new(input.width, input.height)
        .ok_or(anyhow!("Failed to create output Pixmap"))?;
    let output_pixels = output.pixels_mut();
    output_pixels.copy_from_slice(&(input_pixels.iter()
        .map(|input_pixel| {
            paint_array[usize::from(*input_pixel)]
        }).collect::<Vec<PremultipliedColorU8>>()[..]));
    return Ok(output);
}

#[cfg(test)]
pub mod tests {
    use tiny_skia::{ColorU8, FillRule, Paint};
    use tiny_skia_path::{PathBuilder, Transform};
    use super::*;

    #[test]
    fn test_alpha_channel() {
        let side_length = 128;
        let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
        let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
        pixmap.fill_path(&circle, &Paint::default(),
                         FillRule::EvenOdd, Transform::default(), None);
        let alpha_channel = AlphaChannel::from(&*pixmap);
        let pixmap_pixels = pixmap.pixels();
        let alpha_pixels = alpha_channel.pixels();
        for index in 0usize..((side_length * side_length) as usize) {
            assert_eq!(alpha_pixels[index], pixmap_pixels[index].alpha());
        }
    }

    #[test]
    fn test_paint() {
        let side_length = 128;
        let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
        let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
        pixmap.fill_path(&circle, &Paint::default(),
                         FillRule::EvenOdd, Transform::default(), None);
        let alpha_channel = AlphaChannel::from(&*pixmap);
        let repainted_alpha: u8 = 0xcf;
        let red = ColorU8::from_rgba(0xff, 0, 0, repainted_alpha);
        let repainted_red = paint(alpha_channel, ComparableColor::from(red))
            .unwrap();
        let pixmap_pixels = pixmap.pixels();
        let repainted_pixels = repainted_red.pixels();
        for index in 0usize..((side_length * side_length) as usize) {
            let expected_alpha: u8 = (u16::from(repainted_alpha)
                * u16::from(pixmap_pixels[index].alpha()) / 0xff) as u8;
            assert_eq!(repainted_pixels[index].alpha(), expected_alpha);
            if expected_alpha > 0 {
                assert_eq!(repainted_pixels[index].red(), expected_alpha); // premultiplied
                assert_eq!(repainted_pixels[index].green(), 0);
                assert_eq!(repainted_pixels[index].blue(), 0);
            }
        }
    }
}
