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
use std::sync::{Arc};
use std::time::Instant;

use async_std::fs::{create_dir, remove_dir_all};
use fn_graph::daggy::Dag;
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use fn_graph::daggy::petgraph::unionfind::UnionFind;
use fn_graph::daggy::petgraph::visit::{GraphBase, IntoEdgeReferences, NodeCompactIndexable};
use futures::future::join_all;
use lazy_static::lazy_static;
use log::{info, LevelFilter};
use logging_allocator::LoggingAllocator;
use petgraph::graph::DefaultIx;
use texture_base::material::Material;

use crate::image_tasks::task_spec::{OUT_DIR, SVG_DIR, TaskResultFuture, TaskSpec, TaskToFutureGraphNodeMap};

mod image_tasks;
mod texture_base;
mod materials;
lazy_static! {
    static ref TILE_SIZE: u32 = {
        let args: Vec<String> = env::args().collect();
        args[1].parse::<u32>()
            .expect("Tile size must be an integer")
    };
}

/**
 * Splits a dag into weakly-connected components (WCCs, groups that don't share any subtasks).
 * Used to save memory by minimizing the number of WCCs caching their subtasks at once.
 * Adapted from https://docs.rs/petgraph/latest/src/petgraph/algo/mod.rs.html#87-102.
 */
pub fn connected_components<G,N>(g: G) -> Vec<Vec<Arc<N>>>
    where
        G: NodeCompactIndexable + IntoEdgeReferences + IntoNodeReferences + GraphBase<NodeId=N>,
        N: Clone
{
    let mut vertex_sets = UnionFind::new(g.node_bound());
    for edge in g.edge_references() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        vertex_sets.union(g.to_index(a), g.to_index(b));
    }
    let mut component_map: HashMap<usize, Vec<Arc<N>>> = HashMap::new();
    for node in g.node_identifiers() {
        let representative = vertex_sets.find(g.to_index(node.clone()));
        match component_map.get_mut(&representative) {
            Some(existing) => {
                existing.push(Arc::new(node));
            },
            None => {
                component_map.insert(representative, vec![Arc::new(node)]);
            }
        };
    }
    let mut components: Vec<Vec<Arc<N>>> = component_map.into_values().collect();
    components.sort_by_key(Vec::len);
    return components;
}

#[global_allocator]
static ALLOCATOR: LoggingAllocator = LoggingAllocator::with_allocator(System);

#[tokio::main]
async fn main() {
    simple_logging::log_to_file("./log.txt", LevelFilter::Trace).expect("Failed to configure file logging");
    info!("Looking for SVGs in {}", absolute(SVG_DIR.to_path_buf()).unwrap().to_string_lossy());
    info!("Writing output to {}", absolute(OUT_DIR.to_path_buf()).unwrap().to_string_lossy());
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {:?} pixels per tile", tile_size);
    ALLOCATOR.enable_logging();
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
    tasks.into_iter().for_each(|task| {
            task.add_to(&mut graph, &mut added_tasks);});
    clean_out_dir.await.expect("Failed to create or replace output directory");
    let futures: Vec<TaskResultFuture> = connected_components(&graph)
        .into_iter()
        .flatten()
        .map(|node| graph.node_weight(*node).unwrap().to_owned())
        .collect();
    join_all(futures.into_iter().map(|future| tokio::spawn(future.to_owned()))).await;
    info!("Finished after {} ns", start_time.elapsed().as_nanos());
    ALLOCATOR.disable_logging();
}