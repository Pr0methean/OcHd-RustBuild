#![feature(const_trait_impl)]
#![feature(const_type_id)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(ptr_metadata)]
#![feature(async_closure)]

mod tasks;
mod image_tasks;
mod texture_base;
mod materials;
use std::any::{Any, TypeId};
use fn_graph::{FnGraphBuilder, FnId, TypeIds};
use std::collections::{HashMap};
use std::env;
use std::future::Future;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};
use cached::once_cell::sync::Lazy;
use chashmap_next::CHashMap;
use fn_meta::{FnMetaDyn};
use resman::Resources;
use lazy_static::lazy_static;
use texture_base::material::Material;
use threadpool::ThreadPool;
use crate::image_tasks::task_spec::TaskSpec;

lazy_static! {
    static ref TASKS: Vec<TaskSpec> = materials::ALL_MATERIALS.get_output_tasks();
    static ref TILE_SIZE: u32 = {
        let args: Vec<String> = env::args().collect();
        args[1].parse::<u32>()
            .expect("Tile size must be an integer")
    };
}

fn main() {
    let tile_size: u32 = *TILE_SIZE;
    println!("Using {:?} pixels per tile", tile_size);
    let mut g: FnGraphBuilder<&TaskSpec> = FnGraphBuilder::new();
    let mut added_tasks: HashMap<&TaskSpec, FnId> = HashMap::new();
    TASKS.iter()
        .for_each(|task| {
            task.add_to(&mut g, &mut added_tasks, tile_size);});
    let mut graph = g.build();
    let thread_pool = ThreadPool::new(num_cpus::get());
    let num_total_tasks = graph.node_count();
    println!("Graph contains {} total tasks", num_total_tasks);
    graph.for_each(|f| thread_pool.execute(
        || {
            f.register();
        }));
    thread_pool.join();
}