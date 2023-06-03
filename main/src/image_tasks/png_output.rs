use std::collections::HashMap;
use include_dir::{File};
use std::io::{Cursor, Write};
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::path::{Path};
use std::sync::{Arc, Mutex};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
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
use crate::image_tasks::task_spec::{bit_depth_to_u32, channel_to_bit_depth, CloneableError, PngColorMode, PngMode, PngTransparencyMode};
use crate::{TILE_SIZE};
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::MaybeFromPool::FromPool;
use crate::image_tasks::task_spec::PngColorMode::Indexed;
use crate::image_tasks::task_spec::PngTransparencyMode::{AlphaChannel, BinaryTransparency, Opaque};

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
    pub static ref PNG_BUFFER_POOL: Arc<LinearObjectPool<Vec<u8>>> = Arc::new(LinearObjectPool::new(
        || {
            info!("Allocating a PNG buffer for pool");
            Vec::with_capacity(PNG_BUFFER_SIZE)
        },
        Vec::clear
    ));
    static ref OXIPNG_OPTIONS: Options = {
        let mut options = Options::from_preset(if *TILE_SIZE < 1024 {
            6
        } else {
            5
        });
        options.deflate = if *TILE_SIZE < 64 {
            Deflaters::Zopfli {iterations: u8::MAX.try_into().unwrap() }
        } else if *TILE_SIZE < 128 {
            Deflaters::Zopfli {iterations: 50.try_into().unwrap() }
        } else if *TILE_SIZE < 256 {
            Deflaters::Zopfli {iterations: 15.try_into().unwrap() }
        } else if *TILE_SIZE < 512 {
            Deflaters::Zopfli {iterations: 5.try_into().unwrap() }
        } else if *TILE_SIZE < 4096 {
            Deflaters::Libdeflater {compression: 12}
        } else {
            Deflaters::Libdeflater {compression: 10}
        };
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
    let mut reusable = PNG_BUFFER_POOL.pull();
    let encoder = Encoder::new(reusable.deref_mut(), image.width(), image.height());
    match png_mode.color_mode {
        Indexed(palette) => {
            let real_palette_size = palette.len() + if png_mode.transparency_mode == BinaryTransparency {1} else {0};
            match bit_depth_for_palette_size(real_palette_size) {
                None => {
                    write_true_color_png(image, encoder, png_mode.transparency_mode)?;
                }
                Some(indexed_bit_depth) => {
                    let indexed_bits = bit_depth_to_u32(&indexed_bit_depth);
                    if palette.iter().all(ComparableColor::is_gray) {
                        let grayscale_bit_depth = palette.iter().max_by_key(
                            |color| bit_depth_to_u32(&color.bit_depth()))
                            .unwrap().bit_depth();
                        let transparency_mode: GrayscaleTransparencyMode = match png_mode.transparency_mode {
                            Opaque => GrayscaleTransparencyMode::Opaque,
                            BinaryTransparency => {
                                get_grayscale_transparency_mode(&image, &grayscale_bit_depth)
                            },
                            AlphaChannel => GrayscaleTransparencyMode::AlphaChannel
                        };
                        let mut grayscale_bits = bit_depth_to_u32(&grayscale_bit_depth);
                        if transparency_mode == GrayscaleTransparencyMode::AlphaChannel {
                            grayscale_bits *= 2;
                        }
                        if grayscale_bits <= indexed_bits {
                            write_grayscale_png(image, encoder, grayscale_bit_depth, transparency_mode)?;
                        } else {
                            write_indexed_png(image, palette, encoder, indexed_bit_depth, png_mode.transparency_mode)?;
                        }
                    } else {
                        write_indexed_png(image, palette, encoder, indexed_bit_depth, png_mode.transparency_mode)?;
                    }
                }
            }
        },
        PngColorMode::Grayscale => {
            let transparency_mode = match png_mode.transparency_mode {
                Opaque => GrayscaleTransparencyMode::Opaque,
                BinaryTransparency => get_grayscale_transparency_mode(&image, &BitDepth::Eight),
                AlphaChannel => GrayscaleTransparencyMode::AlphaChannel
            };
            write_grayscale_png(image, encoder, BitDepth::Eight, transparency_mode)?;
        },
        PngColorMode::Rgb => {
            write_true_color_png(image, encoder, png_mode.transparency_mode)?;
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

fn get_grayscale_transparency_mode(image: &MaybeFromPool<Pixmap>, grayscale_bit_depth: &BitDepth) -> GrayscaleTransparencyMode {
    let grayscale_bits = bit_depth_to_u32(grayscale_bit_depth);
    let grayscale_shades = 1 << grayscale_bits;
    let mut shades_in_use: Vec<bool> = vec![false;grayscale_shades];
    for pixel in image.pixels() {
        if pixel.alpha() == u8::MAX {
            // No need to demultiply fully opaque
            shades_in_use[channel_to_bit_depth(pixel.red(), *grayscale_bit_depth) as usize] = true;
        }
    }
    match shades_in_use.into_iter().enumerate().find(|(_, in_use)| !in_use) {
        Some((shade, _)) => GrayscaleTransparencyMode::TransparentShade(shade as u8),
        None => GrayscaleTransparencyMode::AlphaChannel
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum GrayscaleTransparencyMode {
    Opaque,
    TransparentShade(u8),
    AlphaChannel
}

pub fn write_grayscale_png<T: Write>(image: MaybeFromPool<Pixmap>, mut encoder: Encoder<T>, depth: BitDepth, transparency_mode: GrayscaleTransparencyMode)
    -> Result<(), CloneableError> {
    let depth_bits: u32 = bit_depth_to_u32(&depth);
    encoder.set_depth(depth);
    match transparency_mode {
        GrayscaleTransparencyMode::Opaque => {
            info!("Writing {}-bit grayscale PNG", depth_bits);
            encoder.set_color(ColorType::Grayscale);
            let mut writer = encoder.write_header()?;
            let mut writer: BitWriter<_, BigEndian> = BitWriter::new(writer.stream_writer()?);
            for pixel in image.pixels() {
                writer.write(depth_bits, channel_to_bit_depth(pixel.red(), depth))?;
            }
            writer.flush()?;
        },
        GrayscaleTransparencyMode::TransparentShade(transparent_shade) => {
            info!("Writing {}-bit grayscale PNG", depth_bits);
            encoder.set_color(ColorType::Grayscale);
            encoder.set_trns(vec![0, transparent_shade]);
            let transparent_shade = transparent_shade as u16;
            let mut writer = encoder.write_header()?;
            let mut writer: BitWriter<_, BigEndian>
                = BitWriter::new(writer.stream_writer()?);
            for pixel in image.pixels() {
                writer.write(depth_bits, if pixel.alpha() != u8::MAX {
                    transparent_shade
                } else {
                    channel_to_bit_depth(pixel.red(), depth)
                })?;
            }
            writer.flush()?;
        },
        GrayscaleTransparencyMode::AlphaChannel => {
            info!("Writing {}-bit grayscale PNG with alpha channel", depth_bits);
            encoder.set_color(ColorType::GrayscaleAlpha);
            let mut writer = encoder.write_header()?;
            let mut writer: BitWriter<_, BigEndian>
                = BitWriter::new(writer.stream_writer()?);
            for pixel in image.pixels() {
                writer.write(depth_bits,
                             channel_to_bit_depth(pixel.demultiply().red(), depth))?;
                writer.write(depth_bits,
                             channel_to_bit_depth(pixel.alpha(), depth))?;
            }
            writer.flush()?;
        }
    }
    Ok(())
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

const RESERVED_TRANSPARENT_COLOR: [u8; 3] = [0xc0, 0xff, 0x3e];
const RESERVED_TRANSPARENT_COLOR_TRNS: [u8; 6] = [
    0, RESERVED_TRANSPARENT_COLOR[0],
    0, RESERVED_TRANSPARENT_COLOR[1],
    0, RESERVED_TRANSPARENT_COLOR[2],
];

pub fn write_indexed_png<T: Write>(image: MaybeFromPool<Pixmap>, palette: Vec<ComparableColor>, mut encoder: Encoder<T>,
                         bit_depth: BitDepth, transparency_mode: PngTransparencyMode)
    -> Result<(), CloneableError> {
    encoder.set_color(ColorType::Indexed);
    encoder.set_depth(bit_depth);
    let real_palette_size = palette.len()
        + if transparency_mode == BinaryTransparency {1} else {0};
    let mut palette_data: Vec<u8> = Vec::with_capacity(real_palette_size);
    let mut trns: Vec<u8> = match transparency_mode {
        Opaque => vec![],
        BinaryTransparency => RESERVED_TRANSPARENT_COLOR_TRNS.to_vec(),
        AlphaChannel => Vec::with_capacity(real_palette_size)
    };
    for color in palette.iter() {
        palette_data.extend_from_slice(&[color.red(), color.green(), color.blue()]);
        if transparency_mode == AlphaChannel {
            trns.push(color.alpha());
        } else {
        }
    }
    let mut transparent_index: u16 = 0;
    if transparency_mode == BinaryTransparency {
        palette_data.extend_from_slice(&RESERVED_TRANSPARENT_COLOR);
        transparent_index = (real_palette_size - 1) as u16;
    }
    encoder.set_palette(palette_data);
    if transparency_mode != Opaque {
        encoder.set_trns(trns);
    }
    if transparency_mode == AlphaChannel {
        info!("Writing an indexed-color PNG with {} colors and alpha", real_palette_size);
    } else {

        info!("Writing an indexed-color PNG with {} colors", real_palette_size);
    }
    let mut writer = encoder.write_header()?;
    let mut bit_writer: BitWriter<_, BigEndian> = BitWriter::new(
        writer.stream_writer()?);
    let mut color_to_index: HashMap<ComparableColor, u16> = HashMap::with_capacity(palette.len());
    for (index, color) in palette.iter().enumerate() {
        color_to_index.insert(*color, index as u16);
    }
    let indexed_bits = bit_depth_to_u32(&bit_depth);
    let mut prev_pixel: Option<PremultipliedColorU8> = None;
    let mut prev_index: Option<u16> = None;
    for pixel in image.pixels() {
        let index = if prev_pixel == Some(*pixel)
            && let Some(prev_index) = prev_index {
            prev_index
        } else if transparency_mode == BinaryTransparency && pixel.alpha() != u8::MAX {
            transparent_index
        } else {
            let pixel_color: ComparableColor = (*pixel).into();
            let index = color_to_index.get(&pixel_color).copied().unwrap_or_else(|| {
                let (index, color)
                    = palette.iter().enumerate()
                    .min_by_key(|(_, color)| color.abs_diff(&pixel_color))
                    .unwrap();
                let index = index as u16;
                color_to_index.insert(*color, index);
                index
            });
            prev_pixel = Some(*pixel);
            prev_index = Some(index);
            index
        };
        bit_writer.write(indexed_bits, index)?;
    }
    let fuzzy_matched_colors = color_to_index.len() - palette.len();
    if fuzzy_matched_colors > 0 {
        warn!("Found {} colors that didn't exactly match the palette", fuzzy_matched_colors);
    }
    bit_writer.flush()?;
    Ok(())
}

fn write_true_color_png<T: Write>(mut image: MaybeFromPool<Pixmap>, mut encoder: Encoder<T>, transparency_mode: PngTransparencyMode) -> Result<(), CloneableError> {
    encoder.set_depth(BitDepth::Eight);
    for pixel in image.pixels_mut() {
        unsafe {
            // Treat this PremultipliedColorU8 slice as a ColorU8 slice
            *pixel = transmute(pixel.demultiply());
        }
    }
    match transparency_mode {
        Opaque => {
            info!("Writing an RGB PNG");
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
        BinaryTransparency => {
            encoder.set_color(ColorType::Rgb);
            encoder.set_trns(RESERVED_TRANSPARENT_COLOR_TRNS.to_vec());
            info!("Writing an RGB PNG with a transparent color");
            let mut writer = encoder.write_header()?;
            let mut data: Vec<u8> = Vec::with_capacity(3 * image.pixels().len());
            for pixel in image.pixels() {
                data.extend_from_slice(&if pixel.alpha() != u8::MAX {
                    RESERVED_TRANSPARENT_COLOR
                } else {
                    [pixel.red(), pixel.green(), pixel.blue()]
                });
            }
            writer.write_image_data(&data)?;
            writer.finish()?;
        }
        AlphaChannel => {
            info!("Writing an RGBA PNG");
            encoder.set_color(ColorType::Rgba);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(image.data())?;
            writer.finish()?;
        }
    }
    Ok(())
}