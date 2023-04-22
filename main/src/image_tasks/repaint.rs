use std::ops::Mul;

use cached::proc_macro::cached;
use log::info;
use tiny_skia::{Pixmap, PremultipliedColorU8};
use tracing::instrument;


use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::make_semitransparent::create_alpha_array;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct AlphaChannel {
    pixels: Vec<u8>,
    width: u32,
    height: u32
}

impl AlphaChannel {
    pub(crate) fn pixels(&self) -> &[u8] {
        return self.pixels.as_slice();
    }

    pub(crate) fn pixels_mut(&mut self) -> &mut [u8] {
        return self.pixels.as_mut_slice();
    }
}

impl From<&Pixmap> for AlphaChannel {
    fn from(value: &Pixmap) -> Self {
        info!("Starting task: convert Pixmap to AlphaChannel");
        let pixels =
            value.pixels().iter()
                .map(|pixel| pixel.alpha())
                .collect::<Vec<u8>>();
        info!("Finishing task: convert Pixmap to AlphaChannel");
        AlphaChannel {
            width: value.width(),
            height: value.height(),
            pixels
        }
    }
}

#[instrument]
pub fn to_alpha_channel(pixmap: &Pixmap) -> AlphaChannel {
    AlphaChannel::from(pixmap)
}

impl Mul<f32> for AlphaChannel {
    type Output = AlphaChannel;

    fn mul(self, rhs: f32) -> Self::Output {
        let alpha_array = create_alpha_array(rhs.into());
        let mut output = self;
        let output_pixels = output.pixels_mut();
        for index in 0..output_pixels.len() {
            output_pixels[index] = alpha_array[output_pixels[index] as usize];
        }
        output
    }
}

impl Mul<ComparableColor> for AlphaChannel {
    type Output = Pixmap;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        paint(&self, &rhs)
    }
}

#[cached(sync_writes = true)]
fn create_paint_array(color: ComparableColor) -> [PremultipliedColorU8; 256] {
    (0u16..256u16)
        .map (|alpha| {
            let alpha_fraction = f32::from(alpha) / 255.0;
            let color_with_alpha: PremultipliedColorU8 = (color * alpha_fraction).into();
            color_with_alpha
        })
        .collect::<Vec<PremultipliedColorU8>>().try_into().unwrap()
}

/// Applies the given [color] to the given [input](alpha channel).
#[instrument]
pub fn paint(input: &AlphaChannel, color: &ComparableColor) -> Pixmap {
    info!("Starting task: paint with color {}", color);
    let paint_array = create_paint_array(*color);
    let input_pixels = input.pixels();
    let mut output = Pixmap::new(input.width, input.height).unwrap();
    let output_pixels = output.pixels_mut();
    output_pixels.copy_from_slice(&(input_pixels.iter()
        .map(|input_pixel| {
            paint_array[usize::from(*input_pixel)]
        }).collect::<Vec<PremultipliedColorU8>>()[..]));
    info!("Finishing task: paint with color {}", color);
    output
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
        let repainted_red: Pixmap = paint(&alpha_channel, &ComparableColor::from(red))
            .try_into().unwrap();
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
