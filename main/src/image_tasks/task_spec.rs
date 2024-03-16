use std::cmp::Ordering;
use std::collections::HashMap;

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

use std::ops::{Deref, Mul};
use std::sync::{Arc};
use BitDepth::Sixteen;
use ColorType::GrayscaleAlpha;

use crate::{debug_assert_unreachable, GRID_SIZE, TILE_SIZE};
use include_dir::{Dir, include_dir};
use itertools::{Either, Itertools};
use itertools::Either::{Left, Right};

use log::info;
use oxipng::{BitDepth, RGB16, RGBA8};
use oxipng::BitDepth::{Eight, Four, One, Two};
use oxipng::ColorType;
use oxipng::ColorType::{Grayscale, Indexed, RGB, RGBA};

use resvg::tiny_skia::{Color, Mask, Pixmap};

use crate::image_tasks::animate::animate;
use crate::image_tasks::cloneable::{ArcowStr, CloneableError, CloneableLazyTask, LazyTaskFunction};
use crate::image_tasks::color::{BIT_DEPTH_FOR_CHANNEL, ComparableColor, gray};
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg, SEMITRANSPARENCY_FREE_SVGS};
use crate::image_tasks::make_semitransparent::{ALPHA_MULTIPLICATION_TABLE, ALPHA_STACKING_TABLE, make_semitransparent};
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::png_output::{copy_out_to_out, png_output};
use crate::image_tasks::repaint::{paint, pixmap_to_mask};
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_alpha_on_background, stack_layer_on_background, stack_layer_on_layer};
use crate::image_tasks::task_spec::ColorDescription::{Rgb, SpecifiedColors};
use crate::image_tasks::task_spec::ToAlphaChannelTaskSpec::StackAlphaOnAlpha;
use crate::image_tasks::task_spec::ToPixmapTaskSpec::UpscaleFromGridSize;
use crate::image_tasks::task_spec::Transparency::{AlphaChannel, BinaryTransparency, Opaque};
use crate::image_tasks::upscale::{upscale_image, upscale_mask};

pub trait TaskSpecTraits <T>: Clone + Debug + Display + Ord + Eq + Hash {
    fn add_to(&self, ctx: &mut TaskGraphBuildingContext, tile_size: u32)
                         -> CloneableLazyTask<T>;
}

impl TaskSpecTraits<MaybeFromPool<Pixmap>> for ToPixmapTaskSpec {
    fn add_to(&self, ctx: &mut TaskGraphBuildingContext, tile_size: u32)
                      -> CloneableLazyTask<MaybeFromPool<Pixmap>> {
        let name = self.to_string();
        if let Some(existing_future) = ctx.get_pixmap_future(tile_size, self) {
            info!("Matched an existing node: {}", name);
            return existing_future.to_owned();
        }
        if let UpscaleFromGridSize {..} = self {
            // Fall through; let expressions can't be inverted
        } else if tile_size != GRID_SIZE && self.is_grid_perfect(ctx) {
            return UpscaleFromGridSize {base: self.to_owned().into()}
                .add_to(ctx, tile_size);
        }
        let function: LazyTaskFunction<MaybeFromPool<Pixmap>> = match self {
            ToPixmapTaskSpec::None { .. } => debug_assert_unreachable("Tried to add None task to graph"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let background_future = background.add_to(ctx, tile_size);
                let background_color_desc_future = background.get_color_description_task(ctx);
                let frame_futures: Box<[CloneableLazyTask<MaybeFromPool<Pixmap>>]>
                    = frames.iter().map(|frame| frame.add_to(ctx, tile_size))
                    .collect();
                Box::new(move || {
                    let background_opaque = background_color_desc_future.into_result()?.transparency() == Opaque;
                    let background: Arc<MaybeFromPool<Pixmap>> = background_future.into_result()?;
                    animate(&background, frame_futures.to_vec(), !background_opaque)
                })
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source = source.to_string();
                Box::new(move || {
                    Ok(Box::new(from_svg(source, tile_size)?))
                })
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                let background: Color = (*background).into();
                let fg_future = foreground.add_to(ctx, tile_size);
                Box::new(move || {
                    let fg_image: Arc<MaybeFromPool<Pixmap>> = fg_future.into_result()?;
                    let mut fg_image = Arc::unwrap_or_clone(fg_image);
                    stack_layer_on_background(background, &mut fg_image)?;
                    Ok(fg_image.into())
                })
            },
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                let bg_future = background.add_to(ctx, tile_size);
                let fg_future = foreground.add_to(ctx, tile_size);
                Box::new(move || {
                    let bg_image: Arc<MaybeFromPool<Pixmap>> = bg_future.into_result()?;
                    let mut out_image = Arc::unwrap_or_clone(bg_image);
                    let fg_image: Arc<MaybeFromPool<Pixmap>> = fg_future.into_result()?;
                    stack_layer_on_layer(&mut out_image, fg_image.deref());
                    Ok(out_image.into())
                })
            },
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                let base_future = base.add_to(ctx, tile_size);
                let color = color.to_owned();
                Box::new(move || {
                    let base_image: Arc<MaybeFromPool<Mask>> = base_future.into_result()?;
                    paint(&*Arc::unwrap_or_clone(base_image), color).into()
                })
            },
            UpscaleFromGridSize { base } => {
                let base_future = base.add_to(ctx, GRID_SIZE);
                if tile_size == GRID_SIZE {
                    return base_future;
                }
                Box::new(move || {
                    let base_image = base_future.into_result()?;
                    Ok(Box::new(upscale_image(base_image.deref(), tile_size)?))
                })
            }
        };
        info!("Adding node: {}", name);
        let task = CloneableLazyTask::new(name, function);
        ctx.insert_pixmap_future(tile_size, self.to_owned(), task.to_owned());
        task
    }
}

impl TaskSpecTraits<MaybeFromPool<Mask>> for ToAlphaChannelTaskSpec {
    fn add_to(&self, ctx: &mut TaskGraphBuildingContext, tile_size: u32)
                         -> CloneableLazyTask<MaybeFromPool<Mask>> {
        let name: String = self.to_string();
        if let Some(existing_future) = ctx.get_alpha_future(tile_size, self) {
            info!("Matched an existing node: {}", name);
            return existing_future.to_owned();
        }
        if let ToAlphaChannelTaskSpec::UpscaleFromGridSize {..} = self {
            // Fall through; let expressions can't be inverted
        } else if tile_size != GRID_SIZE && self.is_grid_perfect(ctx) {
            return ToAlphaChannelTaskSpec::UpscaleFromGridSize {base: self.to_owned().into()}
                .add_to(ctx, tile_size);
        }
        let function: LazyTaskFunction<MaybeFromPool<Mask>> = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                let base_future = base.add_to(ctx, tile_size);
                let alpha = *alpha;
                Box::new(move || {
                    let base_result: Arc<MaybeFromPool<Mask>> = base_future.into_result()?;
                    let mut channel = Arc::unwrap_or_clone(base_result);
                    make_semitransparent(&mut channel, alpha);
                    Ok(channel.into())
                })
            },
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                let base_future = base.add_to(ctx, tile_size);
                Box::new(move || {
                    let base_image: Arc<MaybeFromPool<Pixmap>> = base_future.into_result()?;
                    Ok(pixmap_to_mask(&base_image).into())
                })
            },
            StackAlphaOnAlpha { background, foreground } => {
                let bg_future = background.add_to(ctx, tile_size);
                let fg_future = foreground.add_to(ctx, tile_size);
                Box::new(move || {
                    let bg_mask: Arc<MaybeFromPool<Mask>> = bg_future.into_result()?;
                    let mut out_mask = Arc::unwrap_or_clone(bg_mask);
                    let fg_mask: Arc<MaybeFromPool<Mask>> = fg_future.into_result()?;
                    stack_alpha_on_alpha(&mut out_mask, fg_mask.deref());
                    Ok(out_mask.into())
                })
            },
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background, foreground } => {
                let background = *background;
                let fg_future = foreground.add_to(ctx, tile_size);
                Box::new(move || {
                    let fg_arc: Arc<MaybeFromPool<Mask>> = fg_future.into_result()?;
                    let mut fg_image = Arc::unwrap_or_clone(fg_arc);
                    stack_alpha_on_background(background, &mut fg_image);
                    Ok(fg_image.into())
                })
            },
            ToAlphaChannelTaskSpec::UpscaleFromGridSize {base} => {
                let base_future = base.add_to(ctx, GRID_SIZE);
                if tile_size == GRID_SIZE {
                    return base_future;
                }
                Box::new(move || {
                    let base_mask = base_future.into_result()?;
                    Ok(upscale_mask(base_mask.deref(), tile_size)?.into())
                })
            }
        };
        info!("Adding node: {}", name);
        let task = CloneableLazyTask::new(name, function);
        ctx.insert_alpha_future(tile_size, self.to_owned(), task.to_owned());
        task
    }
}

impl TaskSpecTraits<()> for FileOutputTaskSpec {
    fn add_to(&self, ctx: &mut TaskGraphBuildingContext, tile_size: u32)
                         -> CloneableLazyTask<()> {
        let name: String = self.to_string();
        if let Some(existing_future)
                = ctx.output_task_to_future_map.get(self) {
            info!("Matched an existing node: {}", name);
            return existing_future.to_owned();
        }
        let function: LazyTaskFunction<()> = match self {
            FileOutputTaskSpec::PngOutput {base, .. } => {
                let destination_path = self.get_path();
                let base_size = if base.is_grid_perfect(ctx) {
                    GRID_SIZE
                } else {
                    tile_size
                };
                let base_future = base.add_to(ctx, base_size);
                let base_color_desc_future = base.get_color_description_task(ctx);
                let name = name.to_owned();
                Box::new(move || {
                    let (color_type, bit_depth) = color_description_to_mode(
                        &*base_color_desc_future.into_result()?, &name);
                    let base_result = base_future.into_result()?;
                    Ok(Box::new(png_output(Arc::unwrap_or_clone(base_result),
                                           color_type, bit_depth, destination_path)?))
                })
            }
            FileOutputTaskSpec::Copy {original, ..} => {
                let link = self.get_path();
                let original_path = original.get_path();
                let base_future = original.add_to(ctx, tile_size);
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

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TileSized<T> {
    inner: T,
    tile_size: u32
}

impl <T> Display for TileSized<T> where T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)?;
        f.write_fmt(format_args!(" (size {})", self.tile_size))
    }
}

/// [TaskSpec] for a task that produces a [Pixmap].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToPixmapTaskSpec {
    Animate {background: Box<ToPixmapTaskSpec>, frames: Box<[ToPixmapTaskSpec]>},
    FromSvg {source: ArcowStr<'static>},
    PaintAlphaChannel {base: Box<ToAlphaChannelTaskSpec>, color: ComparableColor},
    StackLayerOnColor {background: ComparableColor, foreground: Box<ToPixmapTaskSpec>},
    StackLayerOnLayer {background: Box<ToPixmapTaskSpec>, foreground: Box<ToPixmapTaskSpec>},
    UpscaleFromGridSize {base: Box<ToPixmapTaskSpec>},
    None,
}

/// [TaskSpec] for a task that produces an [AlphaChannel].
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ToAlphaChannelTaskSpec {
    MakeSemitransparent {base: Box<ToAlphaChannelTaskSpec>, alpha: u8},
    FromPixmap {base: ToPixmapTaskSpec},
    StackAlphaOnAlpha {background: Box<ToAlphaChannelTaskSpec>, foreground: Box<ToAlphaChannelTaskSpec>},
    StackAlphaOnBackground {background: u8, foreground: Box<ToAlphaChannelTaskSpec>},
    UpscaleFromGridSize {base: Box<ToAlphaChannelTaskSpec>}
}

/// [TaskSpec] for a task that doesn't produce a heap object as output.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FileOutputTaskSpec {
    PngOutput {base: ToPixmapTaskSpec, destination_name: ArcowStr<'static>},
    Copy {original: Box<FileOutputTaskSpec>, link_name: ArcowStr<'static>}
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

#[macro_export]
macro_rules! anyhoo {
    ($($args:expr),+ $(,)?) => {
        $crate::image_tasks::cloneable::CloneableError::from(anyhow::anyhow!($($args),+))
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
            UpscaleFromGridSize {base} => {
                write!(f, "upscale({})", base)
            }
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
            StackAlphaOnAlpha {background, foreground} => {
                write!(f, "({}+{})", background, foreground)
            }
            ToAlphaChannelTaskSpec::StackAlphaOnBackground {background, foreground} => {
                write!(f, "({}+{})", background, foreground)
            }
            ToAlphaChannelTaskSpec::UpscaleFromGridSize {base} => {
                write!(f, "upscale({})", base)
            }
        }
    }
}

fn stack_alpha_vecs(background: &[u8], foreground: &[u8]) -> Vec<u8> {
    let mut combined: Vec<u8> = background.iter().flat_map(|bg_alpha|
        foreground.iter().map(move |fg_alpha| {
            ALPHA_STACKING_TABLE[*bg_alpha as usize][*fg_alpha as usize]
        })).collect();
    combined.sort();
    combined.dedup();
    combined
}

fn multiply_alpha_vec(alphas: &[u8], rhs: u8) -> Vec<u8> {
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
    SpecifiedColors(Arc<[ComparableColor]>),
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

const BINARY_SEARCH_THRESHOLD: usize = u8::MAX as usize + 1;

impl ColorDescription {
    pub fn transparency(&self) -> Transparency {
        match self {
            SpecifiedColors(colors) => {
                if contains_semitransparency(colors) {
                    AlphaChannel
                } else if colors[0].alpha() == 0 {
                    BinaryTransparency
                } else {
                    Opaque
                }
            },
            Rgb(transparency) => *transparency
        }
    }

    pub fn put_adjacent(&self, neighbor: &ColorDescription) -> ColorDescription {
        match neighbor {
            Rgb(transparency) => Rgb(self.transparency().put_adjacent(transparency)),
            SpecifiedColors(neighbor_colors) => {
                match self {
                    Rgb(transparency) => Rgb(transparency.put_adjacent(&neighbor.transparency())),
                    SpecifiedColors(self_colors) => {
                        let mut combined_colors = (**self_colors).to_vec();
                        combined_colors.extend(neighbor_colors.iter());
                        combined_colors.sort();
                        combined_colors.dedup();
                        SpecifiedColors(combined_colors.into())
                    }
                }
            }
        }
    }

    pub fn stack_on(&self, background: &ColorDescription, max_colors: usize) -> ColorDescription {
        match background {
            Rgb(transparency) => Rgb(self.transparency().stack_on(transparency)),
            SpecifiedColors(bg_colors) => {
                match &self {
                    Rgb(transparency) => Rgb(transparency.stack_on(&background.transparency())),
                    SpecifiedColors(fg_colors) => {
                        match self.transparency() {
                            Opaque => SpecifiedColors(fg_colors.clone()),
                            BinaryTransparency => {
                                let mut combined_colors = (**bg_colors).to_vec();
                                combined_colors.extend(fg_colors.iter().filter(
                                    |color| color.alpha() == u8::MAX
                                ));
                                combined_colors.sort();
                                combined_colors.dedup();
                                combined_colors.truncate(max_colors);
                                SpecifiedColors(combined_colors.into())
                            }
                            AlphaChannel => {
                                // Using dedup() rather than unique() uses too much memory
                                let mut combined_colors: Vec<ComparableColor> = bg_colors.iter().copied().flat_map(|bg_color|
                                    bg_color.under(fg_colors.iter().copied()).into_iter()
                                ).unique().take(max_colors).collect();
                                combined_colors.sort();
                                SpecifiedColors(combined_colors.into())
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Transparency {
    Opaque,
    BinaryTransparency,
    AlphaChannel
}

fn palette_bit_depth(len: usize) -> BitDepth {
    if len <= 2 {
        One
    } else if len <= 4 {
        Two
    } else if len <= 16 {
        Four
    } else if len <= 256 {
        Eight
    } else {
        debug_assert_unreachable("Indexed mode with more than 256 colors not supported")
    }
}

pub const fn channel_to_bit_depth(input: u8, depth: BitDepth) -> u16 {
    match depth {
        One => if input < 0x80 { 0 } else { 1 },
        Two => {
            (input as u16 + (0x55/2)) / 0x55
        },
        Four => {
            (input as u16 + (0x11/2)) / 0x11
        },
        Eight => input as u16,
        Sixteen => debug_assert_unreachable("16-bit depth")
    }
}

const ALL_U8S: &[u8; u8::MAX as usize + 1] = &ALPHA_MULTIPLICATION_TABLE[u8::MAX as usize];

impl ToAlphaChannelTaskSpec {
    fn get_possible_alpha_values(&self, ctx: &mut TaskGraphBuildingContext) -> CloneableLazyTask<Vec<u8>> {
        if let Some(alpha_vec) = ctx.alpha_task_to_alpha_map.get(self) {
            return alpha_vec.to_owned();
        }
        let name = format!("Possible alpha values for {}", self);
        let alpha_vec: CloneableLazyTask<Vec<u8>> = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { alpha, base } => {
                let alpha = *alpha;
                let base_alphas_task = base.get_possible_alpha_values(ctx);
                CloneableLazyTask::new(name, Box::new(move ||
                    Ok(multiply_alpha_vec(&**base_alphas_task.into_result()?, alpha).into())))
            }
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                 base.get_possible_alpha_values(ctx)
            }
            StackAlphaOnAlpha { background, foreground } => {
                let bg_task = background.get_possible_alpha_values(ctx);
                let fg_task = foreground.get_possible_alpha_values(ctx);
                CloneableLazyTask::new(name, Box::new(move ||
                    Ok(stack_alpha_vecs(&bg_task.into_result()?, &fg_task.into_result()?).into())
                ))
            }
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background: background_alpha, foreground } => {
                let background_alpha = *background_alpha;
                let fg_task = foreground.get_possible_alpha_values(ctx);
                CloneableLazyTask::new(name, Box::new(move ||
                    Ok(stack_alpha_vecs(&[background_alpha], &fg_task.into_result()?).into())
                ))
            }
            ToAlphaChannelTaskSpec::UpscaleFromGridSize {base} => {
                base.get_possible_alpha_values(ctx)
            }
        };
        ctx.alpha_task_to_alpha_map.insert(self.to_owned(), alpha_vec.to_owned());
        alpha_vec
    }

    fn is_grid_perfect(&self, ctx: &mut TaskGraphBuildingContext) -> bool {
        match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, .. }
                => base.is_grid_perfect(ctx),
            ToAlphaChannelTaskSpec::FromPixmap { base }
                => base.is_grid_perfect(ctx),
            StackAlphaOnAlpha { background, foreground }
                => background.is_grid_perfect(ctx) && foreground.is_grid_perfect(ctx),
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { foreground, .. }
                => foreground.is_grid_perfect(ctx),
            ToAlphaChannelTaskSpec::UpscaleFromGridSize {..} => true
        }
    }
}

fn get_grayscale_bit_depth(colors: &[ComparableColor]) -> BitDepth {
    let mut grayscale_bit_depth = One;
    for color in colors.iter() {
        let color_bit_depth = BIT_DEPTH_FOR_CHANNEL[color.red() as usize];
        grayscale_bit_depth = grayscale_bit_depth.max(color_bit_depth);
        if grayscale_bit_depth == Eight {
            break;
        }
    }
    grayscale_bit_depth
}

fn color_description_to_mode(color_description: &ColorDescription,
                             task_name: &str)
    -> (ColorType, BitDepth) {
    match color_description {
        SpecifiedColors(colors) => {
            let transparency = color_description.transparency();
            info!("Task {} has {} possible colors and {:?} transparency",
                task_name, colors.len(), transparency);
            let have_non_gray = colors.iter().any(|color| !color.is_gray());
            let max_indexed_size = 256;
            if colors.len() > max_indexed_size {
                if have_non_gray {
                    info!("Using RGB mode for {}", task_name);
                    match transparency {
                        Opaque => (RGB {transparent_color: None}, Eight),
                        BinaryTransparency => (RGB {
                            transparent_color: Some(RGB16::new(0xc0c0,0xffff,0x3e3e))
                        }, Eight),
                        AlphaChannel => (RGBA, Eight)
                    }
                } else {
                    info!("Using grayscale+alpha mode for {}", task_name);
                    (GrayscaleAlpha, Eight)
                }
            } else if colors.len() > 16 && !have_non_gray && transparency == Opaque {
                info!("Using opaque grayscale for {}", task_name);
                (Grayscale {transparent_shade: None}, Eight)
            } else {
                let indexed_bit_depth = palette_bit_depth(colors.len());
                let indexed_mode = Indexed { palette:
                    colors.iter()
                        .map(|color| RGBA8::new(color.red(), color.green(), color.blue(), color.alpha()))
                        .collect()
                };
                if have_non_gray {
                    info!("Using indexed mode for {} because it has non-gray colors", task_name);
                    return (indexed_mode, indexed_bit_depth);
                }

                let (grayscale_mode, grayscale_bit_depth) = match transparency {
                    AlphaChannel => (GrayscaleAlpha, Eight),
                    BinaryTransparency => {
                        let grayscale_bit_depth = get_grayscale_bit_depth(colors);
                        let grayscale_shades = match grayscale_bit_depth {
                            One => vec![ComparableColor::BLACK, ComparableColor::WHITE],
                            Two => vec![gray(0x00), gray(0x55), gray(0xAA), gray(0xFF)],
                            Four => (0..16).map(|n| gray(n * 0x11)).collect(),
                            Eight => ALL_U8S.iter().copied().map(gray).collect(),
                            Sixteen => debug_assert_unreachable("16-bit depth")
                        };
                        match grayscale_shades.into_iter().find(|color| !colors.contains(color)) {
                            Some(unused) => (
                                Grayscale {transparent_shade: Some(unused.red() as u16 * 0x101)},
                                grayscale_bit_depth
                            ),
                            None => match grayscale_bit_depth {
                                One => (Grayscale {transparent_shade: Some(0x5555)}, Two),
                                Two => (Grayscale {transparent_shade: Some(0x2222)}, Four),
                                Four => (Grayscale {transparent_shade: Some(0x0808)}, Eight),
                                Eight => (GrayscaleAlpha, Eight),
                                Sixteen => debug_assert_unreachable("16-bit depth")
                            }
                        }
                    },
                    Opaque => (Grayscale {transparent_shade: None},
                               get_grayscale_bit_depth(colors))
                };
                let grayscale_bits_per_pixel = if grayscale_mode == GrayscaleAlpha {
                    2
                } else {
                    1
                } * grayscale_bit_depth as u8;
                let indexed_bits_per_pixel = indexed_bit_depth as u8;
                if grayscale_bits_per_pixel <= indexed_bits_per_pixel {
                    info!("Choosing grayscale mode for {} ({} vs {} bpp)", task_name,
                        grayscale_bits_per_pixel, indexed_bits_per_pixel);
                    (grayscale_mode, grayscale_bit_depth)
                } else {
                    info!("Choosing indexed mode for {} ({} vs {} bpp)", task_name,
                        indexed_bits_per_pixel, grayscale_bits_per_pixel);
                    (indexed_mode, indexed_bit_depth)
                }
            }
        },
        Rgb(Opaque) => (RGB {transparent_color: None}, Eight),
        Rgb(BinaryTransparency) => (RGB {
            transparent_color: Some(RGB16::new(0xc0c0,0xffff,0x3e3e))
        }, Eight),
        Rgb(AlphaChannel) => (RGBA, Eight)
    }
}

fn contains_alpha(vec: &[ComparableColor], needle_alpha: u8) -> bool {

    // Optimizations for the fact that fully-transparent can only appear once.
    let least_alpha = vec[0].alpha();
    if least_alpha == 0 {
        if needle_alpha == 0 {
            return true;
        }
        if vec.len() == 1 {
            return false;
        }
        let next_least_alpha = vec[1].alpha();
        debug_assert_ne!(0, next_least_alpha, "Transparent color included twice");
        match needle_alpha.cmp(&next_least_alpha) {
            Ordering::Less => return false,
            Ordering::Equal => return true,
            Ordering::Greater => {}
        }
    } else {
        match needle_alpha.cmp(&least_alpha) {
            Ordering::Less => return false,
            Ordering::Equal => return true,
            Ordering::Greater => {}
        }
    }

    // Check against upper limit of range
    let greatest_alpha = vec[vec.len() - 1].alpha();
    match needle_alpha.cmp(&greatest_alpha) {
        Ordering::Less => {}
        Ordering::Equal => return true,
        Ordering::Greater => return false,
    }

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

pub fn contains_semitransparency(vec: &[ComparableColor]) -> bool {
    match vec[0].alpha {
        0 => if vec.len() == 1 {
            false
        } else { match vec[1].alpha {
            0 => unreachable!(),
            u8::MAX => false,
            _ => true
        }
        }
        u8::MAX => false,
        _ => true,
    }
}

impl ToPixmapTaskSpec {

    /// If true, this texture has no gradients, diagonals or curves, so it can be rendered at a
    /// smaller size.
    pub(crate) fn is_grid_perfect(&self, ctx: &mut TaskGraphBuildingContext) -> bool {
        match self {
            ToPixmapTaskSpec::Animate { background, frames } =>
                background.is_grid_perfect(ctx) && frames.iter().all(|frame| frame.is_grid_perfect(ctx)),
            ToPixmapTaskSpec::FromSvg { source } => SEMITRANSPARENCY_FREE_SVGS.contains(&&**source)
                    && !COLOR_SVGS.contains(&&**source),
            ToPixmapTaskSpec::PaintAlphaChannel { base, .. } => base.is_grid_perfect(ctx),
            ToPixmapTaskSpec::StackLayerOnColor { foreground, .. } =>
                foreground.is_grid_perfect(ctx),
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } =>
                background.is_grid_perfect(ctx) && foreground.is_grid_perfect(ctx),
            UpscaleFromGridSize { .. } => true,
            ToPixmapTaskSpec::None => debug_assert_unreachable("ToPixmapTaskSpec::None::is_grid_perfect()")
        }
    }

    /// Used in [TaskSpec::add_to] to deduplicate certain tasks that are redundant.
    fn get_color_description_task(&self, ctx: &mut TaskGraphBuildingContext)
        -> CloneableLazyTask<ColorDescription> {
        if let Some(desc) = ctx.pixmap_task_to_color_map.get(self) {
            return (*desc).to_owned();
        }
        let name: ArcowStr<'static>
            = ArcowStr::from(format!("Color description for {}", self));
        let side_length = if *TILE_SIZE == GRID_SIZE || self.is_grid_perfect(ctx) {
            GRID_SIZE
        } else {
            *TILE_SIZE
        };
        let mut pixels = side_length as usize * side_length as usize;
        #[allow(clippy::type_complexity)]
        let desc: Either<Box<dyn FnOnce() -> Result<ColorDescription, CloneableError> + Send>,
                CloneableLazyTask<ColorDescription>> = match self {
            ToPixmapTaskSpec::None => debug_assert_unreachable("ToPixmapTaskSpec::None::get_color_description_task()"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                pixels *= frames.len();
                let background_desc_task = background.get_color_description_task(ctx);
                let frame_desc_tasks: Box<[_]> = (*frames)
                    .iter()
                    .map(|frame| frame.get_color_description_task(ctx))
                    .collect();
                Left(Box::new(move || {
                    let mut current_desc: Option<ColorDescription> = None;
                    let background_desc = background_desc_task.into_result()?;
                    for frame_desc_task in frame_desc_tasks.into_vec() {
                        let frame_desc
                            = frame_desc_task.into_result()?.stack_on(&background_desc, pixels + 1);
                        current_desc = Some(match current_desc {
                            None => frame_desc,
                            Some(other_frames_desc) => frame_desc.put_adjacent(&other_frames_desc)
                        });
                    }
                    Ok(current_desc.unwrap())
                }))
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let name = name.clone();
                if COLOR_SVGS.contains(&&**source) {
                    if SEMITRANSPARENCY_FREE_SVGS.contains(&&**source) {
                        Right(CloneableLazyTask::new_immediate_ok(name, Rgb(BinaryTransparency).into()))
                    } else {
                        Right(CloneableLazyTask::new_immediate_ok(name, Rgb(AlphaChannel).into()))
                    }
                } else if SEMITRANSPARENCY_FREE_SVGS.contains(&&**source) {
                    Right(CloneableLazyTask::new_immediate_ok(name,
                      SpecifiedColors(Arc::new([ComparableColor::TRANSPARENT, ComparableColor::BLACK])).into()))
                } else {
                    Right(CloneableLazyTask::new_immediate_ok(name,
                        SpecifiedColors(ALL_U8S.iter()
                        .map(|alpha| ComparableColor { red: 0, green: 0, blue: 0, alpha: *alpha}).collect()).into()))
                }
            },
            ToPixmapTaskSpec::PaintAlphaChannel { color, base } => {
                let base_task = base.get_possible_alpha_values(ctx);
                let color = *color;
                Left(Box::new(move || Ok(SpecifiedColors({
                    let alpha_array = ALPHA_MULTIPLICATION_TABLE[color.alpha() as usize];
                    let mut colored_alphas: Vec<ComparableColor> = base_task.into_result()?
                        .iter()
                        .copied()
                        .map(|alpha| ComparableColor {
                            red: color.red(),
                            green: color.green(),
                            blue: color.blue(),
                            alpha: alpha_array[alpha as usize]
                        })
                        .collect();
                    colored_alphas.sort();
                    colored_alphas.dedup();
                    colored_alphas.into()
                }))))
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                let background = *background;
                let fg_task = foreground.get_color_description_task(ctx);
                Left(Box::new(move || Ok(
                    fg_task.into_result()?.stack_on(&SpecifiedColors(Arc::new([background])), pixels + 1)
                )))
            }
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                let bg_task = background.get_color_description_task(ctx);
                let fg_task = foreground.get_color_description_task(ctx);
                Left(Box::new(move ||
                    Ok(fg_task.into_result()?.stack_on(&*bg_task.into_result()?, pixels + 1))
                ))
            }
            UpscaleFromGridSize {base} => Right(base.get_color_description_task(ctx))
        };
        let task = match desc {
            Left(function) => {
                let image_task = self.add_to(ctx, side_length);
                let pixels = pixels;
                let name_inner = name.clone();
                CloneableLazyTask::new(name.clone(), Box::new(move ||{
                    let uncapped = function()?;
                    Ok(Box::new(match uncapped {
                        SpecifiedColors(colors) => {
                            if colors.len() > pixels {
                                info!("For {}, possible colors exceed pixel count {}; rendering to determine \
                actual color count", name_inner, pixels);
                                let actual_image = image_task.into_result()?;
                                let mut actual_colors: Vec<ComparableColor> = actual_image.pixels().iter().copied()
                                    .map(ComparableColor::from)
                                    .collect();
                                actual_colors.sort();
                                actual_colors.dedup();
                                SpecifiedColors(actual_colors.into())
                            } else {
                                SpecifiedColors(colors)
                            }
                        }
                        Rgb(transparency) => Rgb(transparency)
                    }))}))
            }
            Right(task) => task
        };
        ctx.pixmap_task_to_color_map.insert(self.to_owned(), task.to_owned());
        task
    }

    fn get_possible_alpha_values(&self, ctx: &mut TaskGraphBuildingContext) -> CloneableLazyTask<Vec<u8>> {
        if let Some(alphas) = ctx.pixmap_task_to_alpha_map.get(self) {
            alphas.to_owned()
        } else {
            let color_task = self.get_color_description_task(ctx);
            let task = CloneableLazyTask::new(self.to_string(), Box::new(move || Ok(Box::new({
                let colors = color_task.into_result()?;
                match colors.transparency() {
                    AlphaChannel => match &*colors {
                        Rgb(_) => ALL_U8S.to_vec(),
                        SpecifiedColors(colors) => if colors.len() <= BINARY_SEARCH_THRESHOLD {
                            let mut alphas: Vec<u8> = colors.iter().map(|color| color.alpha()).collect();
                            alphas.sort();
                            alphas.dedup();
                            alphas
                        } else {
                            ALL_U8S.iter().copied()
                                .filter(|alpha| contains_alpha(&colors, *alpha))
                                .collect()
                        }
                    },
                    BinaryTransparency => vec![0, u8::MAX],
                    Opaque => vec![u8::MAX]
                }
            }))));
            ctx.pixmap_task_to_alpha_map.insert(self.to_owned(), task.to_owned());
            task
        }
    }

    pub fn alpha_and_color(&self) -> Option<(ToAlphaChannelTaskSpec, ComparableColor)> {
        match self {
            ToPixmapTaskSpec::Animate { .. } => None,
            ToPixmapTaskSpec::FromSvg { source } => {
                if COLOR_SVGS.contains(&&**source) {
                    None
                } else {
                    Some((ToAlphaChannelTaskSpec::FromPixmap {base: self.to_owned()}, ComparableColor::BLACK))
                }
            }
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } =>
                Some((*base.to_owned(), *color)),
            ToPixmapTaskSpec::StackLayerOnColor { .. } => None,
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                if let Some((bg_alpha, bg_color)) = background.alpha_and_color()
                    && let Some((fg_alpha, fg_color)) = foreground.alpha_and_color()
                    && bg_color == fg_color {
                    Some((StackAlphaOnAlpha {background: bg_alpha.into(), foreground: fg_alpha.into()}, bg_color))
                } else {
                    None
                }
            }
            UpscaleFromGridSize { base } => {
                if let Some((base_alpha, base_color)) = base.alpha_and_color() {
                    Some((ToAlphaChannelTaskSpec::UpscaleFromGridSize { base: base_alpha.into() }, base_color))
                } else {
                    None
                }
            }
            ToPixmapTaskSpec::None => debug_assert_unreachable("ToPixmapTaskSpec::None::alpha_and_color()")
        }
    }
}

impl From<ToPixmapTaskSpec> for ToAlphaChannelTaskSpec {
    fn from(value: ToPixmapTaskSpec) -> Self {
        ToAlphaChannelTaskSpec::FromPixmap {base: value}
    }
}

pub struct TaskGraphBuildingContext {
    pixmap_task_to_future_map: HashMap<u32, HashMap<ToPixmapTaskSpec, CloneableLazyTask<MaybeFromPool<Pixmap>>>>,
    alpha_task_to_future_map: HashMap<u32, HashMap<ToAlphaChannelTaskSpec, CloneableLazyTask<MaybeFromPool<Mask>>>>,
    pub output_task_to_future_map: HashMap<FileOutputTaskSpec, CloneableLazyTask<()>>,
    pixmap_task_to_color_map: HashMap<ToPixmapTaskSpec, CloneableLazyTask<ColorDescription>>,
    alpha_task_to_alpha_map: HashMap<ToAlphaChannelTaskSpec, CloneableLazyTask<Vec<u8>>>,
    pixmap_task_to_alpha_map: HashMap<ToPixmapTaskSpec, CloneableLazyTask<Vec<u8>>>
}

impl TaskGraphBuildingContext {
    pub(crate) fn new() -> Self {
        TaskGraphBuildingContext {
            pixmap_task_to_future_map: HashMap::new(),
            alpha_task_to_future_map: HashMap::new(),
            output_task_to_future_map: HashMap::new(),
            pixmap_task_to_color_map: HashMap::new(),
            alpha_task_to_alpha_map: HashMap::new(),
            pixmap_task_to_alpha_map: HashMap::new()
        }
    }

    pub fn get_pixmap_future(&self, tile_size: u32, task: &ToPixmapTaskSpec)
        -> Option<&CloneableLazyTask<MaybeFromPool<Pixmap>>> {
        self.pixmap_task_to_future_map.get(&tile_size)?.get(task)
    }

    pub fn get_alpha_future(&self, tile_size: u32, task: &ToAlphaChannelTaskSpec)
                             -> Option<&CloneableLazyTask<MaybeFromPool<Mask>>> {
        self.alpha_task_to_future_map.get(&tile_size)?.get(task)
    }

    pub fn insert_pixmap_future(&mut self, tile_size: u32, task: ToPixmapTaskSpec,
                                value: CloneableLazyTask<MaybeFromPool<Pixmap>>) {
        match self.pixmap_task_to_future_map.get_mut(&tile_size) {
            Some(map_for_tile_size) => {
                map_for_tile_size.insert(task, value);
            }
            None => {
                let mut new_map = HashMap::new();
                new_map.insert(task, value);
                self.pixmap_task_to_future_map.insert(tile_size, new_map);
            }
        }
    }

    pub fn insert_alpha_future(&mut self, tile_size: u32, task: ToAlphaChannelTaskSpec,
                               value: CloneableLazyTask<MaybeFromPool<Mask>>) {
        match self.alpha_task_to_future_map.get_mut(&tile_size) {
            Some(map_for_tile_size) => {
                map_for_tile_size.insert(task, value);
            }
            None => {
                let mut new_map = HashMap::new();
                new_map.insert(task, value);
                self.alpha_task_to_future_map.insert(tile_size, new_map);
            }
        }
    }
}

pub const SVG_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/svg");
pub const METADATA_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/metadata");

pub const ASSET_DIR: &str = "assets/minecraft/textures/";

pub fn from_svg_task<T: Into<ArcowStr<'static>>>(name: T) -> ToPixmapTaskSpec {
    ToPixmapTaskSpec::FromSvg {source: name.into()}
}

pub fn svg_alpha_task<T: Into<ArcowStr<'static>>>(name: T) -> ToAlphaChannelTaskSpec {
    ToAlphaChannelTaskSpec::FromPixmap { base: from_svg_task(name) }
}


pub fn paint_task(base: ToAlphaChannelTaskSpec, color: ComparableColor) -> ToPixmapTaskSpec {
    if let ToAlphaChannelTaskSpec::FromPixmap {base: ref base_base} = base {
        match base_base {
            ToPixmapTaskSpec::FromSvg { ref source } => {
                if color == ComparableColor::BLACK
                    && !COLOR_SVGS.contains(&&**source) {
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

pub fn paint_svg_task<T: Into<ArcowStr<'static>> + Display>(name: T, color: ComparableColor) -> ToPixmapTaskSpec {
    let name = name.into();
    if color == ComparableColor::BLACK
        && COLOR_SVGS.binary_search(&name.as_ref()).is_err() {
        info!("Simplified {}@{} -> {}", name, color, name);
        from_svg_task(name)
    } else {
        ToPixmapTaskSpec::PaintAlphaChannel {
            base: Box::new(ToAlphaChannelTaskSpec::FromPixmap { base: from_svg_task(name) }),
            color
        }
    }
}

pub fn out_task<T: Into<ArcowStr<'static>>>(name: T, base: ToPixmapTaskSpec) -> FileOutputTaskSpec {
    FileOutputTaskSpec::PngOutput {base, destination_name: name.into() }
}

fn stack_alpha_presorted(mut layers: Vec<ToAlphaChannelTaskSpec>) -> ToAlphaChannelTaskSpec {
    match layers.len() {
        0 => debug_assert_unreachable("Attempt to create empty stack of alpha channels"),
        1 => layers[0].to_owned(),
        x => {
            let last = layers.remove(x - 1);
            StackAlphaOnAlpha {
                background: stack_alpha_presorted(layers).into(),
                foreground: Box::new(last)
            }
        }
    }
}

pub fn stack_alpha(mut layers: Vec<ToAlphaChannelTaskSpec>) -> ToAlphaChannelTaskSpec {
    let mut upscale_layers = Vec::with_capacity(layers.len());
    let mut non_upscale_layers = Vec::with_capacity(layers.len());
    while !layers.is_empty() {
        match layers.remove(layers.len() - 1) {
            StackAlphaOnAlpha { background, foreground } => {
                layers.push(*background);
                layers.push(*foreground);
            }
            ToAlphaChannelTaskSpec::UpscaleFromGridSize { base } => {
                upscale_layers.push(*base);
            }
            layer => non_upscale_layers.push(layer)
        }
    }
    non_upscale_layers.sort();
    if upscale_layers.is_empty() {
        stack_alpha_presorted(non_upscale_layers)
    } else {
        non_upscale_layers.push(ToAlphaChannelTaskSpec::UpscaleFromGridSize {
            base: stack_alpha(upscale_layers).into()
        });
        stack_alpha_presorted(non_upscale_layers)
    }
}

fn try_simplify_pair(background: ToPixmapTaskSpec, foreground: ToPixmapTaskSpec)
    -> Result<ToPixmapTaskSpec, (ToPixmapTaskSpec, ToPixmapTaskSpec)> {
    let background_desc = background.to_string();
    let foreground_desc = foreground.to_string();
    if let Some((bg_alpha, bg_color)) = background.alpha_and_color()
        && let Some((fg_alpha, fg_color)) = foreground.alpha_and_color()
        && bg_color == fg_color {
        let simplified = paint_task(stack_alpha(vec![bg_alpha, fg_alpha]), bg_color);
        info!("Simplified ({},{}) -> {}", background_desc, foreground_desc, simplified);
        Ok(simplified)
    } else if let UpscaleFromGridSize {base: bg_base} = &background
        && let UpscaleFromGridSize {base: fg_base} = &foreground {
        let simplified = UpscaleFromGridSize {
            base: stack(*bg_base.to_owned(), *fg_base.to_owned()).into()
        };
        info!("Simplified ({},{}) -> {}", background_desc, foreground_desc, simplified);
        Ok(simplified)
    } else {
        Err((background, foreground))
    }
}

pub fn stack(background: ToPixmapTaskSpec, foreground: ToPixmapTaskSpec) -> ToPixmapTaskSpec {
    match try_simplify_pair(background, foreground) {
        Ok(simplified) => simplified,
        Err((background, foreground)) => {
            if let ToPixmapTaskSpec::StackLayerOnLayer {background: fg_bg, foreground: fg_fg}
                = foreground {
                return match try_simplify_pair(background, *fg_bg) {
                    Ok(simplified) => stack(simplified, *fg_fg),
                    Err((background, fg_bg)) => {
                        ToPixmapTaskSpec::StackLayerOnLayer {
                            background: background.into(),
                            foreground: stack(fg_bg, *fg_fg).into()
                        }
                    }
                };
            }
            if let ToPixmapTaskSpec::StackLayerOnLayer {background: bg_bg, foreground: bg_fg}
                = background {
                return match try_simplify_pair(*bg_fg, foreground) {
                    Ok(simplified) => stack(*bg_bg, simplified),
                    Err((bg_fg, foreground)) => {
                        ToPixmapTaskSpec::StackLayerOnLayer {
                            background: stack(*bg_bg, bg_fg).into(),
                            foreground: foreground.into()
                        }
                    }
                };
            }
            ToPixmapTaskSpec::StackLayerOnLayer {
                background: Box::new(background), foreground: Box::new(foreground)
            }
        }
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
