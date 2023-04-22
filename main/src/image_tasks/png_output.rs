use std::collections::HashSet;
use std::fs::{File};
use std::io::{copy, Cursor, Write};
use std::mem;
use std::ops::DerefMut;
use std::path::{PathBuf};
use std::sync::{Mutex, RwLock};
use lazy_static::lazy_static;
use log::info;

use tiny_skia::{Pixmap};
use tracing::instrument;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::{anyhoo};
use crate::image_tasks::task_spec::{CloneableError};

lazy_static!{
    static ref ZIP_OPTIONS: FileOptions = FileOptions::default().compression_level(Some(9));
    static ref MADE_DIRS: RwLock<HashSet<PathBuf>> = RwLock::new(HashSet::new());
    pub static ref ZIP_BUFFER: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(1024 * 1024));
    static ref ZIP: Mutex<(ZipWriter<Cursor<Vec<u8>>>, ZipArchive<Cursor<Vec<u8>>>)> = {
        Mutex::new((
            ZipWriter::new(ZIP_BUFFER.to_owned()),
            ZipArchive::new(ZIP_BUFFER.to_owned()).expect("Failed to open zip file for reading")
        ))
    };
}


#[instrument]
pub fn png_output(image: Pixmap, file: PathBuf) -> Result<(),CloneableError> {
    let file_string = file.to_string_lossy();
    info!("Starting task: write {}", file_string);
    let data = encode_png(image).map_err(|error| anyhoo!(error))?;
    let mut zip = ZIP.lock().map_err(|error| anyhoo!(error.to_string()))?;
    let (ref mut writer, _) = zip.deref_mut();
    writer.start_file(file.to_string_lossy(), *ZIP_OPTIONS).map_err(|error| anyhoo!(error))?;
    writer.write(&data).map_err(|error| anyhoo!(error))?;
    info!("Finishing task: write {}", file_string);
    Ok(())
}

pub fn copy_out_to_out(source: PathBuf, dest: PathBuf) -> Result<(),CloneableError> {
    let source_string = source.to_string_lossy();
    let dest_string = dest.to_string_lossy();
    info!("Starting task: copy {} to {}", &source_string, &dest_string);
    let mut zip = ZIP.lock().map_err(|error| anyhoo!(error.to_string()))?;
    let (ref mut writer, ref mut reader) = zip.deref_mut();
    let source_file = reader.by_name(&source_string).map_err(|error| anyhoo!(error))?;
    writer.raw_copy_file_rename(source_file, &*dest_string).map_err(|error| anyhoo!(error))?;
    info!("Finishing task: copy {} to {}", &source_string, &dest_string);
    Ok(())
}

pub fn copy_in_to_out(source: PathBuf, dest: PathBuf) -> Result<(),CloneableError> {
    let source_string = source.to_string_lossy();
    let dest_string = dest.to_string_lossy();
    info!("Starting task: copy {} to {}", source_string, dest_string);
    let mut source_file = File::open(&source).map_err(|error| anyhoo!(error))?;
    let mut zip = ZIP.lock().map_err(|error| anyhoo!(error.to_string()))?;
    let (ref mut writer, _) = zip.deref_mut();
    writer.start_file(dest_string.clone(), *ZIP_OPTIONS).map_err(|error| anyhoo!(error))?;
    copy(&mut source_file, writer).map_err(|error| anyhoo!(error))?;
    info!("Finishing task: copy {} to {}", source_string, dest_string);
    Ok(())
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
        writer.write_image_data(image.data())?;
    }

    Ok(data)
}