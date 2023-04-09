use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::color::rgb;
use crate::image_tasks::task_spec::{out_task, paint_svg_task, TaskSpec};
use crate::image_tasks::task_spec::TaskSpec::{PngOutput, StackLayerOnLayer};


pub trait Material: Sync + Send {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>>;
}

#[derive(Clone)]
pub struct MaterialGroup<'a> {
    pub(crate) members: Vec<Arc<&'a dyn Material>>
}

impl <'a> Material for MaterialGroup<'a> {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return self.members.iter()
            .flat_map(|material| material.get_output_tasks())
            .collect();
    }
}

#[macro_export]
macro_rules! group {
    ($name:ident = $( $members:expr ),* ) => {
        lazy_static::lazy_static! {pub static ref $name: crate::texture_base::material::MaterialGroup<'static>
        = crate::texture_base::material::MaterialGroup {
            members: vec![$(std::sync::Arc::new(&*$members)),*]
        };}
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureMaterial {
    pub name: &'static str,
    pub directory: &'static str,
    pub has_output: bool,
    pub texture: Arc<TaskSpec>
}

impl Into<Arc<TaskSpec>> for SingleTextureMaterial {
    fn into(self) -> Arc<TaskSpec> {
        return self.texture;
    }
}

impl Material for SingleTextureMaterial {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return if !self.has_output.to_owned() { vec!() } else {
            vec!(out_task(&*format!("{}/{}", self.directory, self.name),
                          self.texture.to_owned()))
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

pub fn item(name: &'static str, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
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

pub fn block(name: &'static str, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
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

pub fn particle(name: &'static str, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
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
    pub base: Arc<TaskSpec>,
    pub cover_side_layers: Arc<TaskSpec>,
    pub top: Arc<TaskSpec>,
}

impl Material for GroundCoverBlock {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return vec![Arc::new(PngOutput {
            base: self.top.clone(),
            destinations: vec![PathBuf::from(format!("block/{}_top", self.name))]}),
        Arc::new(PngOutput {
            base: Arc::new(StackLayerOnLayer {
                background: self.base.clone(),
                foreground: self.cover_side_layers.clone()
            }),
            destinations: vec![PathBuf::from(format!("block/{}_side", self.name))]})];
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleLayerMaterial {
    pub name: &'static str,
    pub layer_name: &'static str,
    pub color: ComparableColor,
}

impl Material for SingleLayerMaterial {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return vec!(out_task(&*self.name,
             paint_svg_task(&*self.layer_name, self.color.to_owned())));
    }
}

pub const REDSTONE_ON: ComparableColor = rgb(0xff, 0x5e, 0x5e);

pub fn redstone_off_and_on(name: &str, generator: Box<dyn Fn(ComparableColor) -> TaskSpec>)
-> Vec<TaskSpec> {
    let mut out: Vec<TaskSpec> = vec!();
    out.push(PngOutput {
        base: Arc::new(generator(ComparableColor::BLACK)),
        destinations: vec![PathBuf::from(name)]
    });
    out.push(PngOutput {
        base: Arc::new(generator(REDSTONE_ON)),
        destinations: vec![PathBuf::from(format!("{}_on", name))]
    });
    return out;
}
