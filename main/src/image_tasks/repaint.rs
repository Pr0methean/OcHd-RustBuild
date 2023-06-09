use std::sync::Arc;
use lazy_static::lazy_static;
use lockfree_object_pool::LinearObjectPool;
use log::info;
use resvg::tiny_skia::{Mask, Paint, Pixmap, Rect, Transform};
use crate::{anyhoo, TILE_SIZE};

use crate::image_tasks::{allocate_pixmap_empty, MaybeFromPool};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::MaybeFromPool::NotFromPool;
use crate::image_tasks::task_spec::{CloneableError};

lazy_static!{
    static ref ALPHA_CHANNEL_POOL: Arc<LinearObjectPool<Mask>> = Arc::new(LinearObjectPool::new(
        || {
            info!("Allocating a Mask for pool");
            Mask::new(*TILE_SIZE, *TILE_SIZE).expect("Failed to allocate a Mask for pool")
        },
        |_alpha_channel| {} // don't need to reset
    ));
}

pub fn prewarm_mask_pool() {
    ALPHA_CHANNEL_POOL.pull();
}

impl Clone for MaybeFromPool<Mask> {
    fn clone(&self) -> Self {
        let width = self.width();
        let height = self.height();
        info!("Copying a Mask of size {}x{}", width, height);
        let mut clone = allocate_mask_for_overwrite(width, height);
        clone.data_mut().copy_from_slice(self.data());
        clone
    }
}

fn allocate_mask_for_overwrite(width: u32, height: u32) -> MaybeFromPool<Mask> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        info!("Borrowing a Mask from pool");
        let pool: &Arc<LinearObjectPool<Mask>> = &ALPHA_CHANNEL_POOL;
        MaybeFromPool::FromPool {
            reusable: pool.pull(),
        }
    } else {
        info!("Allocating a Mask outside pool for unusual size {}x{}", width, height);
        NotFromPool(Mask::new(width, height).expect("Failed to allocate a Mask outside pool"))
    }
}

pub fn pixmap_to_mask(value: &Pixmap) -> MaybeFromPool<Mask> {
    let mut mask = allocate_mask_for_overwrite(value.width(), value.height());
    let mask_pixels = mask.data_mut();
    for (index, pixel) in value.pixels().iter().enumerate() {
        mask_pixels[index] = pixel.alpha();
    }
    mask
}

/// Applies the given [color] to the given [input](alpha channel).
pub fn paint(input: &Mask, color: ComparableColor) -> Result<Box<MaybeFromPool<Pixmap>>, CloneableError> {
    let mut output = allocate_pixmap_empty(input.width(), input.height());
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red(), color.green(), color.blue(), color.alpha());
    output.fill_rect(Rect::from_xywh(0.0, 0.0, input.width() as f32, input.height() as f32)
                         .ok_or(anyhoo!("Failed to create rectangle for paint()"))?,
                     &paint, Transform::default(),
                     Some(input));
    Ok(Box::new(output))
}

#[test]
fn test_alpha_channel() {
    use resvg::tiny_skia::FillRule;
    use resvg::tiny_skia::PathBuilder;

    let side_length = 128;
    let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    pixmap.fill_path(&circle, &Paint::default(),
                     FillRule::EvenOdd, Transform::default(), None);
    let alpha_channel = pixmap_to_mask(&*pixmap);
    let pixmap_pixels = pixmap.pixels();
    let alpha_pixels = alpha_channel.data();
    for index in 0usize..((side_length * side_length) as usize) {
        assert_eq!(alpha_pixels[index], pixmap_pixels[index].alpha());
    }
}

#[test]
fn test_paint() {
    use resvg::tiny_skia::{FillRule, Paint};
    use resvg::tiny_skia::{PathBuilder, Transform};
    use crate::image_tasks::MaybeFromPool::NotFromPool;
    use crate::image_tasks::color::c;

    let side_length = 128;
    let pixmap = &mut NotFromPool(Pixmap::new(side_length, side_length).unwrap());
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    pixmap.fill_path(&circle, &Paint::default(),
                     FillRule::EvenOdd, Transform::default(), None);
    let alpha_channel = pixmap_to_mask(pixmap);
    let repainted_alpha: u8 = 0xcf;
    let repainted_alpha_fraction = 0xcf as f32 / u8::MAX as f32;
    let repainted_red: Box<MaybeFromPool<Pixmap>>
        = paint(&alpha_channel, c(0xff0000) * repainted_alpha_fraction).unwrap();
    let pixmap_pixels = pixmap.pixels();
    let repainted_pixels = repainted_red.pixels();
    for index in 0usize..((side_length * side_length) as usize) {
        let expected_alpha: u8 = (u16::from(repainted_alpha)
            * u16::from(pixmap_pixels[index].alpha()) / 0xff) as u8;
        let actual_alpha = repainted_pixels[index].alpha();
        assert!(actual_alpha.abs_diff(expected_alpha) <= 1,
            "expected alpha of {} but found {}", expected_alpha, actual_alpha);
        if expected_alpha > 0 {
            // premultiplied
            assert_eq!(repainted_pixels[index].red(), repainted_pixels[index].alpha());

            assert_eq!(repainted_pixels[index].green(), 0);
            assert_eq!(repainted_pixels[index].blue(), 0);
        }
    }
}