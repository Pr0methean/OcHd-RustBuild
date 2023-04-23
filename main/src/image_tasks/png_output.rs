use std::fs::{File};
use std::io::{copy, Cursor, Write};
use std::mem;
use std::mem::transmute_copy;
use std::ops::DerefMut;
use std::path::{PathBuf};
use std::ptr::replace;
use std::sync::{Mutex};
use lazy_static::lazy_static;
use log::info;

use tiny_skia::{Pixmap};
use tracing::instrument;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};
use zip::CompressionMethod::Bzip2;
use zip::read::ZipFile;

use crate::{anyhoo};
use crate::image_tasks::task_spec::{CloneableError};

pub type ZipBufferRaw = Cursor<Vec<u8>>;

lazy_static!{
    static ref ZIP_OPTIONS: FileOptions = FileOptions::default()
        .compression_method(Bzip2)
        .compression_level(Some(9));
    pub static ref ZIP: Mutex<ZipWriter<ZipBufferRaw>> = Mutex::new(ZipWriter::new(Cursor::new(
        Vec::with_capacity(1024 * 1024)
    )));
}

#[instrument]
pub fn png_output(image: Pixmap, file: PathBuf) -> Result<(),CloneableError> {
    let file_string = file.to_string_lossy();
    info!("Starting task: write {}", file_string);
    let data = encode_png(image).map_err(|error| anyhoo!(error))?;
    let mut zip = ZIP.lock().map_err(|error| anyhoo!(error.to_string()))?;
    let writer = zip.deref_mut();
    writer.start_file(file.to_string_lossy(), *ZIP_OPTIONS).map_err(|error| anyhoo!(error))?;
    writer.write(&data).map_err(|error| anyhoo!(error))?;
    drop(zip);
    info!("Finishing task: write {}", file_string);
    Ok(())
}

pub fn copy_out_to_out(source: PathBuf, dest: PathBuf) -> Result<(),CloneableError> {
    let source_string = source.to_string_lossy();
    let dest_string = dest.to_string_lossy();
    info!("Starting task: copy {} to {}", &source_string, &dest_string);
    let mut zip = ZIP.lock().map_err(|error| anyhoo!(error.to_string()))?;
    let writer = zip.deref_mut();
    // Need to finish and consume the writer before switching to a reader. (For some reason, refs to
    // the underlying buffer don't reliably see updates, even when using a mutex so the read
    // happens-after the write.)
    let file_so_far = writer.finish().map_err(|error| anyhoo!(error))?;
    let mut reader = ZipArchive::new(Cursor::new(file_so_far.get_ref()))
        .map_err(|error| anyhoo!(error))?;
    let source_file = reader.by_name(&source_string).map_err(|error| anyhoo!(error))?;
    // To copy from within the same file, we need to borrow and mutably borrow the underlying Cursor
    // at the same time, hence the need for unsafe code.
    let source_file_copy: ZipFile = unsafe {
        transmute_copy(&source_file)
    };
    drop(source_file);
    unsafe {
        replace(writer, ZipWriter::new_append(file_so_far).map_err(|error| anyhoo!(error))?);
    }
    writer.raw_copy_file_rename(source_file_copy, &*dest_string).map_err(|error| anyhoo!(error))?;
    drop(zip);
    info!("Finishing task: copy {} to {}", &source_string, &dest_string);
    Ok(())
}

pub fn copy_in_to_out(source: PathBuf, dest: PathBuf) -> Result<(),CloneableError> {
    let source_string = source.to_string_lossy();
    let dest_string = dest.to_string_lossy();
    info!("Starting task: copy {} to {}", source_string, dest_string);
    let mut source_file = File::open(&source).map_err(|error| anyhoo!(error))?;
    let mut zip = ZIP.lock().map_err(|error| anyhoo!(error.to_string()))?;
    let writer = zip.deref_mut();
    writer.start_file(dest_string.clone(), *ZIP_OPTIONS).map_err(|error| anyhoo!(error))?;
    copy(&mut source_file, writer).map_err(|error| anyhoo!(error))?;
    drop(zip);
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