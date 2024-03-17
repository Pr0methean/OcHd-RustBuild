use crate::image_tasks::cloneable::CloneableError;
use crate::image_tasks::repaint::allocate_mask_for_overwrite;
use crate::image_tasks::{allocate_pixmap_for_overwrite, MaybeFromPool};
use resvg::tiny_skia::{Mask, Pixmap};
use std::iter::repeat;

pub fn upscale_image(
    source: &Pixmap,
    new_width: u32,
) -> Result<MaybeFromPool<Pixmap>, CloneableError> {
    let scale_factor = new_width / source.width();
    let new_height = scale_factor * source.height();
    let mut out_scanline = Vec::with_capacity(new_width as usize);
    let mut out = allocate_pixmap_for_overwrite(new_width, new_height);
    for y in 0..source.height() {
        out_scanline.clear();
        for x in 0..source.width() {
            out_scanline.extend(repeat(source.pixel(x, y).unwrap()).take(scale_factor as usize));
        }
        let start_out_y = (y * scale_factor) as usize;
        let end_out_y = start_out_y + scale_factor as usize;
        for out_y in start_out_y..end_out_y {
            let start_scanline = out_y * new_width as usize;
            let end_scanline = start_scanline + new_width as usize;
            out.pixels_mut()[start_scanline..end_scanline].copy_from_slice(&out_scanline);
        }
    }
    Ok(out)
}

pub fn upscale_mask(source: &Mask, new_width: u32) -> Result<MaybeFromPool<Mask>, CloneableError> {
    let scale_factor = new_width / source.width();
    let new_height = scale_factor * source.height();
    let mut out_scanline = Vec::with_capacity(new_width as usize);
    let mut out = allocate_mask_for_overwrite(new_width, new_height);
    for y in 0..source.height() {
        out_scanline.clear();
        for x in 0..source.width() {
            out_scanline.extend(
                repeat(source.data()[(y * source.width() + x) as usize])
                    .take(scale_factor as usize),
            );
        }
        let start_out_y = (y * scale_factor) as usize;
        let end_out_y = start_out_y + scale_factor as usize;
        for out_y in start_out_y..end_out_y {
            let start_scanline = out_y * new_width as usize;
            let end_scanline = start_scanline + new_width as usize;
            out.data_mut()[start_scanline..end_scanline].copy_from_slice(&out_scanline);
        }
    }
    Ok(out)
}
