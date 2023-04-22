use std::collections::HashSet;
use std::fs::{create_dir_all, hard_link, remove_dir_all, write};
use std::io::ErrorKind::NotFound;
use std::mem;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use lazy_static::lazy_static;
use log::info;

use tiny_skia::{Pixmap};
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::{CloneableError, OUT_DIR};

lazy_static!{
    static ref MADE_DIRS: RwLock<HashSet<PathBuf>> = RwLock::new(HashSet::new());
}

fn ensure_made_dir(dir: &Path) -> Result<(),CloneableError> {
    if MADE_DIRS.read().map_err(|error| anyhoo!(error.to_string()))?.contains(dir) {
        return Ok(());
    }
    let mut made_dirs = MADE_DIRS.write()
        .map_err(|error| anyhoo!(error.to_string()))?;
    // Double-checked locking
    if made_dirs.contains(dir) {
        return Ok(());
    }
    if made_dirs.is_empty() {
        let result = remove_dir_all(*OUT_DIR);
        if result.is_err_and(|err| err.kind() != NotFound) {
            panic!("Failed to delete old output directory");
        }
    }
    create_dir_all(dir).map_err(|error| anyhoo!(error))?;
    made_dirs.insert(dir.to_owned());
    Ok(())
}


#[instrument]
pub fn png_output(image: MaybeFromPool<Pixmap>, file: PathBuf) -> Result<(),CloneableError> {
    let file_string = file.to_string_lossy();
    info!("Starting task: write {}", file_string);
    let parent = file.parent().ok_or(anyhoo!("Output file has no parent"))?;
    let data = encode_png(image).map_err(|error| anyhoo!(error))?;
    ensure_made_dir(parent)?;
    write(&file, data).map_err(|error| anyhoo!(error))?;
    info!("Finishing task: write {}", file_string);
    Ok(())
}

pub fn link_with_logging(original: PathBuf, link: PathBuf, hard: bool) -> Result<(),CloneableError> {
    let description =
        format!("{} -> {}", link.to_string_lossy(), original.to_string_lossy());
    info!("Starting task: symlink {}", description);
    let parent = link.parent().ok_or(anyhoo!("Output file has no parent"))?;
    ensure_made_dir(parent)?;
    if hard {
        hard_link(original, link)
    } else {
        symlink(original, link)
    }.map_err(|error| anyhoo!(error))?;
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
        writer.write_image_data(image.data())?;
    }

    Ok(data)
}