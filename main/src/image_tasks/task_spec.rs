use std::collections::{HashMap};

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

use std::ops::{Deref, DerefMut, Mul};
use std::sync::{Arc, Mutex};

use crate::{anyhoo, debug_assert_unreachable};
use include_dir::{Dir, include_dir};
use itertools::{Itertools};

use log::{info};
use png::{BitDepth};
use replace_with::replace_with_and_return;

use resvg::tiny_skia::{Color, Mask, Pixmap};

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::{BIT_DEPTH_FOR_CHANNEL, c, ComparableColor, gray};
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg, SEMITRANSPARENCY_FREE_SVGS};
use crate::image_tasks::make_semitransparent::{ALPHA_MULTIPLICATION_TABLE, make_semitransparent};
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::png_output::{copy_out_to_out, png_output};
use crate::image_tasks::repaint::{paint, pixmap_to_mask};
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_alpha_on_background, stack_alpha_pixel, stack_layer_on_background, stack_layer_on_layer};
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
                let source = source.to_string();
                Box::new(move || {
                    Ok(Box::new(from_svg(source, *TILE_SIZE)?))
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
                let base_future = base.add_to(ctx);
                let alpha = *alpha;
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
                let background = *background;
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
            FileOutputTaskSpec::PngOutput {base, .. } => {
                let destination_path = self.get_path();
                let base_future = base.add_to(ctx);
                let png_mode = color_description_to_mode(base, ctx);
                Box::new(move || {
                    let base_result = base_future.into_result()?;
                    Ok(Box::new(png_output(*Arc::unwrap_or_clone(base_result),
                                           png_mode, destination_path)?))
                })
            }
            FileOutputTaskSpec::Copy {original, ..} => {
                let link = self.get_path();
                let original_path = original.get_path();
                let base_future = original.add_to(ctx);
                Box::new(move || {
                    base_future.into_result()?;
                    Ok(Box::new(copy_out_to_out(original_path, link)?))
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
    FromSvg {source: String},
    PaintAlphaChannel {base: Box<ToAlphaChannelTaskSpec>, color: ComparableColor},
    StackLayerOnColor {background: ComparableColor, foreground: Box<ToPixmapTaskSpec>},
    StackLayerOnLayer {background: Box<ToPixmapTaskSpec>, foreground: Box<ToPixmapTaskSpec>},
    None {},
}

/// [TaskSpec] for a task that produces an [AlphaChannel].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToAlphaChannelTaskSpec {
    MakeSemitransparent {base: Box<ToAlphaChannelTaskSpec>, alpha: u8},
    FromPixmap {base: ToPixmapTaskSpec},
    StackAlphaOnAlpha {background: Box<ToAlphaChannelTaskSpec>, foreground: Box<ToAlphaChannelTaskSpec>},
    StackAlphaOnBackground {background: u8, foreground: Box<ToAlphaChannelTaskSpec>}
}

/// [TaskSpec] for a task that doesn't produce a heap object as output.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FileOutputTaskSpec {
    PngOutput {base: ToPixmapTaskSpec, destination_name: String},
    Copy {original: Box<FileOutputTaskSpec>, link_name: String}
}

impl FileOutputTaskSpec {
    pub(crate) fn get_path(&self) -> String {
        match self {
            FileOutputTaskSpec::PngOutput { destination_name, .. } => {
                let mut out_path = ASSET_DIR.to_string();
                out_path.push_str(destination_name);
                out_path.push_str(".png");
                out_path
            },
            FileOutputTaskSpec::Copy { link_name, .. } => {
                let mut out_path = ASSET_DIR.to_string();
                out_path.push_str(link_name);
                out_path.push_str(".png");
                out_path
            }
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
                f.write_str(source)
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
            FileOutputTaskSpec::PngOutput { .. } => {
                self.get_path()
            },
            FileOutputTaskSpec::Copy { original, .. } => {
                format!("symlink({} -> {})", self.get_path(), original.get_path())
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

fn stack_alpha_vecs(background: &[u8], foreground: &[u8]) -> Vec<u8> {
    let mut combined: Vec<u8> = background.iter().flat_map(|bg_alpha|
        foreground.iter().map(move |fg_alpha| stack_alpha_pixel(*bg_alpha, *fg_alpha)))
        .collect();
    combined.sort();
    combined.dedup();
    combined
}

fn multiply_alpha_vec(alphas: &Vec<u8>, rhs: u8) -> Vec<u8> {
    if rhs == 0 {
        vec![0]
    } else if rhs == u8::MAX {
        alphas.to_owned()
    } else {
        let alpha_array = &ALPHA_MULTIPLICATION_TABLE[rhs as usize];
        let mut output: Vec<u8> = alphas.iter().map(|x|
            alpha_array[*x as usize]).collect();
        // Don't need to sort because input is sorted
        output.dedup();
        output
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

const BINARY_SEARCH_THRESHOLD: usize = 1024;

impl ColorDescription {
    pub fn transparency(&self) -> Transparency {
        match self {
            SpecifiedColors(colors) => {
                if contains_semitransparency(colors) {
                    AlphaChannel
                } else if contains_alpha(colors, 0) {
                    BinaryTransparency
                } else {
                    Opaque
                }
            },
            Rgb(transparency) => *transparency
        }
    }

    pub fn stack_on(&self, background: &ColorDescription) -> ColorDescription {
        match background {
            Rgb(transparency) => Rgb(self.transparency().stack_on(transparency)),
            SpecifiedColors(bg_colors) => {
                match &self {
                    Rgb(transparency) => Rgb(transparency.stack_on(&background.transparency())),
                    SpecifiedColors(fg_colors) => {
                        match self.transparency() {
                            Opaque => SpecifiedColors(fg_colors.clone()),
                            BinaryTransparency => {
                                let mut combined_colors = fg_colors.to_owned();
                                combined_colors.extend(bg_colors);
                                combined_colors.sort();
                                combined_colors.dedup();
                                SpecifiedColors(combined_colors)
                            }
                            AlphaChannel => {
                                let mut combined_colors: Vec<ComparableColor> = bg_colors.iter().flat_map(|bg_color|
                                    bg_color.under(fg_colors.iter().copied()).into_iter()
                                ).unique().collect();
                                combined_colors.sort();
                                combined_colors.dedup();
                                SpecifiedColors(combined_colors)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn put_adjacent(&self, neighbor: &ColorDescription) -> ColorDescription {
        match neighbor {
            Rgb(transparency) => Rgb(self.transparency().put_adjacent(transparency)),
            SpecifiedColors(neighbor_colors) => {
                match self {
                    Rgb(transparency) => Rgb(transparency.put_adjacent(&neighbor.transparency())),
                    SpecifiedColors(self_colors) => {
                        let mut combined_colors = self_colors.to_owned();
                        combined_colors.extend(neighbor_colors);
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
    GrayscaleAlpha,
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
            GrayscaleAlpha => 16,
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

pub fn u32_to_bit_depth_max_eight(depth: u32) -> BitDepth {
    match depth {
        1 => BitDepth::One,
        2 => BitDepth::Two,
        4 => BitDepth::Four,
        8 => BitDepth::Eight,
        _ => debug_assert_unreachable()
    }
}

const ALL_U8S: &[u8; u8::MAX as usize + 1] = &ALPHA_MULTIPLICATION_TABLE[u8::MAX as usize];

impl ToAlphaChannelTaskSpec {
    fn get_possible_alpha_values(&self, ctx: &mut TaskGraphBuildingContext) -> Vec<u8> {
        if let Some(alpha_vec) = ctx.alpha_task_to_alpha_map.get(self) {
            return alpha_vec.to_owned();
        }
        let alpha_vec: Vec<u8> = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { alpha, base } => {
                multiply_alpha_vec(&base.get_possible_alpha_values(ctx), *alpha)
            }
            ToAlphaChannelTaskSpec::FromPixmap { base } => {

                     base.get_possible_alpha_values(ctx)
            }
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { background, foreground } => {
                stack_alpha_vecs(&background.get_possible_alpha_values(ctx),
                                 &foreground.get_possible_alpha_values(ctx))
            }
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background: background_alpha, foreground } => {
                stack_alpha_vecs(&[*background_alpha], &foreground.get_possible_alpha_values(ctx))
            }
        };
        ctx.alpha_task_to_alpha_map.insert(self.to_owned(), alpha_vec.to_owned());
        alpha_vec
    }
}

fn color_description_to_mode(task: &ToPixmapTaskSpec, ctx: &mut TaskGraphBuildingContext) -> PngMode {
    match task.get_color_description(ctx) {
        SpecifiedColors(mut colors) => {
            let transparency = task.get_transparency(ctx);
            let have_non_gray = colors.iter().any(|color| !color.is_gray());
            let max_indexed_size = 256;
            colors.truncate(max_indexed_size + 1);
            if colors.len() > max_indexed_size && have_non_gray {
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
                    GrayscaleAlpha
                }
            } else {
                let mut grayscale_bits = 1;
                for color in colors.iter() {
                    let color_bit_depth = bit_depth_to_u32(
                        &BIT_DEPTH_FOR_CHANNEL[color.red() as usize]);
                    grayscale_bits = grayscale_bits.max(color_bit_depth);
                    if grayscale_bits == 8 {
                        break;
                    }
                }
                let grayscale_bit_depth = u32_to_bit_depth_max_eight(grayscale_bits);
                let indexed_mode = if transparency == Opaque {
                    IndexedRgbOpaque(colors.to_owned())
                } else {
                    IndexedRgba(colors.to_owned())
                };
                if have_non_gray {
                    return indexed_mode;
                }
                let grayscale_mode = match transparency {
                    AlphaChannel => GrayscaleAlpha,
                    BinaryTransparency => {
                        let grayscale_shades = match grayscale_bit_depth {
                            BitDepth::One => vec![ComparableColor::BLACK, ComparableColor::WHITE],
                            BitDepth::Two => vec![gray(0x00), gray(0x55), gray(0xAA), gray(0xFF)],
                            BitDepth::Four => (0..16).map(|n| gray(n * 0x11)).collect(),
                            BitDepth::Eight => ALL_U8S.iter().copied().map(gray).collect(),
                            BitDepth::Sixteen => debug_assert_unreachable()
                        };
                        match grayscale_shades.into_iter().find(|color| !colors.contains(color)) {
                            Some(unused) => GrayscaleWithTransparentShade {
                                bit_depth: grayscale_bit_depth,
                                transparent_shade: unused.red()
                            },
                            None => match grayscale_bit_depth {
                                BitDepth::One => GrayscaleWithTransparentShade {
                                    bit_depth: BitDepth::Two, transparent_shade: 0x55
                                },
                                BitDepth::Two => GrayscaleWithTransparentShade {
                                    bit_depth: BitDepth::Four, transparent_shade: 0x22
                                },
                                BitDepth::Four => GrayscaleWithTransparentShade {
                                    bit_depth: BitDepth::Eight, transparent_shade: 0x08
                                },
                                BitDepth::Eight => GrayscaleAlpha,
                                BitDepth::Sixteen => debug_assert_unreachable()
                            }
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

fn contains_alpha(vec: &Vec<ComparableColor>, needle_alpha: u8) -> bool {
    if vec.len() <= BINARY_SEARCH_THRESHOLD {
        vec.iter().any(|color| color.alpha() == needle_alpha)
    } else {
        match vec.binary_search(&ComparableColor {
            alpha: needle_alpha,
            red: 0,
            green: 0,
            blue: 0
        }) {
            Ok(_) => true,
            Err(insert_black_index) => match vec[insert_black_index..].binary_search(&ComparableColor {
                alpha: needle_alpha,
                red: u8::MAX,
                green: u8::MAX,
                blue: u8::MAX
            }) {
                Ok(_) => true,
                Err(insert_white_index) => insert_white_index > 0
            }
        }
    }
}

pub fn contains_semitransparency(vec: &Vec<ComparableColor>) -> bool {
    match vec[0].alpha {
        0 => if vec.len() == 1 {
            false
        } else { match vec[1].alpha {
            0 => panic!("Transparent color included twice"),
            u8::MAX => false,
            _ => true
        }
        }
        u8::MAX => false,
        _ => true,
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
                if COLOR_SVGS.contains(&source.as_str()) {
                    if SEMITRANSPARENCY_FREE_SVGS.contains(&source.as_str()) {
                        Rgb(BinaryTransparency)
                    } else {
                        Rgb(AlphaChannel)
                    }
                } else if SEMITRANSPARENCY_FREE_SVGS.contains(&source.as_str()) {
                    SpecifiedColors(vec![ComparableColor::TRANSPARENT, ComparableColor::BLACK])
                } else {
                    SpecifiedColors(ALL_U8S.iter()
                        .map(|alpha| ComparableColor { red: 0, green: 0, blue: 0, alpha: *alpha}).collect())
                }
            },
            ToPixmapTaskSpec::PaintAlphaChannel { color, base } => {
                SpecifiedColors({
                    let alpha_array = ALPHA_MULTIPLICATION_TABLE[color.alpha() as usize];
                    let mut colored_alphas: Vec<ComparableColor> = base
                        .get_possible_alpha_values(ctx)
                        .into_iter()
                        .map(|alpha| ComparableColor {
                            red: color.red(),
                            green: color.green(),
                            blue: color.blue(),
                            alpha: alpha_array[alpha as usize]
                        })
                        .collect();
                    colored_alphas.sort();
                    colored_alphas.dedup();
                    colored_alphas
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

    fn get_possible_alpha_values(&self, ctx: &mut TaskGraphBuildingContext) -> Vec<u8> {
        if let Some(alphas) = ctx.pixmap_task_to_alpha_map.get(self) {
            alphas.to_owned()
        } else {
            let colors = self.get_color_description(ctx);
            let alphas = match colors.transparency() {
                AlphaChannel => match colors {
                    Rgb(_) => ALL_U8S.to_vec(),
                    SpecifiedColors(colors) => if colors.len() <= BINARY_SEARCH_THRESHOLD {
                        let mut alphas: Vec<u8> = colors.into_iter().map(|color| color.alpha()).collect();
                        alphas.sort();
                        alphas.dedup();
                        alphas
                    } else {
                        ALL_U8S.iter()
                            .filter(|alpha| contains_alpha(&colors, **alpha))
                            .copied()
                            .collect()
                    }
                },
                BinaryTransparency => vec![0, u8::MAX],
                Opaque => vec![u8::MAX]
            };
            ctx.pixmap_task_to_alpha_map.insert(self.to_owned(), alphas.to_owned());
            alphas
        }
    }
}

impl From<ToPixmapTaskSpec> for ToAlphaChannelTaskSpec {
    fn from(value: ToPixmapTaskSpec) -> Self {
        ToAlphaChannelTaskSpec::FromPixmap {base: value}
    }
}

pub struct TaskGraphBuildingContext {
    pixmap_task_to_future_map: HashMap<ToPixmapTaskSpec, CloneableLazyTask<MaybeFromPool<Pixmap>>>,
    alpha_task_to_future_map: HashMap<ToAlphaChannelTaskSpec, CloneableLazyTask<MaybeFromPool<Mask>>>,
    pub output_task_to_future_map: HashMap<FileOutputTaskSpec, CloneableLazyTask<()>>,
    pixmap_task_to_color_map: HashMap<ToPixmapTaskSpec, ColorDescription>,
    alpha_task_to_alpha_map: HashMap<ToAlphaChannelTaskSpec, Vec<u8>>,
    pixmap_task_to_alpha_map: HashMap<ToPixmapTaskSpec, Vec<u8>>,
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
            pixmap_task_to_alpha_map: HashMap::new(),
            pixmap_task_to_transparency_map: HashMap::new()
        }
    }
}

pub const SVG_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/svg");
pub const METADATA_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/metadata");

pub const ASSET_DIR: &str = "assets/minecraft/textures/";

pub fn from_svg_task(name: &str) -> ToPixmapTaskSpec {
    ToPixmapTaskSpec::FromSvg {source: name.to_string()}
}

pub fn svg_alpha_task(name: &str) -> ToAlphaChannelTaskSpec {
    ToAlphaChannelTaskSpec::FromPixmap { base: from_svg_task(name) }
}


pub fn paint_task(base: ToAlphaChannelTaskSpec, color: ComparableColor) -> ToPixmapTaskSpec {
    if let ToAlphaChannelTaskSpec::FromPixmap {base: ref base_base} = base {
        match base_base {
            ToPixmapTaskSpec::FromSvg { ref source } => {
                if color == ComparableColor::BLACK
                    && !COLOR_SVGS.contains(&source.as_str()) {
                    info!("Simplified {}@{} -> {}", base, color, base_base);
                    return base_base.to_owned();
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
    if color == ComparableColor::BLACK
        && !COLOR_SVGS.contains(&name) {
        info!("Simplified {}@{} -> {}", name, color, name);
        from_svg_task(name)
    } else {
        ToPixmapTaskSpec::PaintAlphaChannel {
            base: Box::new(ToAlphaChannelTaskSpec::FromPixmap { base: from_svg_task(name) }),
            color
        }
    }
}

pub fn out_task(name: &str, base: ToPixmapTaskSpec) -> FileOutputTaskSpec {
    FileOutputTaskSpec::PngOutput {base, destination_name: name.to_string() }
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

impl Mul<f32> for ToAlphaChannelTaskSpec {
    type Output = ToAlphaChannelTaskSpec;

    fn mul(self, rhs: f32) -> Self::Output {
        if rhs == 1.0 {
            self
        } else {
            ToAlphaChannelTaskSpec::MakeSemitransparent {
                base: Box::new(self),
                alpha: (rhs * 255.0 + 0.5) as u8
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
                base: Box::new(ToAlphaChannelTaskSpec::FromPixmap { base: self }), color: rhs
            }
        }
    }
}
