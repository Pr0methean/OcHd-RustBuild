use resvg::tiny_skia::{Pixmap};
use crate::image_tasks::{allocate_pixmap_for_overwrite, MaybeFromPool};
use crate::image_tasks::task_spec::CloneableError;

pub fn upscale_image(source: &Pixmap, new_width: u32) -> Result<MaybeFromPool<Pixmap>, CloneableError> {
    let scale_factor = new_width / source.width();
    let new_height = scale_factor * source.height();
    let mut out = allocate_pixmap_for_overwrite(new_width, new_height);
    for y in 0..source.height() {
        for x in 0..source.width() {
            let first_pixel = (source.width() * y + x) as usize;
            for row in 0..scale_factor as usize {
                let row_start = first_pixel + row * source.width() as usize;
                out.pixels_mut()[row_start..row_start + scale_factor as usize]
                    .fill(source.pixel(x, y).unwrap());
            }
        }
    }
    Ok(out)
}