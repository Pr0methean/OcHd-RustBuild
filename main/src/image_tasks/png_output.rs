use std::fs::{create_dir_all, write};
use std::mem;
use std::os::unix::fs::symlink;
use std::path::{PathBuf};
use log::info;

use tiny_skia::{Pixmap, PremultipliedColorU8};
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::CloneableError;

#[instrument]
pub fn png_output(image: MaybeFromPool<Pixmap>, files: &Vec<PathBuf>) -> Result<(),CloneableError> {
    let file_strings: Vec<String> = files.iter().map(|path| path.to_string_lossy().to_string()).collect();
    let files_string = file_strings.join(", ");
    drop(file_strings);
    info!("Starting task: write {}", files_string);
    let (first_file, extra_files) = files.split_first()
            .expect("Tried to write PNG to empty list of files");
    create_dir_all(first_file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
    let data = encode_png(image).map_err(|error| anyhoo!(error))?;
    write(first_file, data).map_err(|error| anyhoo!(error))?;
    for file in extra_files {
        create_dir_all(first_file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
        symlink(first_file, file).map_err(|error| anyhoo!(error))?;
    }
    info!("Finishing task: write {}", files_string);
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