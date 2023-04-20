use std::fmt::Debug;
use std::hash::Hash;

use crate::anyhoo;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::color::rgb;
use crate::image_tasks::task_spec::{out_task, paint_svg_task, FileOutputTaskSpec, ToPixmapTaskSpec, name_to_out_path, CloneableError};

/// Specification in DSL form of how one or more texture images are to be generated.
pub trait Material: Send {
    /// Converts this specification to a number of [PngOutput] instances, each of which references
    /// another [TaskSpec] to generate the image it will output.
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec>;

    fn get_output_task_by_name(&self, name: &str) -> Result<FileOutputTaskSpec, CloneableError> {
        for output_task in self.get_output_tasks() {
            if output_task.get_path().to_string_lossy().contains(name) {
                return Ok(output_task);
            }
        }
        Err(anyhoo!("No output task found with name {}", name))
    }
}

pub struct MaterialGroup {
    pub(crate) tasks: Vec<FileOutputTaskSpec>
}

impl Material for MaterialGroup {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
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
        lazy_static::lazy_static! {pub static ref $name: crate::texture_base::material::MaterialGroup
        = {
            let mut tasks: Vec<crate::image_tasks::task_spec::FileOutputTaskSpec>
                = Vec::with_capacity(crate::texture_base::material::DEFAULT_GROUP_SIZE);
            $({
                #![allow(unused)]
                use crate::texture_base::material::Material;
                tasks.extend($members.get_output_tasks());
            })*
            tasks.shrink_to_fit();
            crate::texture_base::material::MaterialGroup { tasks }
        };}
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureMaterial {
    pub name: &'static str,
    pub directory: &'static str,
    pub texture: ToPixmapTaskSpec
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureTricolorMaterial {
    pub material: SingleTextureMaterial,
    pub colors: ColorTriad
}

impl Material for SingleTextureTricolorMaterial {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
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
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![out_task(&format!("{}/{}", self.directory, self.name),
                          self.texture.to_owned())]
    }
}

#[macro_export]
macro_rules! material {
    ($name:ident = $directory:expr, $texture:expr) => {
        lazy_static::lazy_static! {
            pub static ref $name: crate::texture_base::material::SingleTextureMaterial =
                    crate::texture_base::material::SingleTextureMaterial {
                name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
                directory: $directory,
                texture: $texture.into()
            };
        }
    }
}

#[macro_export]
macro_rules! single_texture_material {
    ($name:ident = $directory:expr, $background:expr, $( $layers:expr ),* ) => {
        crate::material!($name = $directory, crate::stack_on!($background, $($layers),*));
    }
}

#[macro_export]
macro_rules! single_layer_material {
    ($name:ident = $directory:expr, $layer_name:expr, $color:expr ) => {
        crate::material!($name = $directory,
            crate::image_tasks::task_spec::paint_svg_task($layer_name, $color));
    };
    ($name:ident = $directory:expr, $layer_name:expr) => {
        crate::material!($name = $directory,
            crate::image_tasks::task_spec::from_svg_task($layer_name));
    };
}

pub fn item(name: &'static str, texture: ToPixmapTaskSpec) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "item", texture
    }
}

#[macro_export]
macro_rules! single_texture_item {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        crate::single_texture_material!($name = "item", $background, $($layers),*);
    }
}

#[macro_export]
macro_rules! single_layer_item {
    ($name:ident = $($layer_name_and_maybe_color:expr),+ ) => {
        crate::single_layer_material!($name = "item", $($layer_name_and_maybe_color),+);
    }
}

pub fn block(name: &'static str, texture: ToPixmapTaskSpec) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "block", texture
    }
}

#[macro_export]
macro_rules! single_texture_block {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        crate::single_texture_material!($name = "block", $background, $($layers),*);
    }
}

#[macro_export]
macro_rules! single_layer_block {
    ($name:ident = $($layer_name_and_maybe_color:expr),+ ) => {
        crate::single_layer_material!($name = "block", $($layer_name_and_maybe_color),+);
    }
}

pub fn particle(name: &'static str, texture: ToPixmapTaskSpec) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "particle", texture
    }
}

#[macro_export]
macro_rules! single_texture_particle {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        crate::single_texture_material!($name = "particle", $background, $($layers),*);
    }
}

#[macro_export]
macro_rules! single_layer_particle {
    ($name:ident = $($layer_name_and_maybe_color:expr),+ ) => {
        crate::single_layer_material!($name = "particle", $($layer_name_and_maybe_color),+);
    }
}

pub struct CopiedMaterial {
    pub name: &'static str,
    pub source: FileOutputTaskSpec
}

impl Material for CopiedMaterial {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![FileOutputTaskSpec::Symlink {
            original: Box::new(self.source.to_owned()),
            link: name_to_out_path(self.name)
        }]
    }
}

#[macro_export]
macro_rules! copy_block {
    ($name:ident = $base:expr, $base_name:expr) => {
        lazy_static::lazy_static! {pub static ref $name: crate::texture_base::material::CopiedMaterial =
        crate::texture_base::material::CopiedMaterial {
            name: const_format::formatcp!("block/{}", const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name))),
            source: {
                use crate::texture_base::material::Material;
                $base.get_output_task_by_name($base_name).unwrap()
            }
        };}
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
        lazy_static::lazy_static! {
            pub static ref $name: crate::texture_base::material::SingleTextureTricolorMaterial =
            crate::texture_base::material::SingleTextureTricolorMaterial {
                colors: crate::texture_base::material::ColorTriad {
                    color: color!(),
                    shadow: shadow!(),
                    highlight: highlight!()
                },
                material: crate::texture_base::material::SingleTextureMaterial {
                    name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
                    directory: "block",
                    texture: crate::stack_on!($background, $($layers),*).into()
                }
            };
        }
    }
}

#[macro_export]
macro_rules! make_tricolor_block_macro {
    ($macro_name:ident, $color:expr, $shadow:expr, $highlight:expr) => {
        #[macro_export]
        macro_rules! $macro_name {
            ($$name:ident = $$background:expr, $$($$layers:expr),*) => {
                crate::block_with_colors!($$name =
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
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![out_task(&format!("block/{}_bottom", self.name), self.bottom.to_owned()),
            out_task(&format!("block/{}_top", self.name), self.top.to_owned())
        ]
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
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![
            out_task(
                &*format!("block/{}{}", self.name, self.top_name_suffix),
                self.top.to_owned()
            ),
            out_task(
                &*format!("block/{}_side", self.name),
                ToPixmapTaskSpec::StackLayerOnLayer {
                    background: Box::new(self.base.to_owned()),
                    foreground: Box::new(self.cover_side.to_owned())
                }
            )

        ]
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
        name, top_name_suffix, base: base.texture.to_owned(),
        colors: ColorTriad {color, shadow, highlight},
        cover_side, top
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleLayerMaterial {
    pub name: &'static str,
    pub layer_name: &'static str,
    pub color: ComparableColor,
}

impl Material for SingleLayerMaterial {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![out_task(self.name,
             paint_svg_task(self.layer_name, self.color))]
    }
}

pub const REDSTONE_ON: ComparableColor = rgb(0xff, 0x5e, 0x5e);

pub struct RedstoneOffOnBlockPair {
    pub name: &'static str,
    pub create_texture: Box<dyn Fn(ComparableColor) -> ToPixmapTaskSpec + Send + Sync>
}

impl Material for RedstoneOffOnBlockPair {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        vec![out_task(
                &*format!("block/{}", self.name),
                (self.create_texture)(ComparableColor::BLACK)
        ),
        out_task(
            &*format!("block/{}_on", self.name),
            (self.create_texture)(REDSTONE_ON)
        )]
    }
}

#[macro_export]
macro_rules! redstone_off_on_block {
    ($name:ident = $create_texture:expr ) => {
        lazy_static::lazy_static! {pub static ref $name: crate::texture_base::material::RedstoneOffOnBlockPair =
        crate::texture_base::material::RedstoneOffOnBlockPair {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            create_texture: Box::new(|state_color| { {
                macro_rules! state_color {
                    () => {state_color}
                }
                $create_texture
            } })
        };}
    }
}


pub type AbstractTextureSupplier<T> = Box<dyn Fn(&T) -> ToPixmapTaskSpec + Send + Sync>;
pub type AbstractTextureUnaryFunc<T> = Box<dyn Fn(&T, ToPixmapTaskSpec) -> ToPixmapTaskSpec + Send + Sync>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ColorTriad {
    pub(crate) color: ComparableColor,
    pub(crate) shadow: ComparableColor,
    pub(crate) highlight: ComparableColor,
}