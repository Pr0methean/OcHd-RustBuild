use std::collections::HashMap;
use include_dir::{File};
use std::io::{Cursor, Write};
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::path::{Path};
use std::sync::{Arc, Mutex};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use bytemuck::{cast};
use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool};
use log::{error, info, warn};
use oxipng::{Deflaters, optimize_from_memory, Options, StripChunks};
use png::{BitDepth, ColorType, Encoder};

use resvg::tiny_skia::{Pixmap, PremultipliedColorU8};
use zip_next::CompressionMethod::{Deflated};
use zip_next::write::FileOptions;
use zip_next::{ZipWriter};

use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::{bit_depth_to_u32, channel_to_bit_depth, CloneableError, PngMode};
use crate::{TILE_SIZE};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::MaybeFromPool::FromPool;

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
pub fn into_png(mut image: MaybeFromPool<Pixmap>, png_mode: PngMode) -> Result<MaybeFromPool<Vec<u8>>, CloneableError> {
    let mut reusable = PNG_BUFFER_POOL.pull();
    let mut encoder = Encoder::new(reusable.deref_mut(), image.width(), image.height());
    match png_mode {
        PngMode::RgbOpaque => {
            info!("Writing an RGB PNG");
            demultiply_image(image.deref_mut());
            encoder.set_color(ColorType::Rgb);
            let mut writer = encoder.write_header()?;
            let mut data = Vec::with_capacity(3 * image.pixels().len());
            for pixel in image.pixels() {
                data.push(pixel.red());
                data.push(pixel.green());
                data.push(pixel.blue());
            }
            writer.write_image_data(&data)?;
            writer.finish()?;
        }
        PngMode::RgbWithTransparentShade(transparent) => {
            info!("Writing an RGB PNG with a transparent color");
            demultiply_image(image.deref_mut());
            encoder.set_color(ColorType::Rgb);
            encoder.set_trns(vec![0, transparent.red(), 0, transparent.green(), 0, transparent.blue()]);
            let transparent = [transparent.red(), transparent.green(), transparent.blue()];
            let mut writer = encoder.write_header()?;
            let mut data: Vec<u8> = Vec::with_capacity(3 * image.pixels().len());
            for pixel in image.pixels() {
                data.extend_from_slice(&if pixel.alpha() != u8::MAX {
                    transparent
                } else {
                    [pixel.red(), pixel.green(), pixel.blue()]
                });
            }
            writer.write_image_data(&data)?;
            writer.finish()?;
        }
        PngMode::Rgba => {
            info!("Writing an RGBA PNG");
            demultiply_image(image.deref_mut());
            encoder.set_color(ColorType::Rgba);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(image.data())?;
            writer.finish()?;
        }
        PngMode::GrayscaleOpaque(bit_depth) => {
            let depth_bits: u32 = bit_depth_to_u32(&bit_depth);
            info!("Writing {}-bit grayscale opaque PNG", depth_bits);
            encoder.set_depth(bit_depth);
            encoder.set_color(ColorType::Grayscale);
            let mut writer = encoder.write_header()?;
            let mut writer: BitWriter<_, BigEndian> = BitWriter::new(writer.stream_writer()?);
            for pixel in image.pixels() {
                writer.write(depth_bits, channel_to_bit_depth(pixel.red(), bit_depth))?;
            }
            writer.flush()?;
        }
        PngMode::GrayscaleWithTransparentShade {bit_depth, transparent_shade} => {
            let depth_bits: u32 = bit_depth_to_u32(&bit_depth);
            info!("Writing {}-bit grayscale PNG with a transparent shade", depth_bits);
            encoder.set_color(ColorType::Grayscale);
            encoder.set_trns(vec![0, transparent_shade]);
            encoder.set_depth(bit_depth);
            let transparent_shade = channel_to_bit_depth(transparent_shade, bit_depth);
            let mut writer = encoder.write_header()?;
            let mut writer: BitWriter<_, BigEndian>
                = BitWriter::new(writer.stream_writer()?);
            for pixel in image.pixels() {
                writer.write(depth_bits, if pixel.alpha() != u8::MAX {
                    transparent_shade
                } else {
                    channel_to_bit_depth(pixel.red(), bit_depth)
                })?;
            }
            writer.flush()?;
        }
        PngMode::GrayscaleAlpha(bit_depth) => {
            let depth_bits: u32 = bit_depth_to_u32(&bit_depth);
            info!("Writing {}-bit grayscale PNG with alpha channel", depth_bits);
            encoder.set_color(ColorType::GrayscaleAlpha);
            encoder.set_depth(bit_depth);
            let mut writer = encoder.write_header()?;
            let mut writer: BitWriter<_, BigEndian>
                = BitWriter::new(writer.stream_writer()?);
            for pixel in image.pixels() {
                writer.write(depth_bits,
                             channel_to_bit_depth(pixel.demultiply().red(), bit_depth))?;
                writer.write(depth_bits,
                             channel_to_bit_depth(pixel.alpha(), bit_depth))?;
            }
            writer.flush()?;
        }
        PngMode::IndexedRgbOpaque(palette) => {
            let len = palette.len();
            info!("Writing a 24-bit RGB PNG");
            write_indexed_png(image, palette, encoder,
                              bit_depth_for_palette_size(len).unwrap(), false)?;
        }
        PngMode::IndexedRgba(palette) => {
            let len = palette.len();
            info!("Writing a 32-bit RGBA PNG");
            write_indexed_png(image, palette, encoder,
                              bit_depth_for_palette_size(len).unwrap(), true)?;
        }
    }

    match optimize_from_memory(reusable.deref(), &OXIPNG_OPTIONS) {
        Ok(optimized) => Ok(MaybeFromPool::NotFromPool(optimized)),
        Err(e) => {
            error!("Error from oxipng: {}", e);
            Ok(FromPool {reusable})
        }
    }
}

fn bit_depth_for_palette_size(size: usize) -> Option<BitDepth> {
    if size <= 2 {
        Some(BitDepth::One)
    } else if size <= 4 {
        Some(BitDepth::Two)
    } else if size <= 16 {
        Some(BitDepth::Four)
    } else if size <= 256 {
        Some(BitDepth::Eight)
    } else {
        None
    }
}

pub fn write_indexed_png<T: Write>(image: MaybeFromPool<Pixmap>, palette: Vec<ComparableColor>, mut encoder: Encoder<T>,
                         bit_depth: BitDepth, include_alpha: bool)
    -> Result<(), CloneableError> {
    encoder.set_color(ColorType::Indexed);
    encoder.set_depth(bit_depth);
    let mut sorted_palette: Vec<([u8; 4], ComparableColor)> = Vec::with_capacity(palette.len());
    let mut palette_data: Vec<u8> = Vec::with_capacity(3 * palette.len());
    for color in palette.iter() {
        sorted_palette.push((cast(PremultipliedColorU8::from(*color)), *color));
    }
    sorted_palette.sort_by_key(|(premult_bytes, _)| *premult_bytes);
    let mut trns: Vec<u8> = Vec::with_capacity(if include_alpha {
        palette.len()
    } else { 0 });
    for (_, color) in sorted_palette.iter() {
        palette_data.extend_from_slice(&[color.red(), color.green(), color.blue()]);
        if include_alpha {
            trns.push(color.alpha());
        }
    }
    encoder.set_palette(palette_data);
    if include_alpha {
        encoder.set_trns(trns);
        info!("Writing an indexed-color PNG with {} colors and alpha", palette.len());
    } else {
        info!("Writing an indexed-color PNG with {} colors", palette.len());
    }
    let mut writer = encoder.write_header()?;
    let mut bit_writer: BitWriter<_, BigEndian> = BitWriter::new(
        writer.stream_writer()?);
    let mut palette_premul: Vec<[u8; 4]> = Vec::with_capacity(palette.len());
    for (premul_bytes, _) in sorted_palette.iter() {
        palette_premul.push(*premul_bytes);
    }
    let mut error_corrections = HashMap::new();
    let mut worst_discrepancy: u16 = 0;
    let indexed_bits = bit_depth_to_u32(&bit_depth);
    let mut prev_pixel: PremultipliedColorU8 = cast(palette_premul[0]);
    let mut prev_index: u16 = 0;
    for pixel in image.pixels() {
        let index = if prev_pixel == *pixel {
            prev_index
        } else {
            let pixel_bytes: [u8; 4] = cast(*pixel);
            let index = match palette_premul.binary_search(&pixel_bytes) {
                Ok(index) => index as u16,
                Err(_) => match error_corrections.get(&pixel_bytes) {
                    Some(index) => *index,
                    None => {
                        let pixel_color = ComparableColor::from(pixel.to_owned());
                        let (index, (_, color))
                            = sorted_palette.iter().enumerate()
                            .min_by_key(|(_, (_, color))| color.abs_diff(&pixel_color))
                            .unwrap();
                        let index = index as u16;
                        error_corrections.insert(pixel_bytes, index);
                        worst_discrepancy = worst_discrepancy.max(color.abs_diff(&pixel_color));
                        index
                    }
                }
            };
            prev_pixel = *pixel;
            prev_index = index;
            index
        };
        bit_writer.write(indexed_bits, index)?;
    }
    if !error_corrections.is_empty() {
        warn!("Corrected {} color errors; worst error amount was {}", error_corrections.len(), worst_discrepancy);
    }
    bit_writer.flush()?;
    Ok(())
}

fn demultiply_image(image: &mut Pixmap) {
    for pixel in image.pixels_mut() {
        unsafe {
            // Treat this PremultipliedColorU8 slice as a ColorU8 slice
            *pixel = transmute(pixel.demultiply());
        }
    }
}
