use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::color::rgb;
use crate::image_tasks::task_spec::{out_task, paint_svg_task, TaskSpec};
use crate::image_tasks::task_spec::TaskSpec::{PngOutput, StackLayerOnLayer};


pub trait Material: Sync + Send {
    fn get_output_tasks(&self) -> Vec<TaskSpec>;
}

#[derive(Clone)]
pub struct MaterialGroup {
    pub(crate) tasks: Vec<TaskSpec>
}

impl Material for MaterialGroup {
    fn get_output_tasks(&self) -> Vec<TaskSpec> {
        return self.tasks.to_owned();
    }
}

pub const DEFAULT_GROUP_SIZE: usize = 1024;

#[macro_export]
macro_rules! group {
    ($name:ident = $( $members:expr ),* ) => {
        lazy_static::lazy_static! {pub static ref $name: crate::texture_base::material::MaterialGroup
        = {
            let mut tasks = Vec::with_capacity(crate::texture_base::material::DEFAULT_GROUP_SIZE);
            $(
            let mut more_tasks = crate::texture_base::material::Material::get_output_tasks($members.deref());
            tasks.append(&mut more_tasks);)*
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
    pub texture: Box<TaskSpec>
}

impl Into<Box<TaskSpec>> for SingleTextureMaterial {
    fn into(self) -> Box<TaskSpec> {
        return self.texture;
    }
}

impl Material for SingleTextureMaterial {
    fn get_output_tasks(&self) -> Vec<TaskSpec> {
        return if !self.has_output.to_owned() { vec![] } else {
            vec![out_task(&*format!("{}/{}", self.directory, self.name),
                          self.texture.to_owned()).deref().to_owned()]
        };
    }
}

#[macro_export]
macro_rules! single_texture_material {
    ($name:ident = $directory:expr, $background:expr, $( $layers:expr ),* ) => {
        lazy_static::lazy_static! {pub static ref $name: crate::texture_base::material::SingleTextureMaterial =
        crate::texture_base::material::SingleTextureMaterial {
            name: const_format::map_ascii_case!(const_format::Case::Lower, &stringify!($name)),
            directory: $directory,
            has_output: true,
            texture: crate::stack_on!($background, $($layers),*).into()
        };}
    }
}

pub fn item(name: &'static str, texture: Box<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "items", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_item {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        crate::single_texture_material!($name = "item", $background, $($layers),*);
    }
}

pub fn block(name: &'static str, texture: Box<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "blocks", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_block {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        crate::single_texture_material!($name = "block", $background, $($layers),*);
    }
}

pub fn particle(name: &'static str, texture: Box<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "particles", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_particle {
    ($name:ident = $background:expr, $( $layers:expr ),* ) => {
        crate::single_texture_material!($name = "particle", $background, $($layers),*);
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
    pub base: Box<TaskSpec>,
    pub cover_side_layers: Box<TaskSpec>,
    pub top: Box<TaskSpec>,
}

impl Material for GroundCoverBlock {
    fn get_output_tasks(&self) -> Vec<TaskSpec> {
        return vec![PngOutput {
            base: self.top.clone(),
            destinations: vec![PathBuf::from(format!("block/{}_top", self.name))]},
        PngOutput {
            base: Box::new(StackLayerOnLayer {
                background: self.base.clone(),
                foreground: self.cover_side_layers.clone()
            }),
            destinations: vec![PathBuf::from(format!("block/{}_side", self.name))],
        }];
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleLayerMaterial {
    pub name: &'static str,
    pub layer_name: &'static str,
    pub color: ComparableColor,
}

impl Material for SingleLayerMaterial {
    fn get_output_tasks(&self) -> Vec<TaskSpec> {
        return vec!(out_task(&*self.name,
             paint_svg_task(&*self.layer_name, self.color.to_owned())).deref().to_owned());
    }
}

pub const REDSTONE_ON: ComparableColor = rgb(0xff, 0x5e, 0x5e);

pub fn redstone_off_and_on(name: &str, generator: Box<dyn Fn(ComparableColor) -> TaskSpec>)
-> Vec<TaskSpec> {
    let mut out: Vec<TaskSpec> = Vec::with_capacity(2);
    out.push(PngOutput {
        base: Box::new(generator(ComparableColor::BLACK)),
        destinations: vec![PathBuf::from(name)]
    });
    out.push(PngOutput {
        base: Box::new(generator(REDSTONE_ON)),
        destinations: vec![PathBuf::from(format!("{}_on", name))]
    });
    return out;
}