#![feature(const_trait_impl)]
#![feature(const_type_id)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(ptr_metadata)]
#![feature(async_closure)]
#![feature(try_trait_v2_residual)]
#![feature(try_trait_v2)]
#![feature(is_some_and)]
#![feature(absolute_path)]
#![feature(result_option_inspect)]
#![feature(let_chains)]

use std::collections::HashMap;
use std::env;
use std::future::{IntoFuture};
use std::io::ErrorKind::NotFound;
use std::path::absolute;
use std::time::Instant;
use async_std::fs::{create_dir, remove_dir_all};
use fn_graph::{FnGraph, FnGraphBuilder, FnId};
use futures::future::join_all;
use lazy_static::lazy_static;
use tokio::task::JoinHandle;
use texture_base::material::Material;

use crate::image_tasks::task_spec::{OUT_DIR, SVG_DIR, TaskSpec};

mod image_tasks;
mod texture_base;
mod materials;
lazy_static! {
    static ref TASKS: Vec<TaskSpec> = materials::ALL_MATERIALS.get_output_tasks();
    static ref TILE_SIZE: u32 = {
        let args: Vec<String> = env::args().collect();
        args[1].parse::<u32>()
            .expect("Tile size must be an integer")
    };
    static ref GRAPH: FnGraph<&'static TaskSpec> = {
        let mut g: FnGraphBuilder<&'static TaskSpec> = FnGraphBuilder::new();
        let mut added_tasks: HashMap<&'static TaskSpec, FnId> = HashMap::new();
        TASKS.iter()
            .for_each(|task| {
                task.add_to(&mut g, &mut added_tasks, *TILE_SIZE);});
        g.build().to_owned()
    };
}

#[tokio::main]
async fn main() {
    println!("Looking for SVGs in {}", absolute(SVG_DIR.to_path_buf()).unwrap().to_string_lossy());
    println!("Writing output to {}", absolute(OUT_DIR.to_path_buf()).unwrap().to_string_lossy());
    let start_time = Instant::now();
    let result = remove_dir_all(OUT_DIR.to_owned()).await;
    if result.is_err_and(|err| err.kind() != NotFound) {
        panic!("Failed to delete old output directory");
    }
    create_dir(OUT_DIR.to_owned()).await.expect("Failed to create output directory");
    let tile_size: u32 = *TILE_SIZE;
    println!("Using {:?} pixels per tile", tile_size);
    let num_total_tasks = GRAPH.node_count();
    println!("Graph contains {} total tasks", num_total_tasks);
    let mut futures: Vec<JoinHandle<()>> = vec![];
    for task in GRAPH.iter() {
        let owned_task = task.to_owned();
        futures.push(tokio::spawn(
            async move {
                let success: () = owned_task.clone().into_future().await.try_into().expect(&*format!("Error getting {}", task));
                success
            }));
    }
    join_all(futures).await;
    println!("Finished after {} ns", start_time.elapsed().as_nanos())
}