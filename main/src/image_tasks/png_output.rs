use bytemuck::{cast};
use include_dir::{File};
use std::io::{Cursor, Write};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::{Path};
use std::sync::{Arc, Mutex};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use itertools::Itertools;
use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool};
use log::{error, info};
use oxipng::{Deflaters, optimize_from_memory, Options, StripChunks};
use png::{BitDepth, ColorType};

use resvg::tiny_skia::{Pixmap, PremultipliedColorU8};
use zip_next::CompressionMethod::{Deflated};
use zip_next::write::FileOptions;
use zip_next::{ZipWriter};

use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::{CloneableError};
use crate::{anyhoo, TILE_SIZE};
use crate::image_tasks::color::ComparableColor;

pub type ZipBufferRaw = Cursor<Vec<u8>>;

const PNG_BUFFER_SIZE: usize = 1024 * 1024;

lazy_static!{

    static ref ZIP_BUFFER_SIZE: usize = (*TILE_SIZE as usize) * 32 * 1024;
    // Pixels are already deflated by oxipng, but they're still compressible, probably because PNG
    // chunks are compressed independently.
    static ref PNG_ZIP_OPTIONS: FileOptions = FileOptions::default()
        .compression_method(Deflated)
        .with_zopfli_buffer(Some(PNG_BUFFER_SIZE))
        .compression_level(Some(if *TILE_SIZE < 2048 {
        264
    } else if *TILE_SIZE < 4096 {
        24
    } else {
        8
    }));
    static ref METADATA_ZIP_OPTIONS: FileOptions = FileOptions::default()
        .compression_method(Deflated)
        .compression_level(Some(264));
    pub static ref ZIP: Mutex<ZipWriter<ZipBufferRaw>> = Mutex::new(ZipWriter::new(Cursor::new(
        Vec::with_capacity(*ZIP_BUFFER_SIZE))));
    static ref PNG_BUFFER_POOL: Arc<LinearObjectPool<Vec<u8>>> = Arc::new(LinearObjectPool::new(
        || {
            info!("Allocating a PNG buffer for pool");
            Vec::with_capacity(PNG_BUFFER_SIZE)
        },
        Vec::clear
    ));
    static ref OXIPNG_OPTIONS: Options = {
        let mut options = Options::from_preset(4);
        options.deflate = if *TILE_SIZE < 128 {
            Deflaters::Zopfli {iterations: u8::MAX.try_into().unwrap() }
        } else if *TILE_SIZE < 256 {
            Deflaters::Zopfli {iterations: 15.try_into().unwrap() }
        } else if *TILE_SIZE < 2048 {
            Deflaters::Libdeflater {compression: 12}
        } else if *TILE_SIZE < 4096 {
            Deflaters::Libdeflater {compression: 11}
        } else {
            Deflaters::Libdeflater {compression: 9}
        };
        options.optimize_alpha = true;
        options.strip = StripChunks::All;
        options
    };
}

pub fn prewarm_png_buffer_pool() {
    PNG_BUFFER_POOL.pull();
}

pub fn png_output(image: MaybeFromPool<Pixmap>, omit_alpha: bool, discrete_colors: Option<Vec<ComparableColor>>, file: &Path) -> Result<(),CloneableError> {
    let data = into_png(image, omit_alpha, discrete_colors)?;
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
pub fn into_png(mut image: MaybeFromPool<Pixmap>, omit_alpha: bool,
                discrete_colors: Option<Vec<ComparableColor>>) -> Result<MaybeFromPool<Vec<u8>>, CloneableError> {
    let mut reusable = PNG_BUFFER_POOL.pull();
    let mut encoder = png::Encoder::new(reusable.deref_mut(), image.width(), image.height());
    if let Some(mut colors) = discrete_colors
            && colors.len() <= u16::MAX as usize {
        let mut palette: Vec<u8> = Vec::with_capacity(colors.len() * 3);
        let mut trns: Vec<u8> = Vec::with_capacity(
            if omit_alpha {0} else {colors.len()});
        for color in colors.iter() {
            palette.extend_from_slice( & [color.red(), color.green(), color.blue()]);
            if !omit_alpha {
                trns.push(color.alpha());
            }
        }
        encoder.set_color(ColorType::Indexed);
        encoder.set_palette(palette);
        if !omit_alpha {
            encoder.set_trns(trns);
        }
        if colors.len() <= 2 {
            encoder.set_depth(BitDepth::One);
            let mut writer = encoder.write_header()?;
            let mut bit_writer: BitWriter<_, BigEndian> =
                BitWriter::new(writer.stream_writer()?);
            let first_color: PremultipliedColorU8 = colors[0].into();
            for pixel in image.pixels() {
                bit_writer.write_bit(*pixel == first_color).unwrap();
            }
            bit_writer.flush()?;
        } else {
            let depth = if colors.len() <= 4 {
                BitDepth::Two
            } else if colors.len() <= 16 {
                BitDepth::Four
            } else if colors.len() <= 256 {
                BitDepth::Eight
            } else {
                BitDepth::Sixteen
            };
            encoder.set_depth(depth);
            let depth: u32 = match depth {
                BitDepth::One => 1,
                BitDepth::Two => 2,
                BitDepth::Four => 4,
                BitDepth::Eight => 8,
                BitDepth::Sixteen => 16
            };
            let mut writer = encoder.write_header()?;
            let mut bit_writer: BitWriter<_, BigEndian> = BitWriter::new(
                writer.stream_writer()?);
            colors.sort();
            for pixel in image.pixels() {
                let pixel_color: ComparableColor = (*pixel).into();
                let color_index = colors.binary_search(&pixel_color)
                    .or_else(|_| Err(anyhoo!("Unexpected color {}; expected palette was {}",
                        pixel_color, colors.iter().join(","))))?;
                bit_writer.write(depth, color_index as u16).unwrap();
            }
            bit_writer.flush()?;
        }
    } else {
        for pixel in image.pixels_mut() {
            unsafe {
                // Treat this PremultipliedColorU8 slice as a ColorU8 slice
                *pixel = mem::transmute(pixel.demultiply());
            }
        }
        encoder.set_depth(BitDepth::Eight);
        if omit_alpha {
            encoder.set_color(ColorType::Rgb);
            let mut writer = encoder.write_header()?;
            let mut data: Vec<u8> = Vec::with_capacity(3 * image.pixels().len());
            for pixel in image.pixels() {
                data.extend(&cast::<PremultipliedColorU8, [u8; 4]>(*pixel)[0..3]);
            }
            writer.write_image_data(&data)?;
        } else {
            encoder.set_color(ColorType::Rgba);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(image.data())?;
        }
    }
    match optimize_from_memory(reusable.deref(), &OXIPNG_OPTIONS) {
        Ok(optimized) => Ok(MaybeFromPool::NotFromPool(optimized)),
        Err(e) => {
            error!("Error from oxipng: {}", e);
            Ok(MaybeFromPool::FromPool {reusable})
        }
    }
}