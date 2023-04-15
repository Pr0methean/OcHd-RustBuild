use std::fmt::Debug;
use std::hash::Hash;


use std::path::PathBuf;
use std::sync::Arc;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::color::rgb;
use crate::image_tasks::task_spec::{out_task, paint_svg_task, SinkTaskSpec, TaskSpec, ToPixmapTaskSpec};

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

pub const DEFAULT_GROUP_SIZE: usize = 1024;

#[macro_export]
macro_rules! group {
    ($name:ident = $( $members:expr ),* ) => {
        lazy_static::lazy_static! {pub static ref $name: $crate::texture_base::material::MaterialGroup
        = {
            let mut tasks: Vec<crate::image_tasks::task_spec::SinkTaskSpec>
                = Vec::with_capacity($crate::texture_base::material::DEFAULT_GROUP_SIZE);
            $({
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

impl From<SingleTextureMaterial> for Box<ToPixmapTaskSpec> {
    fn from(val: SingleTextureMaterial) -> Self {
        val.texture
    }
}

impl Material for SingleTextureMaterial {
    fn get_output_tasks(&self) -> Vec<SinkTaskSpec> {
        if !self.has_output { vec![] } else {
            vec![*out_task(&format!("{}/{}", self.directory, self.name),
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
        name, directory: "items", has_output: true, texture
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
        name, directory: "blocks", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_block {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "block", $background, $($layers),*);
    }
}

pub fn particle(name: &'static str, texture: Box<ToPixmapTaskSpec>) -> SingleTextureMaterial {
    SingleTextureMaterial {
        name, directory: "particles", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_particle {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        $crate::single_texture_material!($name = "particle", $background, $($layers),*);
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct DoubleTallBlock<'a> {
    name: &'a str,
    bottom_layers: Arc<TaskSpec>,
    top_layers: Arc<TaskSpec>
}

/*
FIXME: This doesn't compile:

impl Material for DoubleTallBlock<'static> {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        let mut output_tasks = block(&formatcp!("{}_bottom", self.name),
                                     self.bottom_layers.to_owned()).get_output_tasks();
        output_tasks.extend(block(&formatcp!("{}_top", self.name), self.top_layers.to_owned()).get_output_tasks());
        return output_tasks;
    }
}

*/

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct GroundCoverBlock {
    pub name: &'static str,
    pub base: Box<ToPixmapTaskSpec>,
    pub cover_side_layers: Box<ToPixmapTaskSpec>,
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
                foreground: self.cover_side_layers.to_owned()
            }),
            destinations: vec![PathBuf::from(format!("block/{}_side", self.name))],
        }]
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
        vec![*out_task(self.name,
             paint_svg_task(self.layer_name, self.color.to_owned()))]
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
