use std::fmt::Debug;
use std::hash::Hash;

use std::path::PathBuf;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::color::rgb;
use crate::image_tasks::task_spec::{out_task, paint_svg_task, SinkTaskSpec, ToPixmapTaskSpec};

/// Specification in DSL form of how one or more texture images are to be generated.
pub trait Material: Send {
    /// Converts this specification to a number of [PngOutput] instances, each of which references
    /// another [TaskSpec] to generate the image it will output.
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec>;
}

pub struct MaterialGroup {
    pub(crate) tasks: Vec<SinkTaskSpec>
}

impl Material for MaterialGroup {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
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
        lazy_static::lazy_static! {pub static ref $name: $crate::texture_base::material::MaterialGroup
        = {
            let mut tasks: Vec<crate::image_tasks::task_spec::SinkTaskSpec>
                = Vec::with_capacity($crate::texture_base::material::DEFAULT_GROUP_SIZE);
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
    pub has_output: bool,
    pub texture: Box<ToPixmapTaskSpec>
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureTricolorMaterial {
    pub material: SingleTextureMaterial,
    pub colors: ColorTriad
}

impl Material for SingleTextureTricolorMaterial {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
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

impl From<SingleTextureMaterial> for Box<ToPixmapTaskSpec> {
    fn from(val: SingleTextureMaterial) -> Self {
        val.texture
    }
}

impl Material for SingleTextureMaterial {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        if !self.has_output { vec![] } else {
            vec![out_task(&format!("{}/{}", self.directory, self.name),
                          self.texture.to_owned())]
        }
    }
}

#[macro_export]
macro_rules! single_texture_material {
    ($name:ident = $directory:expr, $background:expr, $( $layers:expr ),* ) => {
        lazy_static::lazy_static! {pub static ref $name: $crate::texture_base::material::SingleTextureMaterial =
        $crate::texture_base::material::SingleTextureMaterial {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            directory: $directory,
            has_output: true,
            texture: crate::stack_on!($background, $($layers),*).into()
        };}
    }
}

pub fn item(name: &'static str, texture: Box<ToPixmapTaskSpec>) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "item", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_item {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "item", $background, $($layers),*);
    }
}

pub fn block(name: &'static str, texture: Box<ToPixmapTaskSpec>) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "block", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_block {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "block", $background, $($layers),*);
    }
}

#[macro_export]
macro_rules! copy_block {
    ($name:ident = $base:expr) => {
        lazy_static::lazy_static! {pub static ref $name: $crate::texture_base::material::SingleTextureMaterial =
        $crate::texture_base::material::SingleTextureMaterial {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            directory: "block",
            has_output: true,
            texture: $base.to_owned()
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
                    has_output: true,
                    texture: crate::stack_on!($background, $($layers),*).into()
                }
            };
        }
    }
}

pub fn particle(name: &'static str, texture: Box<ToPixmapTaskSpec>) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "particle", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_particle {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "particle", $background, $($layers),*);
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct DoubleTallBlock {
    name: &'static str,
    bottom: Box<ToPixmapTaskSpec>,
    top: Box<ToPixmapTaskSpec>
}

impl Material for DoubleTallBlock {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        vec![out_task(&format!("block/{}_bottom", self.name), self.bottom.to_owned()),
            out_task(&format!("block/{}_top", self.name), self.top.to_owned())
        ]
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct GroundCoverBlock {
    pub name: &'static str,
    pub colors: ColorTriad,
    pub base: Box<ToPixmapTaskSpec>,
    pub cover_side: Box<ToPixmapTaskSpec>,
    pub top: Box<ToPixmapTaskSpec>,
}

impl Material for GroundCoverBlock {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        vec![SinkTaskSpec::PngOutput {
            base: self.top.to_owned(),
            destinations: vec![PathBuf::from(format!("block/{}_top", self.name))]},
        SinkTaskSpec::PngOutput {
            base: Box::new(ToPixmapTaskSpec::StackLayerOnLayer {
                background: self.base.to_owned(),
                foreground: self.cover_side.to_owned()
            }),
            destinations: vec![PathBuf::from(format!("block/{}_side", self.name))],
        }]
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

#[macro_export]
macro_rules! ground_cover_block {
    ($name:ident = $base:expr, $color:expr, $shadow:expr, $highlight:expr, $cover_side:expr, $top:expr ) => {
        macro_rules! color {
            () => { $color }
        }
        macro_rules! shadow {
            () => { $shadow }
        }
        macro_rules! highlight {
            () => { $highlight }
        }
        lazy_static::lazy_static! {pub static ref $name: $crate::texture_base::material::GroundCoverBlock =
        $crate::texture_base::material::GroundCoverBlock {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            colors: $crate::texture_base::material::ColorTriad {
                color: color!(),
                shadow: shadow!(),
                highlight: highlight!()
            },
            base: $base.material.texture.to_owned(),
            cover_side: {$cover_side},
            top: {$top}
        };}
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleLayerMaterial {
    pub name: &'static str,
    pub layer_name: &'static str,
    pub color: ComparableColor,
}

impl Material for SingleLayerMaterial {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        vec![out_task(self.name,
             paint_svg_task(self.layer_name, self.color))]
    }
}

pub const REDSTONE_ON: ComparableColor = rgb(0xff, 0x5e, 0x5e);

pub fn redstone_off_and_on(name: &str, generator: Box<dyn Fn(ComparableColor) -> ToPixmapTaskSpec>)
-> Vec<SinkTaskSpec> {
    vec![SinkTaskSpec::PngOutput {
        base: Box::new(generator(ComparableColor::BLACK)),
        destinations: vec![PathBuf::from(name)]
    },
    SinkTaskSpec::PngOutput {
        base: Box::new(generator(REDSTONE_ON)),
        destinations: vec![PathBuf::from(format!("{}_on", name))]
    }]
}

pub type AbstractTextureSupplier<T> = Box<dyn Fn(&T) -> Box<ToPixmapTaskSpec> + Send + Sync>;
pub type AbstractTextureUnaryFunc<T> = Box<dyn Fn(&T, Box<ToPixmapTaskSpec>) -> Box<ToPixmapTaskSpec> + Send + Sync>;
pub type AbstractTextureBinaryFunc<T> = Box<dyn Fn(&T, Box<ToPixmapTaskSpec>,Box<ToPixmapTaskSpec>) -> Box<ToPixmapTaskSpec> + Send + Sync>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ColorTriad {
    pub(crate) color: ComparableColor,
    pub(crate) shadow: ComparableColor,
    pub(crate) highlight: ComparableColor,
}