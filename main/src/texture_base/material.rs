use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use crate::anyhoo;
use crate::image_tasks::cloneable::CloneableError;

use crate::image_tasks::color::{c, ComparableColor};
use crate::image_tasks::task_spec::{FileOutputTaskSpec, from_svg_task, out_task, paint_svg_task, ToPixmapTaskSpec};

/// Specification in DSL form of how one or more texture images are to be generated.
pub trait Material {
    /// Converts this specification to a number of [PngOutput] instances, each of which references
    /// another [TaskSpec] to generate the image it will output.
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]>;

    fn get_output_task_by_name(&self, name: &str) -> Result<FileOutputTaskSpec, CloneableError> {
        for output_task in self.get_output_tasks().to_vec() {
            if output_task.get_path().contains(name) {
                return Ok(output_task);
            }
        }
        Err(anyhoo!("No output task found with name {}", name))
    }
}

pub struct MaterialGroup {
    pub(crate) tasks: Arc<[FileOutputTaskSpec]>
}

impl Material for MaterialGroup {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        self.tasks.to_owned()
    }
}

/// Material with 3 associated colors.
pub trait TricolorMaterial: Material {
    fn color(&self) -> ComparableColor;
    fn shadow(&self) -> ComparableColor;
    fn highlight(&self) -> ComparableColor;
}

pub const DEFAULT_GROUP_SIZE: usize = 1024;

#[macro_export]
macro_rules! group {
    ($name:ident = $( $members:expr ),* ) => {
        pub static $name: once_cell::sync::Lazy<$crate::texture_base::material::MaterialGroup>
        = once_cell::sync::Lazy::new(|| {
            let mut tasks: Vec<$crate::image_tasks::task_spec::FileOutputTaskSpec>
                = Vec::with_capacity($crate::texture_base::material::DEFAULT_GROUP_SIZE);
            $({
                #![allow(unused)]
                use $crate::texture_base::material::Material;
                tasks.extend($members.get_output_tasks().iter().cloned());
            })*
            $crate::texture_base::material::MaterialGroup { tasks: tasks.into() }
        });
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureMaterial {
    pub name: &'static str,
    texture: ToPixmapTaskSpec
}

impl SingleTextureMaterial {
    pub fn texture(&self) -> ToPixmapTaskSpec {
        self.texture.to_owned()
    }
    pub const fn new(name: &'static str, texture: ToPixmapTaskSpec) -> Self {
        SingleTextureMaterial {name, texture}
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureTricolorMaterial {
    pub material: SingleTextureMaterial,
    pub colors: ColorTriad
}

impl Material for SingleTextureTricolorMaterial {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        self.material.get_output_tasks()
    }
}

impl TricolorMaterial for SingleTextureTricolorMaterial {
    fn color(&self) -> ComparableColor {
        self.colors.color
    }

    fn shadow(&self) -> ComparableColor {
        self.colors.shadow
    }

    fn highlight(&self) -> ComparableColor {
        self.colors.highlight
    }
}

impl From<SingleTextureMaterial> for ToPixmapTaskSpec {
    fn from(val: SingleTextureMaterial) -> Self {
        val.texture
    }
}

impl Material for SingleTextureMaterial {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        Arc::new([out_task(self.name, self.texture())])
    }
}

#[macro_export]
macro_rules! material {
    ($name:ident = $directory:expr, $texture:expr) => {
        pub static $name: once_cell::sync::Lazy<$crate::texture_base::material::SingleTextureMaterial>
            = once_cell::sync::Lazy::new(||
                    $crate::texture_base::material::SingleTextureMaterial::new(
                        const_format::concatcp!($directory, "/",
                            const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name))
                        ),
                        $texture.into()
            )
        );
    }
}

#[macro_export]
macro_rules! single_texture_material {
    ($name:ident = $directory:expr, $background:expr, $( $layers:expr ),* ) => {
        $crate::material!(
            $name = $directory, $crate::stack_on!($background, $($layers),*));
    }
}

#[macro_export]
macro_rules! single_layer_material {
    ($name:ident = $directory:expr, $layer_name:expr, $color:expr ) => {
        pub const $name: $crate::texture_base::material::SingleLayerMaterial =
            $crate::texture_base::material::SingleLayerMaterial {
            name: const_format::concatcp!(
                $directory, "/",
                const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            ),
            layer_name: $layer_name,
            color: Some($color)
        };
    };
    ($name:ident = $directory:expr, $layer_name:expr) => {
        pub const $name: $crate::texture_base::material::SingleLayerMaterial =
            $crate::texture_base::material::SingleLayerMaterial {
            name: const_format::concatcp!(
                $directory, "/",
                const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            ),
            layer_name: $layer_name,
            color: None
        };
    };
}

#[macro_export]
macro_rules! single_texture_item {
    ($name:ident = $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "item",
            $crate::image_tasks::color::ComparableColor::TRANSPARENT,
            $($layers),*);
    }
}

#[macro_export]
macro_rules! single_layer_item {
    ($name:ident = $($layer_name_and_maybe_color:expr),+ ) => {
        $crate::single_layer_material!($name = "item", $($layer_name_and_maybe_color),+);
    }
}

#[macro_export]
macro_rules! single_texture_block {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "block", $background, $($layers),*);
    }
}

#[macro_export]
macro_rules! single_layer_block {
    ($name:ident = $layer_name:expr, $color:expr ) => {
        $crate::single_layer_material!($name = "block", $layer_name, $color);
    };
    ($name:ident = $layer_name:expr) => {
        $crate::single_layer_material!($name = "block", $layer_name);
    };
}

#[macro_export]
macro_rules! single_texture_particle {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "particle", $background, $($layers),*);
    }
}

#[macro_export]
macro_rules! single_layer_particle {
    ($name:ident = $($layer_name_and_maybe_color:expr),+ ) => {
        $crate::single_layer_material!($name = "particle", $($layer_name_and_maybe_color),+);
    }
}

pub struct CopiedMaterial {
    pub name: &'static str,
    pub source: FileOutputTaskSpec
}

impl Material for CopiedMaterial {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        Arc::new([FileOutputTaskSpec::Copy {
            original: Box::new(self.source.to_owned()),
            link_name: self.name.into()
        }])
    }
}

#[macro_export]
macro_rules! copy_block {
    ($name:ident = $base:expr, $base_name:expr) => {
        pub static $name: once_cell::sync::Lazy<$crate::texture_base::material::CopiedMaterial> =
        once_cell::sync::Lazy::new(|| $crate::texture_base::material::CopiedMaterial {
            name: const_format::concatcp!("block/", const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name))),
            source: {
                use $crate::texture_base::material::Material;
                $base.get_output_task_by_name($base_name).unwrap()
            }
        });
    }
}

#[macro_export]
macro_rules! block_with_colors {
    ($name:ident = $color:expr, $shadow:expr, $highlight:expr, $background:expr, $( $layers:expr ),* ) => {
        macro_rules! color {
            () => { $color }
        }
        macro_rules! shadow {
            () => { $shadow }
        }
        macro_rules! highlight {
            () => { $highlight }
        }
        pub static $name: once_cell::sync::Lazy<$crate::texture_base::material::SingleTextureTricolorMaterial>
            = once_cell::sync::Lazy::new(||
            $crate::texture_base::material::SingleTextureTricolorMaterial {
                colors: $crate::texture_base::material::ColorTriad {
                    color: color!(),
                    shadow: shadow!(),
                    highlight: highlight!()
                },
                material: $crate::texture_base::material::SingleTextureMaterial::new(
                    const_format::concatcp!("block/",
                        const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name))
                    ),
                    $crate::stack_on!($background, $($layers),*).into()
                )
            }
        );
    }
}

#[macro_export]
macro_rules! make_tricolor_block_macro {
    ($macro_name:ident, $color:expr, $shadow:expr, $highlight:expr) => {
        #[macro_export]
        macro_rules! $macro_name {
            ($$name:ident = $$background:expr, $$($$layers:expr),*) => {
                $crate::block_with_colors!($$name =
                    $color,
                    $shadow,
                    $highlight,

                    $$background,
                    $$($$layers),*
                );
            }
        }
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct DoubleTallBlock {
    pub name: &'static str,
    pub bottom: ToPixmapTaskSpec,
    pub top: ToPixmapTaskSpec
}

impl Material for DoubleTallBlock {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        Arc::new([out_task(&format!("block/{}_bottom", self.name), self.bottom.to_owned()),
            out_task(&format!("block/{}_top", self.name), self.top.to_owned())
        ])
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct GroundCoverBlock {
    pub name: &'static str,
    pub top_name_suffix: &'static str,
    pub colors: ColorTriad,
    pub base: ToPixmapTaskSpec,
    pub cover_side: ToPixmapTaskSpec,
    pub top: ToPixmapTaskSpec,
}

impl Material for GroundCoverBlock {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        Arc::new([
            out_task(
                &format!("block/{}{}", self.name, self.top_name_suffix),
                self.top.to_owned()
            ),
            out_task(
                &format!("block/{}_side", self.name),
                ToPixmapTaskSpec::StackLayerOnLayer {
                    background: Box::new(self.base.to_owned()),
                    foreground: Box::new(self.cover_side.to_owned())
                }
            )
        ])
    }
}

impl TricolorMaterial for GroundCoverBlock {
    fn color(&self) -> ComparableColor {
        self.colors.color
    }

    fn shadow(&self) -> ComparableColor {
        self.colors.shadow
    }

    fn highlight(&self) -> ComparableColor {
        self.colors.highlight
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ground_cover_block(name: &'static str,
                          top_name_suffix: &'static str,
                          base: &SingleTextureMaterial,
                          color: ComparableColor,
                          shadow: ComparableColor,
                          highlight: ComparableColor,
                          cover_side: ToPixmapTaskSpec,
                          top: ToPixmapTaskSpec
)->GroundCoverBlock {
    GroundCoverBlock {
        name, top_name_suffix, base: base.texture(),
        colors: ColorTriad {color, shadow, highlight},
        cover_side, top
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleLayerMaterial {
    pub name: &'static str,
    pub layer_name: &'static str,
    pub color: Option<ComparableColor>,
}

impl Material for SingleLayerMaterial {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        Arc::new([out_task(self.name,
             if let Some(color) = self.color {
                 paint_svg_task(self.layer_name, color)
             } else {
                 from_svg_task(self.layer_name)
             })])
    }
}

pub const REDSTONE_ON: ComparableColor = c(0xff5e5e);

pub struct RedstoneOffOnBlockPair {
    pub name: &'static str,
    pub create_texture: Box<dyn Fn(ComparableColor) -> ToPixmapTaskSpec + Send + Sync>
}

impl Material for RedstoneOffOnBlockPair {
    fn get_output_tasks(&self) -> Arc<[FileOutputTaskSpec]> {
        Arc::new([out_task(
                &format!("block/{}", self.name),
                (self.create_texture)(ComparableColor::BLACK)
        ),
        out_task(
            &format!("block/{}_on", self.name),
            (self.create_texture)(REDSTONE_ON)
        )])
    }
}

#[macro_export]
macro_rules! redstone_off_on_block {
    ($name:ident = $create_texture:expr ) => {
        pub static $name: once_cell::sync::Lazy<$crate::texture_base::material::RedstoneOffOnBlockPair> =
        once_cell::sync::Lazy::new(|| $crate::texture_base::material::RedstoneOffOnBlockPair {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            create_texture: Box::new(|state_color| { {
                macro_rules! state_color {
                    () => {state_color}
                }
                $create_texture
            } })
        });
    }
}

pub type TextureSupplier<T> = Box<dyn Fn(&T) -> ToPixmapTaskSpec + Send + Sync>;
pub type TextureUnaryFunc<T> = Box<dyn Fn(&T, ToPixmapTaskSpec) -> ToPixmapTaskSpec + Send + Sync>;
pub type TextureBinaryFunc<T> = Box<dyn Fn(&T, ToPixmapTaskSpec, ToPixmapTaskSpec) -> ToPixmapTaskSpec + Send + Sync>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ColorTriad {
    pub(crate) color: ComparableColor,
    pub(crate) shadow: ComparableColor,
    pub(crate) highlight: ComparableColor,
}