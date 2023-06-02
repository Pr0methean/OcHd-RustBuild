use include_dir::{File};
use std::io::{Cursor, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool};
use log::{error, info};
use oxipng::{Deflaters, optimize_from_memory, Options, StripChunks};

use resvg::tiny_skia::{Pixmap};
use zip_next::CompressionMethod::{Deflated};
use zip_next::write::FileOptions;
use zip_next::{ZipWriter};

use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::{CloneableError, PngMode};
use crate::{TILE_SIZE};

pub type ZipBufferRaw = Cursor<Vec<u8>>;

const PNG_BUFFER_SIZE: usize = 1024 * 1024;

lazy_static!{

    static ref ZIP_BUFFER_SIZE: usize = (*TILE_SIZE as usize) * 32 * 1024;
    // Pixels are already deflated by oxipng, but they're still compressible, probably because PNG
    // chunks are compressed independently.
    static ref PNG_ZIP_OPTIONS: FileOptions = FileOptions::default()
        .compression_method(Deflated)
        .with_zopfli_buffer(Some(PNG_BUFFER_SIZE))
        .compression_level(Some(264));
    static ref METADATA_ZIP_OPTIONS: FileOptions = FileOptions::default()
        .compression_method(Deflated)
        .compression_level(Some(264));
    pub static ref ZIP: Mutex<ZipWriter<ZipBufferRaw>> = Mutex::new(ZipWriter::new(Cursor::new(
        Vec::with_capacity(*ZIP_BUFFER_SIZE))));
    pub static ref PNG_BUFFER_POOL: Arc<LinearObjectPool<Vec<u8>>> = Arc::new(LinearObjectPool::new(
        || {
            info!("Allocating a PNG buffer for pool");
            Vec::with_capacity(PNG_BUFFER_SIZE)
        },
        Vec::clear
    ));
    static ref OXIPNG_OPTIONS: Options = {
        let mut options = Options::from_preset(6);
        options.deflate = Deflaters::Zopfli {iterations: u8::MAX.try_into().unwrap() };
        options.optimize_alpha = true;
        options.strip = StripChunks::All;
        options
    };
}

pub fn prewarm_png_buffer_pool() {
    PNG_BUFFER_POOL.pull();
}

pub fn png_output(image: MaybeFromPool<Pixmap>, png_mode: PngMode, file: &Path) -> Result<(),CloneableError> {
    let data = into_png(image, png_mode)?;
    let mut zip = ZIP.lock()?;
    let writer = zip.deref_mut();
    writer.start_file(file.to_string_lossy(), PNG_ZIP_OPTIONS.to_owned())?;
    writer.write_all(&data)?;
    drop(zip);
    Ok(())
}

pub fn copy_out_to_out(source: &Path, dest: &Path) -> Result<(),CloneableError> {
    ZIP.lock()?.deep_copy_file(&source.to_string_lossy(), &dest.to_string_lossy())?;
    Ok(())
}

pub fn copy_in_to_out(source: &File, dest: &Path) -> Result<(),CloneableError> {
    let mut zip = ZIP.lock()?;
    let writer = zip.deref_mut();
    writer.start_file(dest.to_string_lossy(), METADATA_ZIP_OPTIONS.to_owned())?;
    writer.write_all(source.contents())?;
    Ok(())
}

/// Forked from https://docs.rs/tiny-skia/latest/src/tiny_skia/pixmap.rs.html#390 to eliminate the
/// copy and pre-allocate the byte vector.
pub fn into_png(image: MaybeFromPool<Pixmap>, png_mode: PngMode) -> Result<MaybeFromPool<Vec<u8>>, CloneableError> {
    let reusable = png_mode.write(image)?;
    match optimize_from_memory(reusable.deref(), &OXIPNG_OPTIONS) {
        Ok(optimized) => Ok(MaybeFromPool::NotFromPool(optimized)),
        Err(e) => {
            error!("Error from oxipng: {}", e);
            Ok(reusable)
        }
    }
}