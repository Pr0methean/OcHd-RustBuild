use std::fs::{create_dir_all, hard_link, write};
use std::mem;
use std::os::unix::fs::symlink;
use std::path::{PathBuf};
use std::sync::Arc;
use log::info;

use tiny_skia::{Pixmap};
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::task_spec::{CloneableError, CloneableLazyTask};

#[instrument]
pub fn png_output(image_task: CloneableLazyTask<Pixmap>, file: PathBuf)
        -> Result<Arc<JoinHandle<Result<(),CloneableError>>>,CloneableError> {
    let file_string = file.to_string_lossy().to_owned();
    let mkdir_target = (file.parent().unwrap()).to_owned();
    info!("Starting task: write {}", file_string);
    let file_string_in_mkdirs = file_string.to_string();
    let file_string_in_write_log = file_string.to_string();
    let mkdirs = tokio::spawn(async move {
        create_dir_all(mkdir_target)
            .map_err(|error| anyhoo!("Error writing {}: {}", file_string_in_mkdirs, error))
    });
    let image = Arc::unwrap_or_clone(image_task.into_result()?);
    let data = encode_png(*image).map_err(|error| anyhoo!(error))?;
    let file = file.to_owned();
    let write = tokio::spawn(async move {
        mkdirs.await.map_err(|error| anyhoo!(error))??;
        write(file, data).map_err(|error| anyhoo!(error))?;
        info!("Finishing task: write {}", file_string_in_write_log);
        Ok(())
    });
    Ok(Arc::new(write))
}

pub fn link_with_logging(original: PathBuf, link: PathBuf, hard: bool)
        -> Result<JoinHandle<Result<(),CloneableError>>,CloneableError> {
    let description =
        format!("{} -> {}", link.to_string_lossy(), original.to_string_lossy());
    let future = tokio::spawn(async move {
        info!("Starting task: symlink {}", description);
        create_dir_all(link.parent().unwrap()).map_err(|error| anyhoo!(error))?;
        if hard {
            hard_link(original, link)
        } else {
            symlink(original, link)
        }.map_err(|error| anyhoo!(error))?;
        info!("Finishing task: symlink {}", description);
        Ok(())
    });
    Ok(future)
}

/// Forked from https://docs.rs/tiny-skia/latest/src/tiny_skia/pixmap.rs.html#390 to eliminate the
/// copy and pre-allocate the byte vector.
pub fn encode_png(mut image: Pixmap) -> Result<Vec<u8>, png::EncodingError> {
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