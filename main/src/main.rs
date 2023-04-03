#![feature(const_trait_impl)]

mod tasks;
mod image_tasks;
mod texture_base;
mod materials;
use rgraph::{Graph, GraphSolver, ValuesCache, SolverStatus, SolverError};
use std::collections::{HashSet};
use std::env;
use std::sync::Arc;
use lazy_static::lazy_static;
use texture_base::material::Material;
use crate::image_tasks::task_spec::TaskSpec;

lazy_static! {
    static ref TASKS: Vec<Arc<TaskSpec>> = materials::ALL_MATERIALS.get_output_tasks();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let tile_size: u32 = args[1]
        .parse::<u32>()
        .expect("Tile size must be an integer");
    println!("Using {} pixels per tile", tile_size);
    let mut g = Graph::new();
    let mut added_tasks: HashSet<&TaskSpec> = HashSet::new();
    let mut cache = ValuesCache::new();
    TASKS.iter()
        .for_each(|task|
            task.add_to(&mut g, &mut added_tasks, tile_size));
    let mut solver = GraphSolver::new(&mut g, &mut cache);
    solver.execute_terminals().unwrap();
}