use std::ops::{Deref};
use crate::image_tasks::task_spec::{out_task, paint_svg_task, TaskSpec};
use strum::IntoEnumIterator;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::{PathBuf};
use std::sync::Arc;
use crate::image_tasks::color::rgb;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::TaskSpec::PngOutput;
use crate::image_tasks::task_spec::TaskSpec::Stack;

pub trait Material: Sync + Send {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>>;
}

#[derive(Clone)]
pub struct MaterialGroup {
    pub(crate) members: Vec<Arc<dyn Material>>
}

impl Material for MaterialGroup {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return self.members.iter()
            .flat_map(|material| material.get_output_tasks())
            .collect();
    }
}

#[macro_export]
macro_rules! group {
    ($name:ident = $( $members:expr ),* ) => {
        pub const $name: std::sync::Arc<crate::texture_base::material::MaterialGroup>
        = std::sync::Arc::new(crate::texture_base::material::MaterialGroup {
            members: vec![$(std::sync::Arc::new($members)),*]
        })
    }
}

impl <E, F, G> Material for E where E: IntoEnumIterator<Iterator=F> + Sync + Send,
                                    F : Iterator<Item=G>, G: Material {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return E::iter()
            .flat_map(|material| material.get_output_tasks())
            .collect();
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SingleTextureMaterial {
    pub(crate) name: &'static str,
    directory: &'static str,
    has_output: bool,
    texture: Arc<TaskSpec>
}

impl Into<TaskSpec> for SingleTextureMaterial {
    fn into(self) -> TaskSpec {
        return self.texture.deref().to_owned();
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
    ($name:ident = $directory:expr, $texture:expr ) => {
        pub const $name: std::sync::Arc<crate::texture_base::material::SingleTextureMaterial> =
        std::sync::Arc::new(crate::texture_base::material::SingleTextureMaterial {
            name: const_str::convert_ascii_case!(lower, &stringify!($name)),
            directory: $directory,
            has_output: true,
            texture: $texture
        });
    }
}

pub fn item(name: &str, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "items", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_item {
    ($name:ident = $texture:expr ) => {
        crate::single_texture_material!($name = "item", $texture);
    }
}

pub fn block(name: &str, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "blocks", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_block {
    ($name:ident = $texture:expr ) => {
        crate::single_texture_material!($name = "block", $texture);
    }
}

pub fn particle(name: &str, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "particles", has_output: true, texture
    }
}

#[macro_export]
macro_rules! single_texture_particle {
    ($name:ident = $texture:expr ) => {
        crate::single_texture_material!($name = "particle", $texture);
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct DoubleTallBlock {
    name: &'static str,
    bottom_layers: Arc<TaskSpec>,
    top_layers: Arc<TaskSpec>
}

impl Material for DoubleTallBlock {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        let mut output_tasks = block(&*format!("{}_bottom", self.name), self.bottom_layers.to_owned()).get_output_tasks();
        output_tasks.extend(block(&*format!("{}_top", self.name), self.top_layers.to_owned()).get_output_tasks());
        return output_tasks;
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct GroundCoverBlock {
    name: &'static str,
    base: Arc<TaskSpec>,
    cover_side_layers: Vec<Arc<TaskSpec>>,
    top: Arc<TaskSpec>,
}

impl Material for GroundCoverBlock {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        let mut side_layers: Vec<Arc<TaskSpec>> = vec!(self.base.to_owned());
        side_layers.extend(self.cover_side_layers.to_owned());
        return vec!(Arc::new(PngOutput {
            base: self.top.to_owned(),
            destinations: Arc::new(vec!(PathBuf::from(format!("block/{}_top", self.name))))}),
        Arc::new(PngOutput {
            base: Arc::new(Stack {
                background: ComparableColor::TRANSPARENT,
                layers: side_layers}),
            destinations: Arc::new(vec!(PathBuf::from(format!("block/{}_side", self.name))))})
        );
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct SingleLayerMaterial {
    name: &'static str,
    layer_name: &'static str,
    color: ComparableColor,
}

impl Material for SingleLayerMaterial {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return vec!(out_task(&*self.name,
                             paint_svg_task(&*self.layer_name, self.color.to_owned())));
    }
}

pub const REDSTONE_ON: ComparableColor = rgb(0xff, 0x5e, 0x5e);

pub fn redstone_off_and_on(name: &str, generator: Box<dyn Fn(ComparableColor) -> TaskSpec>) -> Vec<Arc<TaskSpec>> {
    let mut out: Vec<Arc<TaskSpec>> = vec!();
    out.push(Arc::new(PngOutput {
        base: Arc::new(generator(ComparableColor::BLACK)),
        destinations: Arc::new(vec!(PathBuf::from(name)))
    }));
    out.push(Arc::new(PngOutput {
        base: Arc::new(generator(REDSTONE_ON)),
        destinations: Arc::new(vec!(PathBuf::from(format!("{}_on", name))))
    }));
    return out;
}
