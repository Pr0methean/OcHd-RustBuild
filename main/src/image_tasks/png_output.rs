use std::fs::{create_dir_all, write};
use std::mem;
use std::os::unix::fs::symlink;
use std::path::{PathBuf};
use log::info;

use tiny_skia::{Pixmap};
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::CloneableError;

#[instrument]
pub fn png_output(image: MaybeFromPool<Pixmap>, file: PathBuf) -> Result<(),CloneableError> {
    let file_string = file.to_string_lossy();
    info!("Starting task: write {}", file_string);
    create_dir_all(file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
    let data = encode_png(image).map_err(|error| anyhoo!(error))?;
    write(file.to_owned(), data).map_err(|error| anyhoo!(error))?;
    info!("Finishing task: write {}", file_string);
    Ok(())
}

pub fn symlink_with_logging(original: PathBuf, link: PathBuf) -> Result<(),CloneableError> {
    let description =
        format!("{} -> {}", link.to_string_lossy(), original.to_string_lossy());
    info!("Starting task: symlink {}", description);
    create_dir_all(link.parent().unwrap()).map_err(|error| anyhoo!(error))?;
    symlink(original, link).map_err(|error| anyhoo!(error))?;
    info!("Finishing task: symlink {}", description);
    Ok(())
}

/// Forked from https://docs.rs/tiny-skia/latest/src/tiny_skia/pixmap.rs.html#390 to eliminate the
/// copy and pre-allocate the byte vector.
pub fn encode_png(mut image: MaybeFromPool<Pixmap>) -> Result<Vec<u8>, png::EncodingError> {
    for pixel in image.pixels_mut() {
        unsafe {
            // Treat this PremultipliedColorU8 slice as a ColorU8 slice
            *pixel = mem::transmute(pixel.demultiply());
        }
    }

    let mut data = Vec::with_capacity(1024 * 1024);
    {
        let mut encoder = png::Encoder::new(&mut data, image.width(), image.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&image.data())?;
    }

    Ok(data)
}