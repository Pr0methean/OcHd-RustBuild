use anyhow::anyhow;
use tiny_skia::{Pixmap, PremultipliedColorU8};
use crate::image_tasks::color::ComparableColor;
use cached::proc_macro::cached;

#[derive(Clone, Eq, Hash, Ord, PartialOrd, PartialEq)]
struct AlphaChannel {
    pixels: Vec<u8>,
    width: u32,
    height: u32
}

impl From<Pixmap> for AlphaChannel {
    fn from(value: Pixmap) -> Self {
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