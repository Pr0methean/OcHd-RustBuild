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
#![feature(hash_set_entry)]
#![feature(once_cell_try)]
#![feature(allocator_api)]

use std::alloc::System;
use std::collections::{HashMap};
use std::env;
use std::io::ErrorKind::NotFound;
use std::path::absolute;
use std::time::Instant;

use async_std::fs::{create_dir, remove_dir_all};
use fn_graph::daggy::Dag;
use petgraph::visit::{EdgeRef, IntoNodeReferences, IntoEdgeReferences, NodeIndexable};
use fn_graph::daggy::petgraph::unionfind::UnionFind;
use futures::future::join_all;
use lazy_static::lazy_static;
use log::{info, LevelFilter};
use logging_allocator::LoggingAllocator;
use petgraph::graph::{DefaultIx, NodeIndex};
use texture_base::material::Material;

use crate::image_tasks::task_spec::{OUT_DIR, SVG_DIR, TaskResultFuture, TaskToFutureGraphNodeMap};

mod image_tasks;
mod texture_base;
mod materials;
lazy_static! {
    static ref TILE_SIZE: u32 = {
        let args: Vec<String> = env::args().collect();
        args.get(1).expect("Usage: OcHd-RustBuild <tile-size>").parse::<u32>()
            .expect("Tile size (first command-line argument) must be an integer")
    };
}

#[global_allocator]
static ALLOCATOR: LoggingAllocator = LoggingAllocator::with_allocator(System);

#[tokio::main]
async fn main() {
    simple_logging::log_to_file("./log.txt", LevelFilter::Trace).expect("Failed to configure file logging");
    ALLOCATOR.enable_logging();
    info!("Looking for SVGs in {}", absolute(SVG_DIR.to_path_buf()).unwrap().to_string_lossy());
    info!("Writing output to {}", absolute(OUT_DIR.to_path_buf()).unwrap().to_string_lossy());
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {:?} pixels per tile", tile_size);
    let start_time = Instant::now();

    let clean_out_dir = tokio::spawn(async {
        let result = remove_dir_all(OUT_DIR.to_owned()).await;
        if result.is_err_and(|err| err.kind() != NotFound) {
            panic!("Failed to delete old output directory");
        }
        create_dir(OUT_DIR.to_owned()).await.expect("Failed to create output directory");
    });

    let tasks = materials::ALL_MATERIALS.get_output_tasks();
    let mut graph: Dag<TaskResultFuture, (), DefaultIx> = Dag::new();
    let mut added_tasks: TaskToFutureGraphNodeMap<DefaultIx> = HashMap::new();
    tasks.iter().for_each(|task| {
            task.add_to(&mut graph, &mut added_tasks);});
    drop(added_tasks);

    // Split the graph into weakly-connected components (WCCs, groups that don't share any subtasks).
    // Used to save memory by minimizing the number of WCCs caching their subtasks at once.
    // Adapted from https://docs.rs/petgraph/latest/src/petgraph/algo/mod.rs.html#87-102.
    let mut vertex_sets = UnionFind::new(graph.node_bound());
    for edge in graph.edge_references() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        vertex_sets.union(a, b);
    }
    let mut component_map: HashMap<NodeIndex<DefaultIx>, Vec<TaskResultFuture>> = HashMap::new();
    for (index, task) in graph.node_references() {
        let representative = vertex_sets.find(index);
        match component_map.get_mut(&representative) {
            Some(existing) => {
                existing.push(task.to_owned());
            },
            None => {
                let mut vec = Vec::with_capacity(1024);
                vec.push(task.to_owned());
                component_map.insert(representative, vec);
            }
        };
    }
    drop(tasks);
    drop(graph);

    // Run small WCCs first so that their data can leave the heap before the big WCCs run
    let mut components: Vec<Vec<TaskResultFuture>> = component_map.into_values().collect();
    components.sort_by_key(Vec::len);
    let components = components.into_iter().flatten().into_iter();

    clean_out_dir.await.expect("Failed to create or replace output directory");
    join_all(components.map(tokio::spawn)).await;
    info!("Finished after {} ns", start_time.elapsed().as_nanos());
    ALLOCATOR.disable_logging();
}