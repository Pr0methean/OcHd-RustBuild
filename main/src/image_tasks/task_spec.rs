use std::collections::{HashMap};

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

use std::ops::{Deref, DerefMut, Mul};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use cached::lazy_static::lazy_static;
use crate::anyhoo;
use fn_graph::daggy::Dag;
use include_dir::{Dir, include_dir};
use itertools::{Itertools};


use log::{error, info};
use ordered_float::OrderedFloat;
use petgraph::graph::{IndexType, NodeIndex};
use replace_with::replace_with_and_return;

use resvg::tiny_skia::{Color, Mask, Pixmap};

use crate::image_tasks::animate::animate;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::from_svg::{COLOR_SVGS, from_svg};
use crate::image_tasks::make_semitransparent::make_semitransparent;
use crate::image_tasks::MaybeFromPool;
use crate::image_tasks::png_output::{copy_out_to_out, png_output};
use crate::image_tasks::repaint::{paint, pixmap_to_mask};
use crate::image_tasks::stack::{stack_alpha_on_alpha, stack_alpha_on_background, stack_layer_on_background, stack_layer_on_layer};
use crate::TILE_SIZE;

pub trait TaskSpecTraits <T>: Clone + Debug + Display + Ord + Eq + Hash {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<T>)
        where Ix : IndexType, E: Default, 'b: 'a;
}

impl TaskSpecTraits<MaybeFromPool<Pixmap>> for ToPixmapTaskSpec {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<MaybeFromPool<Pixmap>>)
                         where Ix : IndexType, E: Default, 'b: 'a {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future)) = ctx.pixmap_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let (dependencies, function): (Vec<NodeIndex<Ix>>, LazyTaskFunction<MaybeFromPool<Pixmap>>) = match self {
            ToPixmapTaskSpec::None { .. } => panic!("Tried to add None task to graph"),
            ToPixmapTaskSpec::Animate { background, frames } => {
                let background_opaque = background.is_necessarily_opaque();
                let (background_index, background_future) = background.add_to(ctx);
                let mut dependencies = Vec::with_capacity(frames.len() + 1);
                dependencies.push(background_index);
                let mut frame_futures: Vec<CloneableLazyTask<MaybeFromPool<Pixmap>>>
                    = Vec::with_capacity(frames.len());
                for frame in frames {
                    let (frame_index, frame_future) = frame.add_to(ctx);
                    frame_futures.push(frame_future);
                    dependencies.push(frame_index);
                }
                (dependencies, Box::new(move || {
                    let background: Arc<Box<MaybeFromPool<Pixmap>>> = background_future.into_result()?;
                    animate(&background, frame_futures, !background_opaque)
                }))
            },
            ToPixmapTaskSpec::FromSvg { source } => {
                let source = source.to_owned();
                (vec![], Box::new(move || {
                    Ok(Box::new(from_svg(&source, *TILE_SIZE)?))
                }))
            },
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } => {
                let background: Color = (*background).into();
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![fg_index],
                Box::new(move || {
                    let fg_image: Arc<Box<MaybeFromPool<Pixmap>>> = fg_future.into_result()?;
                    let mut fg_image = Arc::unwrap_or_clone(fg_image);
                    stack_layer_on_background(background, &mut fg_image)?;
                    Ok(fg_image)
                }))
            },
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => {
                if let ToPixmapTaskSpec::PaintAlphaChannel {base: base_of_bg, color: color_of_bg} = background.deref()
                        && let ToPixmapTaskSpec::PaintAlphaChannel {base: base_of_fg, color: color_of_fg} = foreground.deref()
                        && color_of_bg == color_of_fg {
                    error!("Wanted to rebuild {} by merging {} and {}, but the borrow checker \
                    doesn't allow this!", self, base_of_bg, base_of_fg);
                    /*
                    FIXME: Fails borrow checker:
                    let simplified = ToPixmapTaskSpec::PaintAlphaChannel {
                        base: Box::new(ToAlphaChannelTaskSpec::StackAlphaOnAlpha {
                            background: base_of_bg.to_owned(),
                            foreground: base_of_fg.to_owned()
                        }),
                        color: color_of_fg.to_owned()
                    };
                    return simplified.add_to(ctx);
                     */
                }
                let (bg_index, bg_future) = background.add_to(ctx);
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![bg_index, fg_index], Box::new(move || {
                    let bg_image: Arc<Box<MaybeFromPool<Pixmap>>> = bg_future.into_result()?;
                    let mut out_image = Arc::unwrap_or_clone(bg_image);
                    let fg_image: Arc<Box<MaybeFromPool<Pixmap>>> = fg_future.into_result()?;
                    stack_layer_on_layer(&mut out_image, fg_image.deref());
                    Ok(out_image)
                }))
            },
            ToPixmapTaskSpec::PaintAlphaChannel { base, color } => {
                let (base_index, base_future) = base.add_to(ctx);
                let color: Color = (*color).into();
                (vec![base_index],
                Box::new(move || {
                    let base_image: Arc<Box<MaybeFromPool<Mask>>> = base_future.into_result()?;
                    paint(Arc::unwrap_or_clone(base_image).as_ref(), color)
                }))
            },
        };
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        info!("Adding node: {}", name);
        let task = CloneableLazyTask::new(name, function);
        ctx.pixmap_task_to_future_map.insert(self, (self_id, task.to_owned()));
        (self_id, task)
    }
}

impl TaskSpecTraits<MaybeFromPool<Mask>> for ToAlphaChannelTaskSpec {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<MaybeFromPool<Mask>>)
                         where Ix : IndexType, E: Default, 'b: 'a {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future))
                = ctx.alpha_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let (dependencies, function): (Vec<NodeIndex<Ix>>, LazyTaskFunction<MaybeFromPool<Mask>>)
                = match self {
            ToAlphaChannelTaskSpec::MakeSemitransparent { base, alpha } => {
                let alpha: f32 = (*alpha).into();
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index],
                Box::new(move || {
                    let base_result: Arc<Box<MaybeFromPool<Mask>>> = base_future.into_result()?;
                    let mut channel = Arc::unwrap_or_clone(base_result);
                    make_semitransparent(&mut channel, alpha);
                    Ok(channel)
                }))
            },
            ToAlphaChannelTaskSpec::FromPixmap { base } => {
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index],
                Box::new(move || {
                    let base_image: Arc<Box<MaybeFromPool<Pixmap>>> = base_future.into_result()?;
                    Ok(Box::new(pixmap_to_mask(&base_image)))
                }))
            },
            ToAlphaChannelTaskSpec::StackAlphaOnAlpha { background, foreground } => {
                let (bg_index, bg_future) = background.add_to(ctx);
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![bg_index, fg_index], Box::new(move || {
                    let bg_mask: Arc<Box<MaybeFromPool<Mask>>> = bg_future.into_result()?;
                    let mut out_mask = Arc::unwrap_or_clone(bg_mask);
                    let fg_mask: Arc<Box<MaybeFromPool<Mask>>> = fg_future.into_result()?;
                    stack_alpha_on_alpha(&mut out_mask, fg_mask.deref());
                    Ok(out_mask)
                }))
            },
            ToAlphaChannelTaskSpec::StackAlphaOnBackground { background, foreground } => {
                let background = background.0;
                let (fg_index, fg_future) = foreground.add_to(ctx);
                (vec![fg_index],
                 Box::new(move || {
                     let fg_arc: Arc<Box<MaybeFromPool<Mask>>> = fg_future.into_result()?;
                     let mut fg_image = Arc::unwrap_or_clone(fg_arc);
                     stack_alpha_on_background(background, &mut fg_image);
                     Ok(fg_image)
                 }))
            }
        };
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        info!("Adding node: {}", name);
        let task = CloneableLazyTask::new(name, function);
        ctx.alpha_task_to_future_map.insert(self, (self_id, task.to_owned()));
        (self_id, task)
    }
}

impl TaskSpecTraits<()> for FileOutputTaskSpec {
    fn add_to<'a, 'b, E, Ix>(&'b self, ctx: &mut TaskGraphBuildingContext<'a, E, Ix>)
                         -> (NodeIndex<Ix>, CloneableLazyTask<()>)
                         where Ix : IndexType, E: Default, 'b: 'a {
        let name: String = self.to_string();
        if let Some((existing_index, existing_future))
                = ctx.output_task_to_future_map.get(&self) {
            info!("Matched an existing node: {}", name);
            return (*existing_index, existing_future.to_owned());
        }
        let (dependencies, function): (Vec<NodeIndex<Ix>>, LazyTaskFunction<()>) = match self {
            FileOutputTaskSpec::PngOutput {base, destination } => {
                let destination = destination.to_owned();
                let (base_index, base_future) = base.add_to(ctx);
                (vec![base_index], Box::new(move || {
                    Ok(Box::new(png_output(*Arc::unwrap_or_clone(base_future.into_result()?),
                                           &destination)?))
                }))
            }
            FileOutputTaskSpec::Copy {original, link} => {
                let link = link.to_owned();
                let original_path = original.get_path();
                let (base_index, base_future) = original.add_to(ctx);
                (vec![base_index], Box::new(move || {
                    base_future.into_result()?;
                    Ok(Box::new(copy_out_to_out(&original_path, &link)?))
                }))
            }
        };
        let self_id = ctx.graph.add_node(TaskSpec::from(self));
        for dependency in dependencies {
            ctx.graph.add_edge(dependency, self_id, E::default())
                .expect("Tried to create a cycle");
        }
        info!("Adding node: {}", name);
        let wrapped_future = CloneableLazyTask::new(name, function);
        ctx.output_task_to_future_map.insert(self, (self_id, wrapped_future.to_owned()));
        (self_id, wrapped_future)
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
    name: String,
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
        let state = self.state;
        let result = Arc::try_unwrap(state);
        match result {
            Ok(mutex) => match mutex.into_inner() {
                Ok(state) => {
                    // We're the last referent to this Lazy, so we don't need to clone anything.
                    match state {
                        CloneableLazyTaskState::Upcoming { function } => {
                            info!("Starting task {}", self.name);
                            let result = function().map(Arc::new);
                            info!("Finished task {}", self.name);
                            result
                        },
                        CloneableLazyTaskState::Finished { result } => {
                            result
                        },
                    }
                },
                Err(e) => Err(e.into())
            },
            Err(arc) => {
                // We're not the last referent to this Lazy, so we need to make at least a shallow
                // copy, which will become deep (via Arc::clone_or_unwrap) if it needs to be
                // mutable.
                let lock_result = arc.lock();
                match lock_result {
                    Ok(mut guard) => {
                        if let CloneableLazyTaskState::Finished {result} = guard.deref() {
                            return result.to_owned();
                        }
                        replace_with_and_return(
                            guard.deref_mut(),
                            || CloneableLazyTaskState::Finished {result: Err(anyhoo!("replace_with failed")) },
                            |state| -> (CloneableResult<T>, CloneableLazyTaskState<T>) {
                                let result = match state {
                            CloneableLazyTaskState::Upcoming { function } => {
                                info!("Starting task {}", self.name);
                                let result = function().map(Arc::new);
                                info!("Finished task {}", self.name);
                                result
                            },
                            CloneableLazyTaskState::Finished { result } => {
                                result
                            }
                        };
                        (result.to_owned(), CloneableLazyTaskState::Finished { result })
                    }
                )},
                    Err(e) => Err(e.into())
                }
            }
        }
    }
}

impl ToPixmapTaskSpec {
    /// Used in [TaskSpec::add_to] to deduplicate certain tasks that are redundant.
    fn is_all_black(&self) -> bool {
        match self {
            ToPixmapTaskSpec::None { .. } => panic!("is_all_black() called on None task"),
            ToPixmapTaskSpec::Animate { background, frames } =>
                background.is_all_black() && frames.iter().all(|frame| frame.is_all_black()),
            ToPixmapTaskSpec::FromSvg { source } => !(COLOR_SVGS.contains(&&*source.to_string_lossy())),
            ToPixmapTaskSpec::PaintAlphaChannel { color, .. } => color.is_black_or_transparent(),
            ToPixmapTaskSpec::StackLayerOnColor { background, foreground } =>
                background.is_black_or_transparent() && foreground.is_all_black(),
            ToPixmapTaskSpec::StackLayerOnLayer { background, foreground } => background.is_all_black() && foreground.is_all_black(),
        }
    }

    fn is_necessarily_opaque(&self) -> bool {
        match self {
            ToPixmapTaskSpec::Animate { background, .. }
                => background.is_necessarily_opaque(),
            ToPixmapTaskSpec::FromSvg { .. } => false,
            ToPixmapTaskSpec::PaintAlphaChannel { .. } => false,
            ToPixmapTaskSpec::StackLayerOnColor { background, .. } => background.alpha() == u8::MAX,
            ToPixmapTaskSpec::StackLayerOnLayer { background, .. } => background.is_necessarily_opaque(),
            ToPixmapTaskSpec::None { .. } => panic!("is_necessarily_opaque() called on None task"),
        }
    }
}

impl From<ToPixmapTaskSpec> for ToAlphaChannelTaskSpec {
    fn from(value: ToPixmapTaskSpec) -> Self {
        ToAlphaChannelTaskSpec::FromPixmap {base: Box::new(value)}
    }
}

pub type TaskGraph<E, Ix> = Dag<TaskSpec, E, Ix>;
pub struct TaskGraphBuildingContext<'a, E, Ix> where Ix: IndexType {
    pub graph: TaskGraph<E, Ix>,
    pixmap_task_to_future_map: HashMap<&'a ToPixmapTaskSpec, (NodeIndex<Ix>, CloneableLazyTask<MaybeFromPool<Pixmap>>)>,
    alpha_task_to_future_map: HashMap<&'a ToAlphaChannelTaskSpec, (NodeIndex<Ix>, CloneableLazyTask<MaybeFromPool<Mask>>)>,
    pub output_task_to_future_map: HashMap<&'a FileOutputTaskSpec, (NodeIndex<Ix>, CloneableLazyTask<()>)>
}

impl <'a,E,Ix> TaskGraphBuildingContext<'a,E,Ix> where Ix: IndexType {
    pub(crate) fn new() -> Self {
        TaskGraphBuildingContext {
            graph: TaskGraph::new(),
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
    if foreground.is_necessarily_opaque() {
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
