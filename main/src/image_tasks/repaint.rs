use std::iter;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Mul};
use std::sync::Arc;
use cached::proc_macro::cached;
use lazy_static::lazy_static;
use log::info;
use lockfree_object_pool::LinearObjectPool;
use tiny_skia::{Pixmap, PremultipliedColorU8};

use tracing::instrument;

use crate::{TILE_SIZE};
use crate::image_tasks::{allocate_pixmap, MaybeFromPool};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::make_semitransparent::create_alpha_array;
use crate::image_tasks::MaybeFromPool::NotFromPool;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct AlphaChannel {
    pixels: Vec<u8>,
    width: u32,
    height: u32
}

impl AlphaChannel {
    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub(crate) fn pixels_mut(&mut self) -> &mut [u8] {
        return self.pixels.deref_mut();
    }

    fn new(width: u32, height: u32) -> AlphaChannel {
        let mut pixels: Vec<u8> = Vec::with_capacity((width * height) as usize);
        pixels.extend(iter::repeat(0).take((width * height) as usize));
        AlphaChannel {pixels, width, height}
    }
}

lazy_static!{
    static ref ALPHA_CHANNEL_POOL: Arc<LinearObjectPool<AlphaChannel>> = Arc::new(LinearObjectPool::new(
        || AlphaChannel::new(*TILE_SIZE, *TILE_SIZE),
        |_alpha_channel| {} // don't need to reset
    ));
}

impl Clone for MaybeFromPool<AlphaChannel> {
    fn clone(&self) -> Self {
        let mut clone = allocate_alpha_channel(self.width, self.height);
        self.deref().clone_into(clone.deref_mut());
        clone
    }
}

#[instrument]
pub fn to_alpha_channel(pixmap: &MaybeFromPool<Pixmap>) -> MaybeFromPool<AlphaChannel> {
    let width = pixmap.width();
    let height = pixmap.height();
    let mut output: MaybeFromPool<AlphaChannel> = allocate_alpha_channel(width, height);
    for (index, pixel) in pixmap.pixels().iter().enumerate() {
        output.pixels[index] = pixel.alpha();
    }
    output
}

fn allocate_alpha_channel(width: u32, height: u32) -> MaybeFromPool<AlphaChannel> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        let pool: &Arc<LinearObjectPool<AlphaChannel>> = &ALPHA_CHANNEL_POOL;
        MaybeFromPool::FromPool {
            reusable: pool.pull_owned(),
        }
    } else {
        NotFromPool(AlphaChannel::new(width, height))
    }
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
    type Output = MaybeFromPool<Pixmap>;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        paint(&self, &rhs)
    }
}

#[cached(sync_writes = true)]
fn create_paint_array(color: ComparableColor) -> [PremultipliedColorU8; 256] {
    return (0u16..256u16)
        .map (|alpha| {
            let alpha_fraction = f32::from(alpha) / 255.0;
            let color_with_alpha: PremultipliedColorU8 = (color * alpha_fraction).into();
            color_with_alpha
        })
        .collect::<Vec<PremultipliedColorU8>>().try_into().unwrap();
}

/// Applies the given [color] to the given [input](alpha channel).
#[instrument]
pub fn paint(input: &AlphaChannel, color: &ComparableColor) -> MaybeFromPool<Pixmap> {
    info!("Starting task: paint with color {}", color);
    let paint_array = create_paint_array(*color);
    let input_pixels = input.pixels();
    let mut output = allocate_pixmap(input.width, input.height);
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
        let mut pixmap = &mut NotFromPool(Pixmap::new(side_length, side_length).unwrap());
        let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
        pixmap.fill_path(&circle, &Paint::default(),
                         FillRule::EvenOdd, Transform::default(), None);
        let alpha_channel = to_alpha_channel(&pixmap);
        let pixmap_pixels = pixmap.deref().pixels();
        let alpha_pixels = alpha_channel.pixels();
        for index in 0usize..((side_length * side_length) as usize) {
            assert_eq!(alpha_pixels[index], pixmap_pixels[index].alpha());
        }
    }

    #[test]
    fn test_paint() {
        let side_length = 128;
        let pixmap = &mut NotFromPool(Pixmap::new(side_length, side_length).unwrap());
        let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
        pixmap.fill_path(&circle, &Paint::default(),
                         FillRule::EvenOdd, Transform::default(), None);
        let alpha_channel = to_alpha_channel(&pixmap);
        let repainted_alpha: u8 = 0xcf;
        let red = ColorU8::from_rgba(0xff, 0, 0, repainted_alpha);
        let repainted_red: MaybeFromPool<Pixmap> = paint(&alpha_channel, &ComparableColor::from(red));
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
