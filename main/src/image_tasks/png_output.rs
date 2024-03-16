use std::collections::HashMap;
use include_dir::File;
use std::io::{Cursor, Write};
use std::mem::transmute;
use std::ops::DerefMut;
use std::sync::{Mutex};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use bytemuck::cast;
use itertools::Itertools;
use log::{info, warn};
use once_cell::sync::Lazy;
use oxipng::{BitDepth, ColorType, Deflaters, IndexSet, Options, RawImage, RowFilter};

use resvg::tiny_skia::{ColorU8, Pixmap, PremultipliedColorU8};
use zip_next::CompressionMethod::Deflated;
use zip_next::write::FileOptions;
use zip_next::ZipWriter;

use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::task_spec::channel_to_bit_depth;
use crate::TILE_SIZE;
use crate::image_tasks::cloneable::CloneableError;
use crate::image_tasks::color::ComparableColor;

pub type ZipBufferRaw = Cursor<Vec<u8>>;

const PNG_BUFFER_SIZE: usize = 1024 * 1024;

static ZIP_BUFFER_SIZE: Lazy<usize> = Lazy::new(|| (*TILE_SIZE as usize) * 32 * 1024);
static PNG_ZIP_OPTIONS: Lazy<FileOptions> = Lazy::new(|| FileOptions::default()
    .compression_method(Deflated)
    .with_zopfli_buffer(Some(PNG_BUFFER_SIZE))
    .compression_level(Some(if *TILE_SIZE < 2048 {
    264
} else if *TILE_SIZE < 4096 {
    24
} else {
    8
})));
static METADATA_ZIP_OPTIONS: Lazy<FileOptions> = Lazy::new(|| {
    FileOptions::default()
        .compression_method(Deflated)
        .compression_level(Some(264))
});
pub static ZIP: Lazy<Mutex<ZipWriter<ZipBufferRaw>>> = Lazy::new(|| Mutex::new(ZipWriter::new(Cursor::new(
    Vec::with_capacity(*ZIP_BUFFER_SIZE)))));
static OXIPNG_OPTIONS: Lazy<Options> = Lazy::new(|| {
    let mut options = Options::from_preset(if *TILE_SIZE < 1024 {
        6
    } else if *TILE_SIZE < 2048 {
        5
    } else {
        4
    });
    options.deflate = if *TILE_SIZE < 64 {
        Deflaters::Zopfli {iterations: u8::MAX.try_into().unwrap() }
    } else if *TILE_SIZE < 128 {
        Deflaters::Zopfli {iterations: 100.try_into().unwrap() }
    } else if *TILE_SIZE < 4096 {
        Deflaters::Libdeflater {compression: 12}
    } else {
        Deflaters::Libdeflater {compression: 10}
    };
    options.optimize_alpha = true;
    options
});

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

pub fn png_output(image: MaybeFromPool<Pixmap>, color_type: ColorType,
                  bit_depth: BitDepth, file_path: String) -> Result<(),CloneableError> {
    let width = image.width();
    let height = image.height();
    info!("Dimensions of {} are {}x{}", file_path, width, height);
    let raw_bytes = match color_type {
        ColorType::RGB {transparent_color} => {
            info!("Writing {} in RGB mode", file_path);
            let mut raw_bytes = Vec::with_capacity(3 * width as usize * height as usize);
            let transparent_color = transparent_color.map(|color| [
                (color.r >> 8) as u8,
                (color.g >> 8) as u8,
                (color.b >> 8) as u8
            ]);
            for pixel in image.pixels() {
                if let Some(transparent_color) = transparent_color && pixel.alpha() == 0 {
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
            let mut image = image.unwrap_or_clone();
            demultiply_image(&mut image);
            image.take()
        }
        ColorType::Grayscale {transparent_shade} => {
            info!("Writing {} in {}-bit grayscale mode", file_path, bit_depth);
            let raw_bytes = Vec::with_capacity(width as usize * height as usize
                * bit_depth as u8 as usize / 8);
            let transparent_shade = transparent_shade.map(|shade|
                channel_to_bit_depth((shade >> 8) as u8, bit_depth));
            let mut writer: BitWriter<_, BigEndian> = BitWriter::new(Cursor::new(raw_bytes));
            for pixel in image.pixels() {
                writer.write(bit_depth as u8 as u32, if let Some(transparent_shade) = transparent_shade && pixel.alpha() == 0 {
                    transparent_shade
                } else {
                    channel_to_bit_depth(pixel.red(), bit_depth)
                })?;
            }
            writer.flush()?;
            writer.into_writer().into_inner()
        }
        ColorType::GrayscaleAlpha => {
            info!("Writing {} in grayscale+alpha mode", file_path);
            let mut raw_bytes = Vec::with_capacity(
                image.width() as usize * image.height() as usize * 2);
            for pixel in image.pixels() {
                raw_bytes.extend_from_slice(&[pixel.demultiply().red(), pixel.alpha()]);
            }
            raw_bytes
        }
        ColorType::Indexed {ref palette} => {
            info!("Writing {} in indexed mode with {} colors", file_path, palette.len());
            let bytes = Vec::with_capacity(image.width() as usize * image.height() as usize
                * bit_depth as u8 as usize / 8);
            let mut sorted_palette: Vec<([u8; 4], u16, ComparableColor)> = Vec::with_capacity(palette.len());
            for (index, color) in palette.iter().enumerate() {
                let color = ColorU8::from_rgba(color.r, color.g, color.b, color.a);
                sorted_palette.push((cast(color.premultiply()), index as u16, ComparableColor::from(color)));
            }
            sorted_palette.sort_by_key(|(premult_bytes, _, _)| *premult_bytes);
            let mut bit_writer: BitWriter<_, BigEndian> = BitWriter::new(Cursor::new(bytes));
            let mut palette_premul: Vec<[u8; 4]> = Vec::with_capacity(palette.len());
            let mut palette_with_error_corrections: HashMap<[u8; 4], u16> = HashMap::new();
            for (premul_bytes, index, _) in sorted_palette.iter() {
                palette_premul.push(*premul_bytes);
                palette_with_error_corrections.push(*premul_bytes, index);
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
                        Ok(index) => {
                            index
                        }
                        None => {
                            let pixel_color = ComparableColor::from(*pixel);
                            let (_, orig_index, color)
                                = sorted_palette.iter()
                                .min_by_key(|(_, _, color)| color.abs_diff(&pixel_color))
                                .unwrap();
                            palette_with_error_corrections.insert(pixel_bytes, *orig_index);
                            worst_discrepancy = worst_discrepancy.max(color.abs_diff(&pixel_color));
                            *orig_index
                        }
                    };
                    prev_pixel = *pixel;
                    prev_index = index;
                    index
                };
                bit_writer.write(bit_depth as u8 as u32, index)?;
            }
            bit_writer.flush()?;
            if let Some(corrected_color_count)
                    = palette_with_error_corrections.len().checked_sub(sorted_palette.len()) {
                let corrections = palette_with_error_corrections.into_iter()
                    .flat_map(|(raw, corrected_index)| {
                        let found: PremultipliedColorU8 = cast(raw);
                        let found: ComparableColor = found.into();
                        let corrected = ComparableColor::from(palette[corrected_index as usize]);
                        if found != corrected {
                            Some(format!("{} -> {}", found, corrected))
                        } else {
                            None
                        }.into_iter()
                    })
                    .join(", ");
                warn!("Corrected {} color errors in {} (worst error amount was {}): {}",
                    corrected_color_count, file_path, worst_discrepancy, corrections);
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
    info!("Starting PNG optimization for {}", file_path);
    let png = RawImage::new(width, height, color_type, bit_depth, raw_bytes)?
        .create_optimized_png(&png_options)?;
    info!("Finished PNG optimization for {}", file_path);
    let zip = &*ZIP;
    let mut writer = zip.lock()?;
    writer.deref_mut().start_file(file_path, PNG_ZIP_OPTIONS.to_owned())?;
    writer.deref_mut().write_all(&png)?;
    Ok(())
}

pub fn copy_out_to_out(source_path: String, dest_path: String) -> Result<(),CloneableError> {
    ZIP.lock()?.deref_mut().deep_copy_file(&source_path, &dest_path)?;
    Ok(())
}

pub fn copy_in_to_out(source: &File, dest_path: String) -> Result<(),CloneableError> {
    let zip = &*ZIP;
    let mut writer = zip.lock()?;
    writer.deref_mut().start_file(dest_path, METADATA_ZIP_OPTIONS.to_owned())?;
    writer.deref_mut().write_all(source.contents())?;
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
