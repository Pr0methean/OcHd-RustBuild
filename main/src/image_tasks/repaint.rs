use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use lazy_static::lazy_static;
use lockfree_object_pool::LinearObjectPool;
use log::info;
use tiny_skia::{Mask, Paint, Pixmap};
use tiny_skia_path::{Rect, Transform};
use crate::{anyhoo, TILE_SIZE};

use crate::image_tasks::{allocate_pixmap, MaybeFromPool};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::MaybeFromPool::NotFromPool;
use crate::image_tasks::task_spec::{CloneableError};

lazy_static!{
    static ref ALPHA_CHANNEL_POOL: Arc<LinearObjectPool<Mask>> = Arc::new(LinearObjectPool::new(
        || Mask::new(*TILE_SIZE, *TILE_SIZE).unwrap(),
        |_alpha_channel| {} // don't need to reset
    ));
}

impl Clone for MaybeFromPool<Mask> {
    fn clone(&self) -> Self {
        let mut clone = allocate_alpha_channel(self.width(), self.height());
        clone.data_mut().copy_from_slice(self.data());
        clone
    }
}

fn allocate_alpha_channel(width: u32, height: u32) -> MaybeFromPool<Mask> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        let pool: &Arc<LinearObjectPool<Mask>> = &ALPHA_CHANNEL_POOL;
        MaybeFromPool::FromPool {
            reusable: pool.pull_owned(),
        }
    } else {
        NotFromPool(Mask::new(width, height).unwrap())
    }
}

pub fn pixmap_to_mask(value: &Pixmap) -> MaybeFromPool<Mask> {
    info!("Starting task: convert Pixmap to AlphaChannel");
    let mut mask = allocate_alpha_channel(value.width(), value.height());
    let mask_pixels = mask.data_mut();
    for (index, pixel) in value.pixels().iter().enumerate() {
        mask_pixels[index] = pixel.alpha();
    }
    info!("Finishing task: convert Pixmap to AlphaChannel");
    mask
}

/// Applies the given [color] to the given [input](alpha channel).
pub fn paint(input: &Mask, color: ComparableColor) -> Result<Box<MaybeFromPool<Pixmap>>, CloneableError> {
    info!("Starting task: paint with color {}", color);
    let mut output = allocate_pixmap(input.width(), input.height());
    let mut paint = Paint::default();
    paint.set_color(color.into());
    output.fill_rect(Rect::from_ltrb(0.0, 0.0, input.width() as f32, input.height() as f32)
                         .ok_or(anyhoo!("Failed to create rectangle for paint()"))?,
                     &paint, Transform::default(),
                     Some(input));
    info!("Finishing task: paint with color {}", color);
    Ok(Box::new(output))
}

#[test]
fn test_alpha_channel() {
    use tiny_skia::FillRule;
    use tiny_skia_path::PathBuilder;

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
    use tiny_skia::{ColorU8, FillRule, Paint};
    use tiny_skia_path::{PathBuilder, Transform};
    use crate::image_tasks::MaybeFromPool::NotFromPool;

    let side_length = 128;
    let pixmap = &mut NotFromPool(Pixmap::new(side_length, side_length).unwrap());
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    pixmap.fill_path(&circle, &Paint::default(),
                     FillRule::EvenOdd, Transform::default(), None);
    let alpha_channel = pixmap_to_mask(pixmap);
    let repainted_alpha: u8 = 0xcf;
    let red = ColorU8::from_rgba(0xff, 0, 0, repainted_alpha);
    let repainted_red: Box<MaybeFromPool<Pixmap>>
        = paint(&alpha_channel, ComparableColor::from(red)).unwrap();
    let pixmap_pixels = pixmap.pixels();
    let repainted_pixels = repainted_red.pixels();
    for index in 0usize..((side_length * side_length) as usize) {
        let expected_alpha: u8 = (u16::from(repainted_alpha)
            * u16::from(pixmap_pixels[index].alpha()) / 0xff) as u8;
        assert!(repainted_pixels[index].alpha().abs_diff(expected_alpha) <= 1);
        if expected_alpha > 0 {
            // premultiplied
            assert_eq!(repainted_pixels[index].red(), repainted_pixels[index].alpha());

            assert_eq!(repainted_pixels[index].green(), 0);
            assert_eq!(repainted_pixels[index].blue(), 0);
        }
    }
}