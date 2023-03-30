use std::ops::Deref;
use crate::image_tasks::task_spec::{name_to_out_path, TaskSpec};
use strum::IntoEnumIterator;
use std::path::{PathBuf};
use std::sync::Arc;
use crate::image_tasks::color::c;
use crate::image_tasks::color::ComparableColor;
use crate::image_tasks::task_spec::TaskSpec::PngOutput;
use crate::image_tasks::task_spec::TaskSpec::Stack;

trait Material {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>>;
}

struct MaterialGroup {
    members: Vec<Box<dyn Material>>
}

impl Material for MaterialGroup {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return self.members.iter()
            .flat_map(|material| material.get_output_tasks())
            .collect();
    }
}

impl <E, F, G> Material for E where E: IntoEnumIterator<Iterator=F>, F : Iterator<Item=G>, G: Material {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return E::iter()
            .flat_map(|material| material.get_output_tasks())
            .collect();
    }
}

pub struct SingleTextureMaterial {
    name: String,
    directory: String,
    has_output: bool,
    texture: Arc<TaskSpec>
}

impl Into<TaskSpec> for SingleTextureMaterial {
    fn into(self) -> TaskSpec {
        return self.texture.deref().clone();
    }
}

impl Material for SingleTextureMaterial {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        return if !self.has_output { vec!() } else {
            vec!(Arc::new(PngOutput{base: self.texture.clone(),
                destinations: Arc::new(vec!(name_to_out_path(format!("{}/{}", self.directory, self.name))))}))
        };
    }
}

pub fn item(name: String, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "items".to_string(), has_output: true, texture
    }
}

pub fn block(name: String, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "blocks".to_string(), has_output: true, texture
    }
}

pub fn particle(name: String, texture: Arc<TaskSpec>) -> SingleTextureMaterial {
    return SingleTextureMaterial {
        name, directory: "particles".to_string(), has_output: true, texture
    }
}

pub struct DoubleTallBlock {
    name: String,
    bottom_layers: Arc<TaskSpec>,
    top_layers: Arc<TaskSpec>
}

impl Material for DoubleTallBlock {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        let mut output_tasks = block(format!("{}_bottom", self.name), self.bottom_layers.clone()).get_output_tasks();
        output_tasks.extend(block(format!("{}_top", self.name), self.top_layers.clone()).get_output_tasks());
        return output_tasks;
    }
}

struct GroundCoverBlock<'a> {
    name: String,
    base: &'a TaskSpec,
    cover_side_layers: Vec<Arc<TaskSpec>>,
    top: &'a TaskSpec,
}

impl <'a> Material for GroundCoverBlock<'a> {
    fn get_output_tasks(&self) -> Vec<Arc<TaskSpec>> {
        let mut side_layers: Vec<Arc<TaskSpec>> = vec!(Arc::new(self.base.clone()));
        for cover_side_layer in self.cover_side_layers.clone() {
            side_layers.push(cover_side_layer);
        }
        return vec!(Arc::new(PngOutput {
            base: Arc::new(self.top.clone()),
            destinations: Arc::new(vec!(PathBuf::from(format!("block/{}_top", self.name))))}),
        Arc::new(PngOutput {
            base: Arc::new(Stack {
                background: ComparableColor::TRANSPARENT,
                layers: side_layers}),
            destinations: Arc::new(vec!(PathBuf::from(format!("block/{}_side", self.name))))})
        );
    }
}

pub const REDSTONE_ON: ComparableColor = c(0xff, 0x5e, 0x5e);

pub fn redstone_off_and_on(name: String, generator: Box<dyn Fn(ComparableColor) -> TaskSpec>) -> Vec<Arc<TaskSpec>> {
    let mut out: Vec<Arc<TaskSpec>> = vec!();
    out.push(Arc::new(PngOutput {
        base: Arc::new(generator(ComparableColor::BLACK)),
        destinations: Arc::new(vec!(PathBuf::from(&name)))
    }));
    out.push(Arc::new(PngOutput {
        base: Arc::new(generator(REDSTONE_ON)),
        destinations: Arc::new(vec!(PathBuf::from(format!("{}_on", name))))
    }));
    return out;
}

/*
fun OutputTaskEmitter.redstoneOffAndOn(baseName: String,
                                       layers: LayerListBuilder.(redstoneStateColor: Color) -> Unit) {
    out(baseName) { layers(Color.BLACK) }
    out(baseName + "_on") { layers(Ore.REDSTONE.highlight) }
}

 */
