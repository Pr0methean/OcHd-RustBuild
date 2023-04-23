#![feature(const_trait_impl)]
#![feature(const_type_id)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(ptr_metadata)]
#![feature(async_closure)]
#![feature(try_trait_v2_residual)]
#![feature(try_trait_v2)]
#![feature(absolute_path)]
#![feature(result_option_inspect)]
#![feature(let_chains)]
#![feature(hash_set_entry)]
#![feature(once_cell_try)]
#![feature(allocator_api)]
#![feature(poll_ready)]
#![feature(arc_unwrap_or_clone)]
#![feature(lazy_cell)]
#![feature(concat_idents)]
#![feature(macro_metavar_expr)]
#![feature(const_io_structs)]
use std::collections::{HashMap};
use std::path::{absolute, PathBuf};
use std::time::Instant;


use petgraph::visit::{EdgeRef, IntoNodeReferences, IntoEdgeReferences, NodeIndexable};
use fn_graph::daggy::petgraph::unionfind::UnionFind;
use log::{info, LevelFilter};
use logging_allocator::LoggingAllocator;
use petgraph::graph::{DefaultIx, NodeIndex};
use texture_base::material::Material;
use rayon::prelude::*;

use crate::image_tasks::task_spec::{SVG_DIR, CloneableLazyTask, TaskGraphBuildingContext, TaskSpec, TaskSpecTraits, METADATA_DIR};

mod image_tasks;
mod texture_base;
mod materials;
#[cfg(not(any(test,clippy)))]
use std::env;
use std::fs;
use std::ops::DerefMut;
use lazy_static::lazy_static;
use pathdiff::diff_paths;
use tikv_jemallocator::Jemalloc;
use crate::image_tasks::png_output::{copy_in_to_out, ZIP};

#[cfg(not(any(test,clippy)))]
lazy_static! {
    static ref TILE_SIZE: u32 = {
        let args: Vec<String> = env::args().collect();
        args.get(1).expect("Usage: OcHd-RustBuild <tile-size>").parse::<u32>()
            .expect("Tile size (first command-line argument) must be an integer")
    };
}

#[cfg(any(test,clippy))]
lazy_static! {
    static ref TILE_SIZE: u32 = 128;
}

#[global_allocator]
static ALLOCATOR: LoggingAllocator<Jemalloc>
    = LoggingAllocator::with_allocator(Jemalloc);

fn copy_metadata(source_path: &PathBuf) {
    fs::read_dir(source_path).expect("Failed to read metadata directory").for_each(
        |entry| {
            let entry = entry.expect("Failed to read a DirEntry");
            let file_type = entry.file_type().expect("Failed to get a file type");
            let path = entry.path();
            if file_type.is_file() {
                let destination = diff_paths(&path, *METADATA_DIR)
                    .expect("Got a DirEntry that wasn't in METADATA_DIR");
                copy_in_to_out(path, destination).expect("Failed to copy a file");
            } else if file_type.is_dir() {
                copy_metadata(&path);
            }
        }
    );
}

#[tokio::main]
async fn main() {
    simple_logging::log_to_file("./log.txt", LevelFilter::Trace).expect("Failed to configure file logging");
    ALLOCATOR.enable_logging();
    let out_dir = PathBuf::from("./out");
    let out_file = out_dir.join(format!("OcHD-{}x{}.zip", *TILE_SIZE, *TILE_SIZE));
    info!("Looking for SVGs in {}", absolute(SVG_DIR.to_path_buf()).unwrap().to_string_lossy());
    info!("Writing output to {}", absolute(&out_file).unwrap().to_string_lossy());
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {:?} pixels per tile", tile_size);
    let start_time = Instant::now();

    let copy_metadata = tokio::spawn(async {
        create_dir_all(out_dir);
        copy_metadata(&(METADATA_DIR.to_path_buf()));
    });
    let output_tasks = materials::ALL_MATERIALS.get_output_tasks();
    let mut output_task_ids = Vec::with_capacity(1024);
    let mut ctx: TaskGraphBuildingContext<(), DefaultIx>
        = TaskGraphBuildingContext::new();
    for task in output_tasks.iter() {
        let (output_task_id, _) = task.add_to(&mut ctx);
        output_task_ids.push(output_task_id);
    }
    // Split the graph into weakly-connected components (WCCs, groups that don't share any subtasks).
    // Used to save memory by minimizing the number of WCCs caching their subtasks at once.
    // Adapted from https://docs.rs/petgraph/latest/src/petgraph/algo/mod.rs.html#87-102.
    let mut vertex_sets = UnionFind::new(ctx.graph.node_bound());
    for edge in ctx.graph.edge_references() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        vertex_sets.union(a, b);
    }
    let mut component_map: HashMap<NodeIndex<DefaultIx>, Vec<CloneableLazyTask<()>>> = HashMap::new();
    for (index, task) in ctx.graph.node_references() {
        let representative = vertex_sets.find(index);
        let (_, future) = match task {
            TaskSpec::FileOutput(sink_task_spec) => {
                ctx.output_task_to_future_map.get(&sink_task_spec).unwrap()
            }
            _ => continue
        };
        match component_map.get_mut(&representative) {
            Some(existing) => {
                existing.push(future.to_owned());
            },
            None => {
                let mut vec = Vec::with_capacity(1024);
                vec.push(future.to_owned());
                component_map.insert(representative, vec);
            }
        };
    }
    info!("Done with task graph");
    drop(output_task_ids);
    drop(ctx);
    drop(output_tasks);

    // Run small WCCs first so that their data can leave the heap before the big WCCs run
    let mut components: Vec<Vec<CloneableLazyTask<()>>> = component_map.into_values().collect();
    components.sort_by_key(Vec::len);
    let components: Vec<CloneableLazyTask<()>> = components.into_iter().flatten().collect();
    info!("Starting tasks");
    components.into_par_iter()
        .map(|task| task.into_result())
        .for_each(|result| {
            result.expect("Error running a task");
        });
    copy_metadata.await.expect("Failed to copy metadata");
    let mut zip = ZIP.lock().expect("Failed to lock ZIP buffer");
    let zip_writer = zip.deref_mut();
    let zip_contents = zip_writer.finish()
        .expect("Failed to finalize ZIP file").into_inner();
    info!("ZIP file size is {} bytes", zip_contents.len());
    fs::write(out_file.as_path(), zip_contents).expect("Failed to write ZIP file");
    info!("Finished after {} ns", start_time.elapsed().as_nanos());
    ALLOCATOR.disable_logging();
}