use std::collections::{HashMap};

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

use std::ops::{Deref, DerefMut, Mul};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use cached::lazy_static::lazy_static;
use crate::anyhoo;
use include_dir::{Dir, include_dir};
use itertools::Itertools;

use log::{info};
use ordered_float::OrderedFloat;
use png::{BitDepth};
use replace_with::replace_with_and_return;

use resvg::tiny_skia::{Color, ColorU8, Mask, Pixmap};

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::{c, ComparableColor, gray};
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg, SEMITRANSPARENCY_FREE_SVGS};
use crate::image_tasks::make_semitransparent::{create_alpha_array, make_semitransparent};
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::png_output::{copy_out_to_out, png_output};
use crate::image_tasks::repaint::{paint, pixmap_to_mask};
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_alpha_on_background, stack_layer_on_background, stack_layer_on_layer};
use crate::image_tasks::task_spec::ColorDescription::{Rgb, SpecifiedColors};
use crate::image_tasks::task_spec::PngMode::{GrayscaleAlpha, GrayscaleOpaque, GrayscaleWithTransparentShade, IndexedRgba, IndexedRgbOpaque, Rgba, RgbOpaque, RgbWithTransparentShade};
use crate::image_tasks::task_spec::Transparency::{AlphaChannel, BinaryTransparency, Opaque};
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
                let background_opaque = background.get_transparency(ctx) == Opaque;
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
                let png_mode = color_description_to_mode(base, ctx);
                Box::new(move || {
                    let base_result = base_future.into_result()?;
                    Ok(Box::new(png_output(*Arc::unwrap_or_clone(base_result),
                                           png_mode, &destination)?))
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

#[derive(Clone)]
pub enum ColorDescription {
    SpecifiedColors(Vec<ComparableColor>),
    Rgb(Transparency)
}

impl Transparency {
    pub fn stack_on(&self, other: &Transparency) -> Transparency {
        if *self == Opaque || *other == Opaque {
            Opaque
        } else if *self == BinaryTransparency && *other == BinaryTransparency {
            BinaryTransparency
        } else {
            AlphaChannel
        }
    }

    pub fn put_adjacent(&self, other: &Transparency) -> Transparency {
        if *self == AlphaChannel || *other == AlphaChannel {
            AlphaChannel
        } else if *self == Opaque && *other == Opaque {
            Opaque
        } else {
            BinaryTransparency
        }
    }
}

impl ColorDescription {
    pub fn transparency(&self) -> Transparency {
        match self {
            SpecifiedColors(colors) => {
                let mut have_transparent = false;
                for color in colors {
                    match color.alpha() {
                        0 => have_transparent = true,
                        u8::MAX => {},
                        _ => return AlphaChannel
                    }
                }
                if have_transparent {
                    BinaryTransparency
                } else {
                    Opaque
                }
            },
            Rgb(transparency) => *transparency
        }
    }

    pub fn stack_on(self, background: &ColorDescription) -> ColorDescription {
        match background {
            Rgb(transparency) => Rgb(self.transparency().stack_on(transparency)),
            SpecifiedColors(bg_colors) => {
                match self {
                    Rgb(transparency) => Rgb(transparency.stack_on(&background.transparency())),
                    SpecifiedColors(self_colors) => {
                        let mut combined_colors: Vec<ComparableColor> = self_colors.iter().flat_map(|fg_color| {
                                match fg_color.alpha() {
                                    u8::MAX => vec![*fg_color],
                                    0 => bg_colors.to_owned(),
                                    _ => bg_colors.iter().map(move |bg_color| fg_color.blend_atop(bg_color)).collect()
                                }.into_iter()
                        }).collect();
                        combined_colors.sort();
                        combined_colors.dedup();
                        SpecifiedColors(combined_colors)
                    }
                }
            }
        }
    }

    pub fn put_adjacent(&self, adjacent: &ColorDescription) -> ColorDescription {
        match adjacent {
            Rgb(transparency) => Rgb(self.transparency().put_adjacent(transparency)),
            SpecifiedColors(bg_colors) => {
                match self {
                    Rgb(transparency) => Rgb(transparency.put_adjacent(&adjacent.transparency())),
                    SpecifiedColors(self_colors) => {
                        let mut combined_colors = self_colors.to_owned();
                        combined_colors.extend(bg_colors);
                        combined_colors.sort();
                        combined_colors.dedup();
                        SpecifiedColors(combined_colors)
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PngMode {
    IndexedRgbOpaque(Vec<ComparableColor>),
    IndexedRgba(Vec<ComparableColor>),
    GrayscaleOpaque(BitDepth),
    GrayscaleWithTransparentShade {
        bit_depth: BitDepth,
        transparent_shade: u8,
    },
    GrayscaleAlpha(BitDepth),
    RgbOpaque,
    RgbWithTransparentShade(ComparableColor),
    Rgba
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Transparency {
    Opaque,
    BinaryTransparency,
    AlphaChannel
}

fn palette_bits_per_pixel(len: usize) -> usize {
    if len <= 2 {
        1
    } else if len <= 4 {
        2
    } else if len <= 16 {
        4
    } else if len <= 256 {
        8
    } else {
        panic!("Indexed mode with more than 256 colors not supported")
    }
}

impl PngMode {
    pub fn bits_per_pixel(&self) -> usize {
        match self {
            IndexedRgbOpaque(palette) => palette_bits_per_pixel(palette.len()),
            IndexedRgba(palette) => palette_bits_per_pixel(palette.len()),
            GrayscaleOpaque(bit_depth) => bit_depth_to_u32(bit_depth) as usize,
            GrayscaleWithTransparentShade { bit_depth, .. } => bit_depth_to_u32(bit_depth) as usize,
            GrayscaleAlpha(bit_depth) => 2 * bit_depth_to_u32(bit_depth) as usize,
            RgbOpaque => 24,
            RgbWithTransparentShade(_) => 24,
            Rgba => 32
        }
    }
}

pub fn channel_to_bit_depth(input: u8, depth: BitDepth) -> u16 {
    match depth {
        BitDepth::One => if input < 0x80 { 0 } else { 1 },
        BitDepth::Two => {
            (input as u16 + (0x055/2)) / 0x55
        },
        BitDepth::Four => {
            (input as u16 + (0x011/2)) / 0x11
        },
        BitDepth::Eight => input as u16,
        BitDepth::Sixteen => input as u16 * 0x101
    }
}

pub fn bit_depth_to_u32(depth: &BitDepth) -> u32 {
    match depth {
        BitDepth::One => 1,
        BitDepth::Two => 2,
        BitDepth::Four => 4,
        BitDepth::Eight => 8,
        BitDepth::Sixteen => 16
    }
}

lazy_static! {
    static ref ALL_ALPHA_VALUES: Vec<u8> = (0..=u8::MAX).collect();
}

impl ToAlphaChannelTaskSpec {
    fn get_possible_alpha_values(&self, ctx: &mut TaskGraphBuildingContext) -> Vec<u8> {
        if let Some(alpha_vec) = ctx.alpha_task_to_alpha_map.get(self) {
            return alpha_vec.to_owned();
        }
        let alpha_vec: Vec<u8> = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { alpha, base } => {
                let alpha_array = create_alpha_array(*alpha);
                let mut values: Vec<u8> = base.get_possible_alpha_values(ctx).into_iter().map(|alpha| alpha_array[alpha as usize]).collect();
                values.sort();
                values.dedup();
                values
            }
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                if let ToPixmapTaskSpec::FromSvg { source } = &**base
                        && !SEMITRANSPARENCY_FREE_SVGS.contains(&&*source.to_string_lossy()) {
                    ALL_ALPHA_VALUES.to_owned()
                } else {
                    match base.get_transparency(ctx) {
                        Opaque => vec![u8::MAX],
                        BinaryTransparency => vec![0, u8::MAX],
                        AlphaChannel => if let SpecifiedColors(colors) = base.get_color_description(ctx) {
                            let mut alphas: Vec<u8> = colors.iter().map(|color| color.alpha()).collect();
                            alphas.sort();
                            alphas.dedup();
                            alphas
                        } else {
                            ALL_ALPHA_VALUES.to_owned()
                        }
                    }
                }
            }
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { background, foreground } => {
                let fg_values = foreground.get_possible_alpha_values(ctx);
                let mut combined_alphas: Vec<u8> = background.get_possible_alpha_values(ctx).into_iter().flat_map(|background_alpha| {
                    fg_values.iter().map(move |foreground_alpha|
                        (background_alpha as u16 +
                        (*foreground_alpha as u16) * ((u8::MAX - background_alpha) as u16) / (u8::MAX as u16)) as u8
                    )
                }).collect();
                combined_alphas.sort();
                combined_alphas.dedup();
                combined_alphas
            }
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background: background_alpha, foreground } => {
                let background_alpha = (**background_alpha * 255.0) as u8;
                let mut combined_alphas: Vec<u8> = foreground.get_possible_alpha_values(ctx).into_iter().map(
                    move |foreground_alpha| (background_alpha as u16 +
                        (foreground_alpha as u16) * ((u8::MAX - background_alpha) as u16) / (u8::MAX as u16)) as u8
                ).collect();
                combined_alphas.sort();
                combined_alphas.dedup();
                combined_alphas
            }
        };
        ctx.alpha_task_to_alpha_map.insert(self.to_owned(), alpha_vec.to_owned());
        alpha_vec
    }
}

lazy_static!{
    pub static ref SEMITRANSPARENT_BLACK_PALETTE: Vec<ComparableColor> = (0..=u8::MAX).map(|alpha| ComparableColor::from(
        ColorU8::from_rgba(0, 0, 0, alpha))).collect();
}

fn color_description_to_mode(task: &ToPixmapTaskSpec, ctx: &mut TaskGraphBuildingContext) -> PngMode {
    match task.get_color_description(ctx) {
        SpecifiedColors(colors) => {
            let transparency = task.get_transparency(ctx);
            let max_indexed_size = u8::MAX as usize + 1;
            let mut have_non_gray = false;
            for color in &colors {
                if !color.is_gray() {
                    have_non_gray = true;
                }
            }
            if colors.len() >= max_indexed_size && have_non_gray {
                return match transparency {
                    Opaque => RgbOpaque,
                    BinaryTransparency => RgbWithTransparentShade(c(0xc0ff3e)),
                    AlphaChannel => Rgba
                };
            }
            if colors.len() >= max_indexed_size && !have_non_gray {
                if transparency == Opaque {
                    GrayscaleOpaque(BitDepth::Eight)
                } else {
                    GrayscaleAlpha(BitDepth::Eight)
                }
            } else {
                let indexed_mode = if transparency == Opaque {
                    IndexedRgbOpaque(colors.to_owned())
                } else {
                    IndexedRgba(colors.to_owned())
                };
                if have_non_gray {
                    return indexed_mode;
                }
                let grayscale_bit_depth = colors.iter().max_by_key(
                    |color| bit_depth_to_u32(&color.bit_depth()))
                    .unwrap().bit_depth();
                let grayscale_mode = match transparency {
                    AlphaChannel => GrayscaleAlpha(grayscale_bit_depth),
                    BinaryTransparency => {
                        let grayscale_shades = match grayscale_bit_depth {
                            BitDepth::One => vec![ComparableColor::BLACK, ComparableColor::WHITE],
                            BitDepth::Two => vec![gray(0x00), gray(0x55), gray(0xAA), gray(0xFF)],
                            BitDepth::Four => (0..16).map(|n| gray(n * 0x11)).collect(),
                            BitDepth::Eight => (0..=u8::MAX).map(gray).collect(),
                            BitDepth::Sixteen => panic!("16-bit greyscale not handled")
                        };
                        match grayscale_shades.into_iter().find(|color| !colors.contains(color)) {
                            Some(unused) => GrayscaleWithTransparentShade {
                                bit_depth: grayscale_bit_depth,
                                transparent_shade: unused.red()
                            },
                            None => GrayscaleAlpha(grayscale_bit_depth)
                        }
                    },
                    Opaque => GrayscaleOpaque(grayscale_bit_depth)
                };
                if grayscale_mode.bits_per_pixel() <= indexed_mode.bits_per_pixel() {
                    grayscale_mode
                } else {
                    indexed_mode
                }
            }
        },
        Rgb(Opaque) => RgbOpaque,
        Rgb(BinaryTransparency) => RgbWithTransparentShade(c(0xc0ff3e)),
        Rgb(AlphaChannel) => Rgba
    }
}

impl ToPixmapTaskSpec {

    fn get_transparency(&self, ctx: &mut TaskGraphBuildingContext) -> Transparency {
        if let Some(transparency) = ctx.pixmap_task_to_transparency_map.get(self) {
            *transparency
        } else {
            let transparency = self.get_color_description(ctx).transparency();
            ctx.pixmap_task_to_transparency_map.insert(self.to_owned(), transparency);
            transparency
        }
    }

    /// Used in [TaskSpec::add_to] to deduplicate certain tasks that are redundant.
    fn get_color_description(&self, ctx: &mut TaskGraphBuildingContext) -> ColorDescription {
        if let Some(desc) = ctx.pixmap_task_to_color_map.get(self) {
            return (*desc).to_owned();
        }
        let desc = match self {
            ToPixmapTaskSpec::None { .. } => panic!("get_discrete_colors() called on None task"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let background_desc = background.get_color_description(ctx);
                let mut current_desc: Option<ColorDescription> = None;
                for frame in frames {
                    let frame_desc
                        = frame.get_color_description(ctx).stack_on(&background_desc);
                    current_desc = Some(match current_desc {
                        None => frame_desc,
                        Some(other_frames_desc) => frame_desc.put_adjacent(&other_frames_desc)
                    });
                }
                current_desc.unwrap()
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source: &str = &source.to_string_lossy();
                if COLOR_SVGS.contains(&source) {
                    if SEMITRANSPARENCY_FREE_SVGS.contains(&source) {
                        Rgb(BinaryTransparency)
                    } else {
                        Rgb(AlphaChannel)
                    }
                } else if SEMITRANSPARENCY_FREE_SVGS.contains(&source) {
                    SpecifiedColors(vec![ComparableColor::BLACK, ComparableColor::TRANSPARENT])
                } else {
                    SpecifiedColors(SEMITRANSPARENT_BLACK_PALETTE.to_owned())
                }
            },
            ToPixmapTaskSpec::PaintAlphaChannel { color, base } => {
                SpecifiedColors({
                    let mut alphas: Vec<ComparableColor> = base
                        .get_possible_alpha_values(ctx)
                        .into_iter()
                        .map(|alpha| *color * (alpha as f32 / 255.0))
                        .collect();
                    alphas.sort();
                    alphas.dedup();
                    alphas
                })
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                let background = *background;
                foreground.get_color_description(ctx).stack_on(&SpecifiedColors(vec![background]))
            }
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                foreground.get_color_description(ctx).stack_on(&background.get_color_description(ctx))
            }
        };
        ctx.pixmap_task_to_color_map.insert(self.to_owned(), desc.to_owned());
        desc
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
    pub output_task_to_future_map: HashMap<FileOutputTaskSpec, CloneableLazyTask<()>>,
    pixmap_task_to_color_map: HashMap<ToPixmapTaskSpec, ColorDescription>,
    alpha_task_to_alpha_map: HashMap<ToAlphaChannelTaskSpec, Vec<u8>>,
    pixmap_task_to_transparency_map: HashMap<ToPixmapTaskSpec, Transparency>
}

impl TaskGraphBuildingContext {
    pub(crate) fn new() -> Self {
        TaskGraphBuildingContext {
            pixmap_task_to_future_map: HashMap::new(),
            alpha_task_to_future_map: HashMap::new(),
            output_task_to_future_map: HashMap::new(),
            pixmap_task_to_color_map: HashMap::new(),
            alpha_task_to_alpha_map: HashMap::new(),
            pixmap_task_to_transparency_map: HashMap::new()
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
    if let ToAlphaChannelTaskSpec::FromPixmap {base: ref base_base} = base {
        match &**base_base {
            ToPixmapTaskSpec::FromSvg { ref source } => {
                if color == ComparableColor::BLACK
                    && !COLOR_SVGS.contains(&&*source.to_string_lossy()) {
                    info!("Simplified {}@{} -> {}", base, color, base_base);
                    return *base_base.to_owned();
                }
            },
            ToPixmapTaskSpec::PaintAlphaChannel {base: base_base_base, color: base_color } => {
                if base_color.alpha() == u8::MAX {
                    info!("Simplified {}@{} -> {}", base, color, base_base_base);
                    return paint_task(*base_base_base.to_owned(), color);
                }
            },
            _ => {}
        }
    }
    ToPixmapTaskSpec::PaintAlphaChannel { base: Box::new(base), color }
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
