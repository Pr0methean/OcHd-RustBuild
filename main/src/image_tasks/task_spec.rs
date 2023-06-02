use std::collections::{HashMap, HashSet};

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::mem::transmute;

use std::ops::{Deref, DerefMut, Mul};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use bytemuck::{cast};

use cached::lazy_static::lazy_static;
use crate::anyhoo;
use include_dir::{Dir, include_dir};
use itertools::Itertools;

use log::{info, warn};
use ordered_float::OrderedFloat;
use png::{BitDepth, ColorType, Encoder};
use replace_with::replace_with_and_return;

use resvg::tiny_skia::{Color, ColorU8, Mask, Pixmap, PremultipliedColorU8};

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg, SEMITRANSPARENCY_FREE_SVGS};
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::png_output::{copy_out_to_out, PNG_BUFFER_POOL, png_output};
use crate::image_tasks::repaint::{paint, pixmap_to_mask};
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_alpha_on_background, stack_layer_on_background, stack_layer_on_layer};
use crate::image_tasks::task_spec::PngColorMode::{Grayscale, Indexed, Rgb};
use crate::image_tasks::task_spec::PngTransparencyMode::{AlphaChannel, BinaryTransparency, Opaque};
use crate::TILE_SIZE;

pub trait TaskSpecTraits <T>: Clone + Debug + Display + Ord + Eq + Hash {
    fn add_to<'a, 'b>(&'b self, ctx: &mut TaskGraphBuildingContext)
                         -> CloneableLazyTask<T>
        where 'b: 'a;
}

impl TaskSpecTraits<MaybeFromPool<Pixmap>> for ToPixmapTaskSpec {
    fn add_to<'a, 'b>(&'b self, ctx: &mut TaskGraphBuildingContext)
                         -> CloneableLazyTask<MaybeFromPool<Pixmap>>
                         where 'b: 'a {
        let name: String = self.to_string();
        if let Some(existing_future) = ctx.pixmap_task_to_future_map.get(self) {
            info!("Matched an existing node: {}", name);
            return existing_future.to_owned();
        }
        let function: LazyTaskFunction<MaybeFromPool<Pixmap>> = match self {
            ToPixmapTaskSpec::None { .. } => panic!("Tried to add None task to graph"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let background_opaque = background.get_transparency_mode() == Opaque;
                let background_future = background.add_to(ctx);
                let mut frame_futures: Vec<CloneableLazyTask<MaybeFromPool<Pixmap>>>
                    = Vec::with_capacity(frames.len());
                for frame in frames {
                    let frame_future = frame.add_to(ctx);
                    frame_futures.push(frame_future);
                }
                Box::new(move || {
                    let background: Arc<Box<MaybeFromPool<Pixmap>>> = background_future.into_result()?;
                    animate(&background, frame_futures, !background_opaque)
                })
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source = source.to_owned();
                Box::new(move || {
                    Ok(Box::new(from_svg(&source, *TILE_SIZE)?))
                })
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                let background: Color = (*background).into();
                let fg_future = foreground.add_to(ctx);
                Box::new(move || {
                    let fg_image: Arc<Box<MaybeFromPool<Pixmap>>> = fg_future.into_result()?;
                    let mut fg_image = Arc::unwrap_or_clone(fg_image);
                    stack_layer_on_background(background, &mut fg_image)?;
                    Ok(fg_image)
                })
            },
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                let bg_future = background.add_to(ctx);
                let fg_future = foreground.add_to(ctx);
                Box::new(move || {
                    let bg_image: Arc<Box<MaybeFromPool<Pixmap>>> = bg_future.into_result()?;
                    let mut out_image = Arc::unwrap_or_clone(bg_image);
                    let fg_image: Arc<Box<MaybeFromPool<Pixmap>>> = fg_future.into_result()?;
                    stack_layer_on_layer(&mut out_image, fg_image.deref());
                    Ok(out_image)
                })
            },
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                let base_future = base.add_to(ctx);
                let color = color.to_owned();
                Box::new(move || {
                    let base_image: Arc<Box<MaybeFromPool<Mask>>> = base_future.into_result()?;
                    paint(Arc::unwrap_or_clone(base_image).as_ref(), color)
                })
            },
        };
        info!("Adding node: {}", name);
        let task = CloneableLazyTask::new(name, function);
        ctx.pixmap_task_to_future_map.insert(self.to_owned(), task.to_owned());
        task
    }
}

impl TaskSpecTraits<MaybeFromPool<Mask>> for ToAlphaChannelTaskSpec {
    fn add_to<'a, 'b>(&'b self, ctx: &mut TaskGraphBuildingContext)
                         -> CloneableLazyTask<MaybeFromPool<Mask>>
                         where 'b: 'a {
        let name: String = self.to_string();
        if let Some(existing_future)
                = ctx.alpha_task_to_future_map.get(self) {
            info!("Matched an existing node: {}", name);
            return existing_future.to_owned();
        }
        let function: LazyTaskFunction<MaybeFromPool<Mask>> = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                let alpha: f32 = (*alpha).into();
                let base_future = base.add_to(ctx);
                Box::new(move || {
                    let base_result: Arc<Box<MaybeFromPool<Mask>>> = base_future.into_result()?;
                    let mut channel = Arc::unwrap_or_clone(base_result);
                    make_semitransparent(&mut channel, alpha);
                    Ok(channel)
                })
            },
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                let base_future = base.add_to(ctx);
                Box::new(move || {
                    let base_image: Arc<Box<MaybeFromPool<Pixmap>>> = base_future.into_result()?;
                    Ok(Box::new(pixmap_to_mask(&base_image)))
                })
            },
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { background, foreground } => {
                let bg_future = background.add_to(ctx);
                let fg_future = foreground.add_to(ctx);
                Box::new(move || {
                    let bg_mask: Arc<Box<MaybeFromPool<Mask>>> = bg_future.into_result()?;
                    let mut out_mask = Arc::unwrap_or_clone(bg_mask);
                    let fg_mask: Arc<Box<MaybeFromPool<Mask>>> = fg_future.into_result()?;
                    stack_alpha_on_alpha(&mut out_mask, fg_mask.deref());
                    Ok(out_mask)
                })
            },
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background, foreground } => {
                let background = background.0;
                let fg_future = foreground.add_to(ctx);
                Box::new(move || {
                    let fg_arc: Arc<Box<MaybeFromPool<Mask>>> = fg_future.into_result()?;
                    let mut fg_image = Arc::unwrap_or_clone(fg_arc);
                    stack_alpha_on_background(background, &mut fg_image);
                    Ok(fg_image)
                })
            }
        };
        info!("Adding node: {}", name);
        let task = CloneableLazyTask::new(name, function);
        ctx.alpha_task_to_future_map.insert(self.to_owned(), task.to_owned());
        task
    }
}

impl TaskSpecTraits<()> for FileOutputTaskSpec {
    fn add_to<'a, 'b>(&'b self, ctx: &mut TaskGraphBuildingContext)
                         -> CloneableLazyTask<()>
                         where 'b: 'a {
        let name: String = self.to_string();
        if let Some(existing_future)
                = ctx.output_task_to_future_map.get(self) {
            info!("Matched an existing node: {}", name);
            return existing_future.to_owned();
        }
        let function: LazyTaskFunction<()> = match self {
            FileOutputTaskSpec::PngOutput {base, destination } => {
                let destination = destination.to_owned();
                let base_future = base.add_to(ctx);
                let color_mode = base.get_color_mode();
                let transparency_mode = base.get_transparency_mode();
                Box::new(move || {
                    let base_result = base_future.into_result()?;
                    Ok(Box::new(png_output(*Arc::unwrap_or_clone(base_result),
                                           PngMode {color_mode, transparency_mode},
                                           &destination)?))
                })
            }
            FileOutputTaskSpec::Copy {original, link} => {
                let link = link.to_owned();
                let original_path = original.get_path();
                let base_future = original.add_to(ctx);
                Box::new(move || {
                    base_future.into_result()?;
                    Ok(Box::new(copy_out_to_out(&original_path, &link)?))
                })
            }
        };
        info!("Adding node: {}", name);
        let wrapped_future = CloneableLazyTask::new(name, function);
        ctx.output_task_to_future_map.insert(self.to_owned(), wrapped_future.to_owned());
        wrapped_future
    }
}

pub type CloneableResult<T> = Result<Arc<Box<T>>, CloneableError>;

/// [TaskSpec] for a task that produces a [Pixmap].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToPixmapTaskSpec {
    Animate {background: Box<ToPixmapTaskSpec>, frames: Vec<ToPixmapTaskSpec>},
    FromSvg {source: PathBuf},
    PaintAlphaChannel {base: Box<ToAlphaChannelTaskSpec>, color: ComparableColor},
    StackLayerOnColor {background: ComparableColor, foreground: Box<ToPixmapTaskSpec>},
    StackLayerOnLayer {background: Box<ToPixmapTaskSpec>, foreground: Box<ToPixmapTaskSpec>},
    None {},
}

/// [TaskSpec] for a task that produces an [AlphaChannel].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToAlphaChannelTaskSpec {
    MakeSemitransparent {base: Box<ToAlphaChannelTaskSpec>, alpha: OrderedFloat<f32>},
    FromPixmap {base: Box<ToPixmapTaskSpec>},
    StackAlphaOnAlpha {background: Box<ToAlphaChannelTaskSpec>, foreground: Box<ToAlphaChannelTaskSpec>},
    StackAlphaOnBackground {background: OrderedFloat<f32>, foreground: Box<ToAlphaChannelTaskSpec>}
}

/// [TaskSpec] for a task that doesn't produce a heap object as output.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FileOutputTaskSpec {
    PngOutput {base: ToPixmapTaskSpec, destination: PathBuf},
    Copy {original: Box<FileOutputTaskSpec>, link: PathBuf}
}

impl FileOutputTaskSpec {
    pub(crate) fn get_path(&self) -> PathBuf {
        match self {
            FileOutputTaskSpec::PngOutput { destination, .. } => destination.to_owned(),
            FileOutputTaskSpec::Copy { link, .. } => link.to_owned()
        }
    }
}

/// Specification of a task that produces one of several output types. Created so that
/// copies of the same task created for different [Material] instances can be deduplicated, since
/// function closures and futures don't implement [Eq] or [Hash].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum TaskSpec {
    ToPixmap(ToPixmapTaskSpec),
    ToAlphaChannel(ToAlphaChannelTaskSpec),
    FileOutput(FileOutputTaskSpec)
}

impl Display for TaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskSpec::ToPixmap(inner) => (inner as &dyn Display).fmt(f),
            TaskSpec::ToAlphaChannel(inner) => (inner as &dyn Display).fmt(f),
            TaskSpec::FileOutput(inner) => (inner as &dyn Display).fmt(f),
        }
    }
}

impl From<&ToPixmapTaskSpec> for TaskSpec {
    fn from(value: &ToPixmapTaskSpec) -> Self {
        TaskSpec::ToPixmap(value.to_owned())
    }
}

impl From<&ToAlphaChannelTaskSpec> for TaskSpec {
    fn from(value: &ToAlphaChannelTaskSpec) -> Self {
        TaskSpec::ToAlphaChannel(value.to_owned())
    }
}

impl From<&FileOutputTaskSpec> for TaskSpec {
    fn from(value: &FileOutputTaskSpec) -> Self {
        TaskSpec::FileOutput(value.to_owned())
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CloneableError {
    message: String
}

impl <T> From<T> for CloneableError where T: ToString {
    fn from(value: T) -> Self {
        CloneableError {message: value.to_string()}
    }
}

#[macro_export]
macro_rules! anyhoo {
    ($($args:expr),+ $(,)?) => {
        $crate::image_tasks::task_spec::CloneableError::from(anyhow::anyhow!($($args),+))
    }
}

impl Display for ToPixmapTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToPixmapTaskSpec::Animate { background, frames } => {
                write!(f, "animate({};{})", background, frames.iter().join(";"))
            }
            ToPixmapTaskSpec::FromSvg { source } => {
                write!(f, "{}", source.to_string_lossy())
            }
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                if let ToAlphaChannelTaskSpec::FromPixmap {base: base_of_base} = &**base {
                    write!(f, "{}@{}", *base_of_base, color)
                } else {
                    write!(f, "{}@{}", *base, color)
                }
            }
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                write!(f, "{}+{}", background, foreground)
            }
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                write!(f, "({}+{})", background, foreground)
            }
            ToPixmapTaskSpec::None {} => {
                write!(f, "None")
            },
        }
    }
}

impl Display for FileOutputTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            FileOutputTaskSpec::PngOutput { destination, .. } => {
                destination.to_string_lossy().to_string()
            },
            FileOutputTaskSpec::Copy { original, link } => {
                format!("symlink({} -> {})", link.to_string_lossy(), original.to_string())
            }
        })
    }
}

impl Display for ToAlphaChannelTaskSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                write!(f, "{}@{}", base, alpha)
            }
            ToAlphaChannelTaskSpec::FromPixmap {base} => {
                write!(f, "alpha({})", base)
            }
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha {background, foreground} => {
                write!(f, "({}+{})", background, foreground)
            }
            ToAlphaChannelTaskSpec::StackAlphaOnBackground {background, foreground} => {
                write!(f, "({}+{})", background, foreground)
            }
        }
    }
}

pub type LazyTaskFunction<T> = Box<dyn FnOnce() -> Result<Box<T>, CloneableError> + Send>;

pub enum CloneableLazyTaskState<T> where T: ?Sized {
    Upcoming {
        function: LazyTaskFunction<T>,
    },
    Finished {
        result: CloneableResult<T>
    }
}

#[derive(Clone,Debug)]
pub struct CloneableLazyTask<T> where T: ?Sized {
    pub name: String,
    state: Arc<Mutex<CloneableLazyTaskState<T>>>
}

impl <T> Debug for CloneableLazyTaskState<T> where T: ?Sized {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneableLazyTaskState::Upcoming { .. } => {
                f.write_str("Upcoming")
            },
            CloneableLazyTaskState::Finished { result } => {
                match result {
                    Ok(..) => f.write_str("Ok"),
                    Err(error) => f.write_fmt(
                        format_args!("Error({})", error.message))
                }
            }
        }
    }
}

impl <T> CloneableLazyTask<T> where T: ?Sized {
    pub fn new(name: String, base: LazyTaskFunction<T>) -> CloneableLazyTask<T> {
        CloneableLazyTask {
            name,
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Upcoming {
                function: base
            }))
        }
    }

    /// Consumes this particular copy of the task and returns the result. Trades off readability and
    /// maintainability to maximize the chance of avoiding unnecessary copies.
    pub fn into_result(self) -> CloneableResult<T> {
        match Arc::try_unwrap(self.state) {
            Ok(exclusive_state) => {
                // We're the last referent to this Lazy, so we don't need to clone anything.
                match exclusive_state.into_inner() {
                    Ok(state) => match state {
                        CloneableLazyTaskState::Upcoming { function } => {
                            info!("Starting task {}", self.name);
                            let result: CloneableResult<T> = function().map(Arc::new);
                            info!("Finished task {}", self.name);
                            info!("Unwrapping the only reference to {}", self.name);
                            result
                        },
                        CloneableLazyTaskState::Finished { result } => {
                            info!("Unwrapping the last reference to {}", self.name);
                            result
                        },
                    }
                    Err(e) => Err(e.into())
                }
            }
            Err(shared_state) => {
                match shared_state.lock() {
                    Ok(mut locked_state) => {
                        replace_with_and_return(
                            locked_state.deref_mut(),
                            || CloneableLazyTaskState::Finished {
                                result: Err(anyhoo!("replace_with_and_return_failed"))
                            },
                            |exec_state| {
                                match exec_state {
                                    CloneableLazyTaskState::Upcoming { function} => {
                                        info! ("Starting task {}", self.name);
                                        let result: CloneableResult<T> = function().map(Arc::new);
                                        info! ("Finished task {}", self.name);
                                        info!("Unwrapping one of {} references to {} after computing it",
                                            Arc::strong_count(&shared_state), self.name);
                                        (result.to_owned(), CloneableLazyTaskState::Finished { result })
                                    },
                                    CloneableLazyTaskState::Finished { result } => {
                                        info!("Unwrapping one of {} references to {}",
                                            Arc::strong_count(&shared_state), self.name);
                                        (result.to_owned(), CloneableLazyTaskState::Finished { result })
                                    },
                                }
                            }
                        )
                    }
                    Err(e) => Err(e.into())
                }
            }
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum PngColorMode {
    Indexed(Vec<ComparableColor>),
    Grayscale,
    Rgb
}

impl PngColorMode {
    pub fn is_grayscale_compatible(&self) -> bool {
        match self {
            Indexed(palette ) => palette.iter().all(ComparableColor::is_gray),
            Grayscale => true,
            Rgb => false
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum PngTransparencyMode {
    Opaque,
    BinaryTransparency,
    AlphaChannel
}

pub struct PngMode {
    pub color_mode: PngColorMode,
    pub transparency_mode: PngTransparencyMode
}

impl PngMode {
    pub fn write(self, mut image: MaybeFromPool<Pixmap>) -> Result<MaybeFromPool<Vec<u8>>, CloneableError> {
        let mut reusable = PNG_BUFFER_POOL.pull();
        let mut encoder = Encoder::new(reusable.deref_mut(), image.width(), image.height());
        match self.color_mode {
            Indexed(mut palette) => {
                if ((palette.len() > 16 && self.transparency_mode != AlphaChannel)
                    || palette.len() > u8::MAX as usize + 1)
                        && palette.iter().all(ComparableColor::is_gray) {
                    return PngMode {color_mode: Grayscale, transparency_mode: self.transparency_mode}
                        .write(image);
                }
                if palette.len() > u8::MAX as usize + 1 {
                    return PngMode {color_mode: Rgb, transparency_mode: self.transparency_mode}
                        .write(image);
                }
                let include_alpha = self.transparency_mode != Opaque;
                let mut palette_data: Vec<u8> = Vec::with_capacity(palette.len() * 3);
                let mut trns: Vec<u8> = Vec::with_capacity(
                    if self.transparency_mode == Opaque { 0 } else { palette.len() });
                for color in palette.iter() {
                    palette_data.extend_from_slice(&[color.red(), color.green(), color.blue()]);
                    if include_alpha {
                        trns.push(color.alpha());
                    }
                }
                encoder.set_color(ColorType::Indexed);
                encoder.set_palette(palette_data);
                if include_alpha {
                    encoder.set_trns(trns);
                }
                let depth = if palette.len() <= 2 {
                    BitDepth::One
                } else if palette.len() <= 4 {
                    BitDepth::Two
                } else if palette.len() <= 16 {
                    BitDepth::Four
                } else if palette.len() <= 256 {
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
                for pixel in image.pixels() {
                    let mut written = false;
                    let pixel_color: ComparableColor = (*pixel).into();
                    for (index, color) in palette.iter().enumerate() {
                        if *color == pixel_color {
                            bit_writer.write(depth, index as u16)?;
                            written = true;
                            break;
                        }
                    }
                    if !written {
                        for (index, color) in palette.iter_mut().enumerate() {
                            if color.red().abs_diff(pixel_color.red()) <= 3
                                && color.green().abs_diff(pixel_color.green()) <= 3
                                && color.blue().abs_diff(pixel_color.blue()) <= 3
                                && color.alpha().abs_diff(pixel_color.alpha()) <= 1 {
                                if *color != pixel_color {
                                    warn!("Rounding discrepancy: expected {}, found {}",
                                    color, pixel_color);
                                    *color = pixel_color;
                                }
                                bit_writer.write(depth, index as u16)?;
                                written = true;
                                break;
                            }
                        }
                    }
                    if !written {
                        return Err(anyhoo!("Unexpected color {}; expected palette was {}",
                            pixel_color, palette.iter().join(",")));
                    }
                }
                bit_writer.flush()?;
            }
            Grayscale => {
                if self.transparency_mode == Opaque {
                    encoder.set_color(ColorType::Grayscale);
                    encoder.set_depth(BitDepth::Eight);
                    let mut writer = encoder.write_header()?;
                    let data: Vec<u8> = image.pixels().iter().map(|pixel| pixel.red()).collect();
                    writer.write_image_data(data.as_slice())?;
                } else {
                    encoder.set_color(ColorType::GrayscaleAlpha);
                    encoder.set_depth(BitDepth::Eight);
                    let mut writer = encoder.write_header()?;
                    let data: Vec<u8> = image.pixels().iter().flat_map(|pixel| {
                        let demul_red = pixel.demultiply().red();
                        [demul_red, pixel.alpha()]
                    }).collect();
                    writer.write_image_data(data.as_slice())?;
                }

            }
            Rgb => {
                for pixel in image.pixels_mut() {
                    unsafe {
                        // Treat this PremultipliedColorU8 slice as a ColorU8 slice
                        *pixel = transmute(pixel.demultiply());
                    }
                }
                match self.transparency_mode {
                    Opaque => {
                        info!("Writing an RGB PNG");
                        encoder.set_color(ColorType::Rgb);
                        let mut data = Vec::with_capacity(3 * image.pixels().len());
                        let mut writer = encoder.write_header()?;
                        for pixel in image.pixels() {
                            data.push(pixel.red());
                            data.push(pixel.green());
                            data.push(pixel.blue());
                        }
                        writer.write_image_data(&data)?;
                    }
                    BinaryTransparency => {
                        let mut data: Vec<u8> = Vec::with_capacity(3 * image.pixels().len());
                        for pixel in image.pixels() {
                            let pixel_color: ComparableColor = (*pixel).into();
                            if pixel_color.red().abs_diff(0xc0) <= 1
                                    && pixel_color.green() >= 0xfe
                                    && pixel_color.blue().abs_diff(0x3e) <= 1 {
                                panic!("Found pixel color {} close to reserved-for-transparency color",
                                    pixel_color);
                            }
                            data.extend(&cast::<PremultipliedColorU8, [u8; 4]>(*pixel)[0..3]);
                        }
                        encoder.set_color(ColorType::Rgb);
                        encoder.set_trns(vec![0xc0, 0xff, 0x3e]);
                        info!("Writing an RGB PNG with a transparent color");
                        let mut writer = encoder.write_header()?;
                        writer.write_image_data(&data)?;
                    }
                    AlphaChannel => {
                        info!("Writing an RGBA PNG");
                        encoder.set_color(ColorType::Rgba);
                        let mut writer = encoder.write_header()?;
                        writer.write_image_data(image.data())?;
                    }
                }
            }
        }
        Ok(MaybeFromPool::FromPool {reusable})
    }
}

impl ToAlphaChannelTaskSpec {
    fn is_semitransparency_free(&self) -> bool {
        match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { alpha, base } =>
                (*alpha == 1.0 && base.is_semitransparency_free()) || *alpha == 0.0,
            ToAlphaChannelTaskSpec::FromPixmap { base } => base.get_transparency_mode() != AlphaChannel,
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { background, foreground } =>
                background.is_semitransparency_free() && foreground.is_semitransparency_free(),
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background, foreground } => {
                *background == 1.0 || (*background == 0.0 && foreground.is_semitransparency_free())
            }
        }
    }
}

lazy_static!{
    pub static ref SEMITRANSPARENT_BLACK_PALETTE: Vec<ComparableColor> = (0..=u8::MAX).map(|alpha| ComparableColor::from(
        ColorU8::from_rgba(0, 0, 0, alpha))).collect();
}

impl ToPixmapTaskSpec {
    /// Used in [TaskSpec::add_to] to deduplicate certain tasks that are redundant.
    fn get_color_mode(&self) -> PngColorMode {
        match self {
            ToPixmapTaskSpec::None { .. } => panic!("get_discrete_colors() called on None task"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let frame_color_modes: Vec<PngColorMode> = frames.iter().
                    map(|frame| ToPixmapTaskSpec::StackLayerOnLayer {
                        background: background.to_owned(), foreground: frame.to_owned().into()
                    }.get_color_mode()).collect();
                if frame_color_modes.contains(&Rgb) {
                    Rgb
                } else if frame_color_modes.contains(&Grayscale) {
                    Grayscale
                } else {
                    let mut combined_colors: HashSet<ComparableColor> = HashSet::new();
                    for frame_color_mode in frame_color_modes {
                        let Indexed(colors) = frame_color_mode
                            else {
                                panic!("Found non-indexed mode after ruling it out")
                            };
                        combined_colors.extend(colors);
                    }
                    Indexed(combined_colors.into_iter().collect())
                }
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source = &&*source.to_string_lossy();
                if COLOR_SVGS.contains(source) {
                    Rgb
                } else if SEMITRANSPARENCY_FREE_SVGS.contains(source) {
                    Indexed(vec![ComparableColor::TRANSPARENT, ComparableColor::BLACK])
                } else {
                    Indexed(SEMITRANSPARENT_BLACK_PALETTE.to_owned())
                }
            },
            ToPixmapTaskSpec::PaintAlphaChannel { color, base } => {
                if base.is_semitransparency_free() {
                    Indexed(vec![ComparableColor::TRANSPARENT, *color])
                } else {
                    let max_alpha = color.alpha();
                    Indexed((0..=max_alpha).map(|alpha| ComparableColor::from(ColorU8::from_rgba(
                        color.red(), color.green(), color.blue(), alpha
                    ))).collect())
                }
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                match foreground.get_transparency_mode() {
                    Opaque => foreground.get_color_mode(),
                    AlphaChannel => {
                        if background.is_gray() && foreground.get_color_mode().is_grayscale_compatible() {
                            Grayscale
                        } else {
                            Rgb
                        }
                    }
                    BinaryTransparency => {
                        match foreground.get_color_mode() {
                            Rgb => Rgb,
                            Grayscale => if background.is_gray() {
                                Grayscale
                            } else {
                                Rgb
                            },
                            Indexed(fg_palette) => {
                                let mut combined_colors = HashSet::with_capacity(fg_palette.len() * fg_palette.len());
                                for fg_color in fg_palette {
                                    combined_colors.insert(fg_color.blend_atop(background));
                                }
                                Indexed(combined_colors.into_iter().collect())
                            }
                        }
                    }
                }
            }
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                match foreground.get_transparency_mode() {
                    Opaque => foreground.get_color_mode(),
                    BinaryTransparency => {
                        if let Indexed(fg_palette) = foreground.get_color_mode()
                                && let Indexed(bg_palette) = background.get_color_mode()
                                && let combined_size = bg_palette.len() * fg_palette.len()
                                && combined_size <= u8::MAX as usize + 1 {
                            let mut combined_colors = HashSet::with_capacity(combined_size);
                            for bg_color in bg_palette {
                                for fg_color in fg_palette.iter() {
                                    combined_colors.insert(fg_color.blend_atop(&bg_color));
                                }
                            }
                            Indexed(combined_colors.into_iter().collect())
                        } else if background.get_color_mode().is_grayscale_compatible()
                                && foreground.get_color_mode().is_grayscale_compatible() {
                            Grayscale
                        } else {
                            Rgb
                        }
                    },
                    AlphaChannel => {
                        if background.get_color_mode().is_grayscale_compatible()
                                && foreground.get_color_mode().is_grayscale_compatible() {
                            Grayscale
                        } else {
                            Rgb
                        }
                    },
                }
            }
        }
    }

    fn is_all_black(&self) -> bool {
        match self.get_color_mode() {
            Indexed(colors) => colors.iter().all(ComparableColor::is_black_or_transparent),
            _ => false
        }
    }

    fn get_transparency_mode(&self) -> PngTransparencyMode {
        match self {
            ToPixmapTaskSpec::Animate { background, frames } => {
                match background.get_transparency_mode() {
                    AlphaChannel => {
                        if frames.iter().all(|frame| frame.get_transparency_mode() == Opaque) {
                            Opaque
                        } else {
                            AlphaChannel
                        }
                    },
                    Opaque => Opaque,
                    BinaryTransparency => {
                        let mut has_transparency = false;
                        for frame in frames {
                            match frame.get_transparency_mode() {
                                AlphaChannel => return AlphaChannel,
                                BinaryTransparency => has_transparency = true,
                                Opaque => {}
                            }
                        }
                        if has_transparency {
                            BinaryTransparency
                        } else {
                            Opaque
                        }
                    }
                }
            }
            ToPixmapTaskSpec::FromSvg { source } => {
                if SEMITRANSPARENCY_FREE_SVGS.contains(&&*source.to_string_lossy()) {
                    BinaryTransparency
                } else {
                    AlphaChannel
                }
            },
            ToPixmapTaskSpec::PaintAlphaChannel { color, base } => {
                if color.alpha() == u8::MAX && base.is_semitransparency_free() {
                    BinaryTransparency
                } else {
                    AlphaChannel
                }
            }
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                match background.alpha() {
                    u8::MAX => Opaque,
                    0 => foreground.get_transparency_mode(),
                    _ => AlphaChannel
                }
            }
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                match background.get_transparency_mode() {
                    Opaque => Opaque,
                    BinaryTransparency => foreground.get_transparency_mode(),
                    AlphaChannel => if foreground.get_transparency_mode() == Opaque {
                        Opaque
                    } else {
                        AlphaChannel
                    }
                }
            }
            ToPixmapTaskSpec::None { .. } => panic!("get_transparency_mode() called on None task"),
        }
    }
}

impl From<ToPixmapTaskSpec> for ToAlphaChannelTaskSpec {
    fn from(value: ToPixmapTaskSpec) -> Self {
        ToAlphaChannelTaskSpec::FromPixmap {base: Box::new(value)}
    }
}

pub struct TaskGraphBuildingContext {
    pixmap_task_to_future_map: HashMap<ToPixmapTaskSpec, CloneableLazyTask<MaybeFromPool<Pixmap>>>,
    alpha_task_to_future_map: HashMap<ToAlphaChannelTaskSpec, CloneableLazyTask<MaybeFromPool<Mask>>>,
    pub output_task_to_future_map: HashMap<FileOutputTaskSpec, CloneableLazyTask<()>>
}

impl TaskGraphBuildingContext {
    pub(crate) fn new() -> Self {
        TaskGraphBuildingContext {
            pixmap_task_to_future_map: HashMap::new(),
            alpha_task_to_future_map: HashMap::new(),
            output_task_to_future_map: HashMap::new()
        }
    }
}

pub const SVG_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/svg");
pub const METADATA_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/metadata");

lazy_static! {
    pub static ref ASSET_DIR: &'static Path = Path::new("assets/minecraft/textures");
}

pub fn name_to_out_path(name: &str) -> PathBuf {
    ASSET_DIR.join(format!("{}.png", name))
}

pub fn name_to_svg_path(name: &str) -> PathBuf {
    PathBuf::from(format!("{}.svg", name))
}

pub fn from_svg_task(name: &str) -> ToPixmapTaskSpec {
    ToPixmapTaskSpec::FromSvg {source: name_to_svg_path(name)}
}

pub fn svg_alpha_task(name: &str) -> ToAlphaChannelTaskSpec {
    ToAlphaChannelTaskSpec::from(from_svg_task(name))
}


pub fn paint_task(base: ToAlphaChannelTaskSpec, color: ComparableColor) -> ToPixmapTaskSpec {
    if color == ComparableColor::BLACK
            && let ToAlphaChannelTaskSpec::FromPixmap {base: base_base} = &base
            && base_base.is_all_black() {
        info!("Simplified {}@{} -> {}", base, color, base_base);
        *(base_base.to_owned())
    } else {
        ToPixmapTaskSpec::PaintAlphaChannel { base: Box::new(base), color }
    }
}

pub fn paint_svg_task(name: &str, color: ComparableColor) -> ToPixmapTaskSpec {
    if color == ComparableColor::BLACK && !COLOR_SVGS.contains(&&*name_to_svg_path(name).to_string_lossy()) {
        info!("Simplified {}@{} -> {}", name, color, name);
        from_svg_task(name)
    } else {
        paint_task(ToAlphaChannelTaskSpec::FromPixmap { base: Box::new(from_svg_task(name)) },
                   color)
    }
}

pub fn out_task(name: &str, base: ToPixmapTaskSpec) -> FileOutputTaskSpec {
    FileOutputTaskSpec::PngOutput {base, destination: name_to_out_path(name) }
}

fn stack_alpha_presorted(mut layers: Vec<ToAlphaChannelTaskSpec>) -> ToAlphaChannelTaskSpec {
    match layers.len() {
        0 => panic!("Attempt to create empty stack of alpha channels"),
        1 => layers[0].to_owned(),
        x => {
            let last = layers.remove(x - 1);
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha {
                background: stack_alpha_presorted(layers).into(),
                foreground: Box::new(last)
            }
        }
    }
}

pub fn stack_alpha(layers: Vec<ToAlphaChannelTaskSpec>) -> ToAlphaChannelTaskSpec {
    let mut layers: Vec<ToAlphaChannelTaskSpec> = layers;
    layers.sort();
    stack_alpha_presorted(layers)
}

pub fn stack(background: ToPixmapTaskSpec, foreground: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
    if foreground.get_transparency_mode() == Opaque {
        panic!("{} would completely occlude {}", foreground, background);
    }
    if let ToPixmapTaskSpec::PaintAlphaChannel {base: fg_base, color: fg_color} = &foreground {
        if let ToPixmapTaskSpec::PaintAlphaChannel { base: bg_base, color: bg_color } = &background
            && fg_color == bg_color {
            // Simplify: merge two adjacent PaintAlphaChannel tasks using same color
            let simplified = paint_task(
                stack_alpha(vec![*bg_base.to_owned(), *fg_base.to_owned()]),
                fg_color.to_owned()
            );
            info!("Simplified ({},{}) -> {}", background, foreground, simplified);
            return simplified;
        } else if let ToPixmapTaskSpec::StackLayerOnLayer { background: bg_bg, foreground: bg_fg } = &background
            && let ToPixmapTaskSpec::PaintAlphaChannel { base: bg_fg_base, color: bg_fg_color } = &**bg_fg
            && fg_color == bg_fg_color {
            // Simplify: merge top two layers
            let simplified = stack(*bg_bg.to_owned(),
                                   paint_task(stack_alpha(vec![*bg_fg_base.to_owned(), *fg_base.to_owned()]), fg_color.to_owned())
            );
            info!("Simplified ({},{}) -> {}", background, foreground, simplified);
            return simplified;
        }
    } else if let ToPixmapTaskSpec::PaintAlphaChannel {base: bg_base, color: bg_color} = &background
                && let ToPixmapTaskSpec::StackLayerOnLayer {background: fg_bg, foreground: fg_fg} = &foreground
                && let ToPixmapTaskSpec::PaintAlphaChannel {base: fg_bg_base, color: fg_bg_color} = &**fg_bg
                && fg_bg_color == bg_color {
        // Simplify: merge bottom two layers
        let simplified = stack(
            paint_task(stack_alpha(vec![*bg_base.to_owned(), *fg_bg_base.to_owned()]), bg_color.to_owned()),
            *fg_fg.to_owned()
        );
        info!("Simplified ({},{}) -> {}", background, foreground, simplified);
        return simplified;
    }
    ToPixmapTaskSpec::StackLayerOnLayer {
        background: Box::new(background), foreground: Box::new(foreground)
    }
}

#[macro_export]
macro_rules! stack {
    ( $first_layer:expr, $second_layer:expr $(,)? ) => {
        $crate::image_tasks::task_spec::stack($first_layer.into(), $second_layer.into())
    };
    ( $first_layer:expr, $second_layer:expr, $( $more_layers:expr ),+ $(,)? ) => {{
        let mut layers_so_far = $crate::stack!($first_layer, $second_layer);
        $( layers_so_far = $crate::stack!(layers_so_far, $more_layers); )+
        layers_so_far
    }};
}

#[macro_export]
macro_rules! stack_on {
    ( $background:expr, $foreground:expr $(,)? ) => {
        if $background == $crate::image_tasks::color::ComparableColor::TRANSPARENT {
            $foreground
        } else {
            $crate::image_tasks::task_spec::ToPixmapTaskSpec::StackLayerOnColor {
                background: $background,
                foreground: Box::new($foreground.into())
            }
        }
    };
    ( $background:expr, $first_layer:expr, $( $more_layers:expr ),+ ) => {{
        $crate::stack_on!($background, $crate::stack!($first_layer, $($more_layers),+))
    }};
}

#[macro_export]
macro_rules! paint_stack {
    ( $color:expr, $( $layers:expr ),* $(,)? ) => {
        $crate::image_tasks::task_spec::paint_task(
            $crate::stack_alpha!($($layers),*).into(),
            $color)
    }
}

#[macro_export]
macro_rules! stack_alpha {
    ( $( $layers:expr ),* $(,)? ) => {
        $crate::image_tasks::task_spec::stack_alpha(vec![
            $(
                $crate::image_tasks::task_spec::svg_alpha_task($layers)
            ),*
        ])
    };
}

impl FromStr for ToPixmapTaskSpec {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ToPixmapTaskSpec::FromSvg {
            source: name_to_svg_path(s)
        })
    }
}

impl Mul<f32> for ToAlphaChannelTaskSpec {
    type Output = ToAlphaChannelTaskSpec;

    fn mul(self, rhs: f32) -> Self::Output {
        if rhs == 1.0 {
            self
        } else {
            ToAlphaChannelTaskSpec::MakeSemitransparent {
                base: Box::new(self),
                alpha: OrderedFloat::from(rhs)
            }
        }
    }
}

impl Mul<ComparableColor> for ToAlphaChannelTaskSpec {
    type Output = ToPixmapTaskSpec;

    fn mul(self, rhs: ComparableColor) -> Self::Output {
        paint_task(self, rhs)
    }
}

impl Mul<ComparableColor> for ToPixmapTaskSpec {
    type Output = ToPixmapTaskSpec;
    fn mul(self, rhs: ComparableColor) -> Self::Output {
        match &self {
            ToPixmapTaskSpec::PaintAlphaChannel { base, .. } => {
                ToPixmapTaskSpec::PaintAlphaChannel {
                    base: Box::new(*base.to_owned()),
                    color: rhs
                }
            },
            _ => ToPixmapTaskSpec::PaintAlphaChannel {
                base: Box::new(ToAlphaChannelTaskSpec::FromPixmap { base: Box::new(self) }),
                color: rhs
            }
        }
    }
}
