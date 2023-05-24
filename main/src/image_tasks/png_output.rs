use include_dir::{File};
use std::io::{Cursor, Write};
use std::mem;
use std::ops::DerefMut;
use std::path::{Path};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool, LinearReusable};
use log::info;

use resvg::tiny_skia::{Pixmap};
use zip_next::CompressionMethod::Deflated;
use zip_next::write::FileOptions;
use zip_next::{ZipWriter};

use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::{CloneableError};

pub type ZipBufferRaw = Cursor<Vec<u8>>;

lazy_static!{
    static ref ZIP_OPTIONS: FileOptions = FileOptions::default()
        .compression_method(Deflated)
        .compression_level(Some(9));
    pub static ref ZIP: Mutex<ZipWriter<ZipBufferRaw>> = Mutex::new(ZipWriter::new(Cursor::new(
        Vec::with_capacity(1024 * 1024)
    )));
    static ref PNG_BUFFER_POOL: Arc<LinearObjectPool<Vec<u8>>> = Arc::new(LinearObjectPool::new(
        || {
            info!("Allocating a PNG buffer for pool");
            Vec::with_capacity(1024 * 1024)
        },
        |vec| vec.clear()
));
}

pub fn png_output(image: MaybeFromPool<Pixmap>, file: &Path) -> Result<(),CloneableError> {
    let data = into_png(image)?;
    let mut zip = ZIP.lock()?;
    let writer = zip.deref_mut();
    writer.start_file(file.to_string_lossy(), ZIP_OPTIONS.to_owned())?;
    writer.write_all(&data)?;
    drop(zip);
    drop(data);
    Ok(())
}

pub fn copy_out_to_out(source: &Path, dest: &Path) -> Result<(),CloneableError> {
    ZIP.lock()?.deep_copy_file(&source.to_string_lossy(), &dest.to_string_lossy())?;
    Ok(())
}

pub fn copy_in_to_out(source: &File, dest: &Path) -> Result<(),CloneableError> {
    let mut zip = ZIP.lock()?;
    let writer = zip.deref_mut();
    writer.start_file(dest.to_string_lossy(), ZIP_OPTIONS.to_owned())?;
    writer.write_all(source.contents())?;
    Ok(())
}

/// Forked from https://docs.rs/tiny-skia/latest/src/tiny_skia/pixmap.rs.html#390 to eliminate the
/// copy and pre-allocate the byte vector.
pub fn into_png(mut image: MaybeFromPool<Pixmap>) -> Result<LinearReusable<'static, Vec<u8>>, png::EncodingError> {
    for pixel in image.pixels_mut() {
        unsafe {
            // Treat this PremultipliedColorU8 slice as a ColorU8 slice
            *pixel = mem::transmute(pixel.demultiply());
        }
    }

    let mut reusable = PNG_BUFFER_POOL.pull();
    let mut data = reusable.deref_mut();
    {
        let mut encoder = png::Encoder::new(&mut data, image.width(), image.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(image.data())?;
    }

    Ok(reusable)
}