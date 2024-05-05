use bitstream_io::{BigEndian, BitWrite, BitWriter};
use bytemuck::cast;
use core::mem::{size_of, transmute};
use include_dir::File;
use itertools::Itertools;
use log::{info, warn};
use once_cell::sync::Lazy;
#[cfg(not(debug_assertions))]
use oxipng::Deflaters;
use oxipng::{BitDepth, ColorType, IndexSet, Options, RawImage, RowFilter};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::ops::DerefMut;

use resvg::tiny_skia::{ColorU8, Pixmap, PremultipliedColorU8};
use tracing::{info_span, instrument};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use zip::{CompressionMethod, ZipArchive};

use crate::image_tasks::cloneable::CloneableError;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::channel_to_bit_depth;
use crate::image_tasks::MaybeFromPool;
use crate::TILE_SIZE;

pub type ZipBufferRaw = Cursor<Vec<u8>>;

#[cfg(not(debug_assertions))]
const PNG_BUFFER_SIZE: usize = 1024 * 1024;

static ZIP_BUFFER_SIZE: Lazy<usize> = Lazy::new(|| (*TILE_SIZE as usize) * 32 * 1024);
#[cfg(not(debug_assertions))]
static PNG_ZIP_OPTIONS: Lazy<SimpleFileOptions> = Lazy::new(|| {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .with_zopfli_buffer(Some(PNG_BUFFER_SIZE))
        .compression_level(Some(if *TILE_SIZE < 2048 {
            264
        } else if *TILE_SIZE < 4096 {
            24
        } else {
            8
        }))
});
#[cfg(debug_assertions)]
static PNG_ZIP_OPTIONS: Lazy<SimpleFileOptions> =
    Lazy::new(|| SimpleFileOptions::default().compression_method(CompressionMethod::Stored));

static METADATA_ZIP_OPTIONS: Lazy<SimpleFileOptions> = Lazy::new(|| {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(264))
});
pub static ZIP: Lazy<Mutex<ZipWriter<ZipBufferRaw>>> = Lazy::new(|| {
    Mutex::new(ZipWriter::new(Cursor::new(Vec::with_capacity(
        *ZIP_BUFFER_SIZE,
    ))))
});
#[cfg(not(debug_assertions))]
static OXIPNG_OPTIONS: Lazy<Options> = Lazy::new(|| {
    let mut options = Options::from_preset(if *TILE_SIZE < 1024 {
        6
    } else if *TILE_SIZE < 2048 {
        5
    } else {
        4
    });
    options.deflate = if *TILE_SIZE < 64 {
        Deflaters::Zopfli {
            iterations: u8::MAX.try_into().unwrap(),
        }
    } else if *TILE_SIZE < 128 {
        Deflaters::Zopfli {
            iterations: 100.try_into().unwrap(),
        }
    } else if *TILE_SIZE < 4096 {
        Deflaters::Libdeflater { compression: 12 }
    } else {
        Deflaters::Libdeflater { compression: 10 }
    };
    options.optimize_alpha = true;
    options
});
#[cfg(debug_assertions)]
static OXIPNG_OPTIONS: Lazy<Options> = Lazy::new(|| Options::from_preset(0));

fn png_filters_to_try(file_path: &str) -> Option<IndexSet<RowFilter>> {
    let tile_size = *TILE_SIZE;
    if tile_size > 2048 {
        if file_path.contains("compass") {
            Some(IndexSet::from([RowFilter::None]))
        } else {
            None
        }
    } else {
        None
    }
}

#[instrument(skip(image, color_type))]
pub fn png_output(
    image: MaybeFromPool<Pixmap>,
    color_type: ColorType,
    bit_depth: BitDepth,
    file_path: Box<str>,
) -> Result<(), CloneableError> {
    let width = image.width();
    let height = image.height();
    info!("Dimensions of {} are {}x{}", file_path, width, height);
    let raw_bytes = match color_type {
        ColorType::RGB { transparent_color } => {
            info!("Writing {} in RGB mode", file_path);
            let mut raw_bytes = Vec::with_capacity(3 * width as usize * height as usize);
            let transparent_color = transparent_color.map(|color| {
                [
                    (color.r >> 8) as u8,
                    (color.g >> 8) as u8,
                    (color.b >> 8) as u8,
                ]
            });
            for pixel in image.pixels() {
                if let Some(transparent_color) = transparent_color
                    && pixel.alpha() == 0
                {
                    raw_bytes.extend_from_slice(&transparent_color);
                } else {
                    raw_bytes.push(pixel.red());
                    raw_bytes.push(pixel.green());
                    raw_bytes.push(pixel.blue());
                }
            }
            raw_bytes
        }
        ColorType::RGBA => {
            info!("Writing {} in RGBA mode", file_path);
            take_demultiplied(image.unwrap_or_clone())
        }
        ColorType::Grayscale { transparent_shade } => {
            info!("Writing {} in {}-bit grayscale mode", file_path, bit_depth);
            let raw_bytes =
                Vec::with_capacity(width as usize * height as usize * bit_depth as u8 as usize / 8);
            let transparent_shade =
                transparent_shade.map(|shade| channel_to_bit_depth((shade >> 8) as u8, bit_depth));
            let mut writer: BitWriter<_, BigEndian> = BitWriter::new(Cursor::new(raw_bytes));
            for pixel in image.pixels() {
                writer.write(
                    bit_depth as u8 as u32,
                    if let Some(transparent_shade) = transparent_shade
                        && pixel.alpha() == 0
                    {
                        transparent_shade
                    } else {
                        channel_to_bit_depth(pixel.red(), bit_depth)
                    },
                )?;
            }
            writer.flush()?;
            writer.into_writer().into_inner()
        }
        ColorType::GrayscaleAlpha => {
            info!("Writing {} in grayscale+alpha mode", file_path);
            let mut raw_bytes =
                Vec::with_capacity(image.width() as usize * image.height() as usize * 2);
            for pixel in image.pixels() {
                raw_bytes.extend_from_slice(&[pixel.demultiply().red(), pixel.alpha()]);
            }
            raw_bytes
        }
        ColorType::Indexed { ref palette } => {
            info!(
                "Writing {} in indexed mode with {} colors",
                file_path,
                palette.len()
            );
            let bytes = Vec::with_capacity(
                image.width() as usize * image.height() as usize * bit_depth as u8 as usize / 8,
            );
            let mut bit_writer: BitWriter<_, BigEndian> = BitWriter::new(Cursor::new(bytes));
            let mut palette_premul: Vec<[u8; 4]> = Vec::with_capacity(palette.len());
            let mut palette_demult: Vec<ComparableColor> = Vec::with_capacity(palette.len());
            let mut palette_with_error_corrections: HashMap<[u8; 4], usize> = HashMap::new();
            for (index, color) in palette.iter().enumerate() {
                let premul_bytes =
                    cast(ColorU8::from_rgba(color.r, color.g, color.b, color.a).premultiply());
                palette_premul.push(premul_bytes);
                palette_with_error_corrections.insert(premul_bytes, index);
                palette_demult.push(ComparableColor::from(*color));
            }
            let mut worst_discrepancy: u16 = 0;
            let mut prev_pixel: PremultipliedColorU8 = cast(palette_premul[0]);
            let mut prev_index: u16 = 0;
            for pixel in image.pixels() {
                let index = if prev_pixel == *pixel {
                    prev_index
                } else {
                    let pixel_bytes: [u8; 4] = cast(*pixel);
                    let index = match palette_with_error_corrections.get(&pixel_bytes) {
                        Some(index) => *index,
                        None => {
                            let pixel_color = ComparableColor::from(*pixel);
                            let (index, (_, discrepancy)) = palette_demult
                                .iter()
                                .map(|color| (color, color.abs_diff(&pixel_color)))
                                .enumerate()
                                .min_by_key(|(_, (_, discrepancy))| *discrepancy)
                                .unwrap();
                            palette_with_error_corrections.insert(pixel_bytes, index);
                            worst_discrepancy = worst_discrepancy.max(discrepancy);
                            index
                        }
                    } as u16;
                    prev_pixel = *pixel;
                    prev_index = index;
                    index
                };
                bit_writer.write(bit_depth as u8 as u32, index)?;
            }
            bit_writer.flush()?;
            if let Some(corrected_color_count) = palette_with_error_corrections
                .len()
                .checked_sub(palette.len())
                && corrected_color_count > 0
            {
                let corrections = palette_with_error_corrections
                    .into_iter()
                    .flat_map(|(raw, corrected_index)| {
                        let found: PremultipliedColorU8 = cast(raw);
                        let found: ComparableColor = found.into();
                        let corrected = palette_demult[corrected_index];
                        if found != corrected {
                            Some(format!("{} -> {}", found, corrected))
                        } else {
                            None
                        }
                        .into_iter()
                    })
                    .join(", ");
                warn!(
                    "Corrected {} color errors in {} (worst error amount was {}): {}",
                    corrected_color_count, file_path, worst_discrepancy, corrections
                );
            }
            bit_writer.into_writer().into_inner()
        }
    };
    let mut mut_png_options: Options;
    let png_options = if let Some(png_filters) = png_filters_to_try(&file_path) {
        mut_png_options = OXIPNG_OPTIONS.clone();
        mut_png_options.filter = png_filters;
        &mut_png_options
    } else {
        &*OXIPNG_OPTIONS
    };
    let png_span = info_span!("PNG optimization");
    let png_span = png_span.enter();
    let png = RawImage::new(width, height, color_type, bit_depth, raw_bytes)?
        .create_optimized_png(png_options)?;
    drop(png_span);
    let deflate_span = info_span!("Deflating file");
    let deflate_span = deflate_span.enter();
    let zip = &*ZIP;
    match zip.try_lock() {
        Some(mut writer_guard) => {
            writer_guard.start_file(file_path, PNG_ZIP_OPTIONS.to_owned())?;
            writer_guard.write_all(&png)?;
        }
        None => {
            let mut single_file_out =
                ZipWriter::new(Cursor::new(Vec::with_capacity(*ZIP_BUFFER_SIZE)));
            single_file_out.start_file(file_path, PNG_ZIP_OPTIONS.to_owned())?;
            single_file_out.write_all(&png)?;
            let mut single_compressed_file = ZipArchive::new(single_file_out.finish()?)?;
            drop(deflate_span);
            let mut writer = match zip.try_lock() {
                None => {
                    let get_lock_span = info_span!("Waiting for lock on ZIP file");
                    let get_lock_span = get_lock_span.enter();
                    let writer = zip.lock();
                    drop(get_lock_span);
                    writer
                }
                Some(locked_writer) => locked_writer,
            };
            let write_file_span = info_span!("Adding file to ZIP file");
            let write_file_span = write_file_span.enter();
            writer.raw_copy_file(single_compressed_file.by_index_raw(0).unwrap())?;
            drop(write_file_span);
        }
    }
    Ok(())
}

pub fn copy_out_to_out(source_path: Box<str>, dest_path: Box<str>) -> Result<(), CloneableError> {
    ZIP.lock()
        .deref_mut()
        .deep_copy_file(&source_path, &dest_path)?;
    Ok(())
}

pub fn copy_in_to_out(source: &File, dest_path: Box<str>) -> Result<(), CloneableError> {
    let zip = &*ZIP;
    let mut writer = zip.lock();
    writer
        .deref_mut()
        .start_file(dest_path, METADATA_ZIP_OPTIONS.to_owned())?;
    writer.deref_mut().write_all(source.contents())?;
    Ok(())
}

fn take_demultiplied(image: Pixmap) -> Vec<u8> {
    let mut pixels = image.take();
    for pixel in pixels.array_chunks_mut() {
        unsafe {
            // Treat this as a PremultipliedColorU8 slice for input and a ColorU8 slice for output
            *pixel = transmute(
                transmute::<[u8; size_of::<PremultipliedColorU8>()], PremultipliedColorU8>(*pixel)
                    .demultiply(),
            );
        }
    }
    pixels.to_vec()
}
