use crate::{anyhoo, GRID_SIZE, TILE_SIZE};
use lockfree_object_pool::LinearObjectPool;
use log::info;
use once_cell::sync::Lazy;
use resvg::tiny_skia::{IntSize, Mask, Paint, Pixmap, Rect, Transform};

use crate::image_tasks::cloneable::{Arcow, CloneableError, SimpleArcow};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::MaybeFromPool::NotFromPool;
use crate::image_tasks::{allocate_pixmap_empty, MaybeFromPool};
static TILE_SIZE_MASK_POOL: Lazy<LinearObjectPool<Mask>> = Lazy::new(|| {
    LinearObjectPool::new(
        || {
            info!("Allocating a tile-size Mask for pool");
            let tile_size: u32 = *TILE_SIZE;
            new_mask_uninit(tile_size, tile_size)
        },
        |_| {}, // don't need to reset because we always overwrite
    )
});
static GRID_SIZE_MASK_POOL: Lazy<LinearObjectPool<Mask>> = Lazy::new(|| {
    LinearObjectPool::new(
        || {
            info!("Allocating a grid-size Mask for pool");
            new_mask_uninit(GRID_SIZE, GRID_SIZE)
        },
        |_| {}, // don't need to reset because we always overwrite
    )
});

pub fn prewarm_mask_pool() {
    GRID_SIZE_MASK_POOL.pull();
    let tile_size = *TILE_SIZE;
    if tile_size != GRID_SIZE {
        TILE_SIZE_MASK_POOL.pull();
    }
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

#[allow(clippy::uninit_vec)]
fn new_mask_uninit(width: u32, height: u32) -> Mask {
    let data_len = width as usize * height as usize;
    let mut data = Vec::with_capacity(data_len);
    unsafe {
        data.set_len(data_len);
    }
    Mask::from_vec(data, IntSize::from_wh(width, height).unwrap())
        .unwrap_or_else(|| panic!("Failed to allocate a {}x{} Mask", width, height))
}

pub fn allocate_mask_for_overwrite(width: u32, height: u32) -> MaybeFromPool<Mask> {
    if width == GRID_SIZE && height == GRID_SIZE {
        info!("Borrowing a grid-size Mask from pool");
        MaybeFromPool::FromPool {
            reusable: GRID_SIZE_MASK_POOL.pull(),
        }
    } else {
        let tile_size = *TILE_SIZE;
        if width == tile_size && height == tile_size {
            info!("Borrowing a tile-size Mask from pool");
            MaybeFromPool::FromPool {
                reusable: TILE_SIZE_MASK_POOL.pull(),
            }
        } else {
            info!(
                "Allocating a Mask outside pool for unusual size {}x{}",
                width, height
            );
            NotFromPool(new_mask_uninit(width, height))
        }
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
pub fn paint(
    input: &Mask,
    color: ComparableColor,
) -> Result<SimpleArcow<MaybeFromPool<Pixmap>>, CloneableError> {
    let mut output = allocate_pixmap_empty(input.width(), input.height());
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red(), color.green(), color.blue(), color.alpha());
    output.fill_rect(
        Rect::from_xywh(0.0, 0.0, input.width() as f32, input.height() as f32)
            .ok_or(anyhoo!("Failed to create rectangle for paint()"))?,
        &paint,
        Transform::default(),
        Some(input),
    );
    Ok(Arcow::sharing_from(output))
}

#[test]
fn test_alpha_channel() {
    use resvg::tiny_skia::FillRule;
    use resvg::tiny_skia::PathBuilder;

    let side_length = 128;
    let pixmap = &mut Pixmap::new(side_length, side_length).unwrap();
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    pixmap.fill_path(
        &circle,
        &Paint::default(),
        FillRule::EvenOdd,
        Transform::default(),
        None,
    );
    let alpha_channel = pixmap_to_mask(&*pixmap);
    let pixmap_pixels = pixmap.pixels();
    let alpha_pixels = alpha_channel.data();
    for index in 0usize..((side_length * side_length) as usize) {
        assert_eq!(alpha_pixels[index], pixmap_pixels[index].alpha());
    }
}

#[test]
fn test_paint() {
    use crate::image_tasks::color::c;
    use crate::image_tasks::MaybeFromPool::NotFromPool;
    use resvg::tiny_skia::{FillRule, Paint};
    use resvg::tiny_skia::{PathBuilder, Transform};

    let side_length = 128;
    let pixmap = &mut NotFromPool(Pixmap::new(side_length, side_length).unwrap());
    let circle = PathBuilder::from_circle(64.0, 64.0, 50.0).unwrap();
    pixmap.fill_path(
        &circle,
        &Paint::default(),
        FillRule::EvenOdd,
        Transform::default(),
        None,
    );
    let alpha_channel = pixmap_to_mask(pixmap);
    let repainted_alpha: u8 = 0xcf;
    let repainted_alpha_fraction = 0xcf as f32 / u8::MAX as f32;
    let repainted_red: SimpleArcow<MaybeFromPool<Pixmap>> =
        paint(&alpha_channel, c(0xff0000) * repainted_alpha_fraction).unwrap();
    let pixmap_pixels = pixmap.pixels();
    let repainted_pixels = repainted_red.pixels();
    for index in 0usize..((side_length * side_length) as usize) {
        let expected_alpha: u8 =
            (u16::from(repainted_alpha) * u16::from(pixmap_pixels[index].alpha()) / 0xff) as u8;
        let actual_alpha = repainted_pixels[index].alpha();
        assert!(
            actual_alpha.abs_diff(expected_alpha) <= 1,
            "expected alpha of {} but found {}",
            expected_alpha,
            actual_alpha
        );
        if expected_alpha > 0 {
            // premultiplied
            assert_eq!(
                repainted_pixels[index].red(),
                repainted_pixels[index].alpha()
            );

            assert_eq!(repainted_pixels[index].green(), 0);
            assert_eq!(repainted_pixels[index].blue(), 0);
        }
    }
}
