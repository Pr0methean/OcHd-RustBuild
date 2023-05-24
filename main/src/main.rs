#![feature(absolute_path)]
#![feature(arc_unwrap_or_clone)]
#![feature(const_type_id)]
#![feature(let_chains)]
#![feature(macro_metavar_expr)]

use std::cell::RefCell;
use std::collections::{HashMap};
use std::path::{absolute, PathBuf};
use std::time::Instant;

use itertools::Itertools;
use petgraph::visit::{EdgeRef, IntoNodeReferences, IntoEdgeReferences, NodeIndexable};
use fn_graph::daggy::petgraph::unionfind::UnionFind;
use log::{info, LevelFilter};
use petgraph::graph::{DefaultIx, NodeIndex};
use texture_base::material::Material;
use rayon::prelude::*;

use crate::image_tasks::task_spec::{CloneableLazyTask, TaskGraphBuildingContext, TaskSpec, TaskSpecTraits, METADATA_DIR, CloneableError};

mod image_tasks;
mod texture_base;
mod materials;
#[cfg(not(any(test,clippy)))]
use std::env;
use std::fs;
use std::fs::create_dir_all;
use std::ops::DerefMut;
use include_dir::{Dir, DirEntry};
use lazy_static::lazy_static;
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
static ALLOCATOR: Jemalloc = Jemalloc;

fn copy_metadata(source_dir: &Dir) {
    source_dir.entries().iter().for_each(
        |entry| {
            match entry {
                DirEntry::Dir(dir) => {
                    copy_metadata(dir);
                }
                DirEntry::File(file) => {
                    copy_in_to_out(file, file.path()).expect("Failed to copy a file");
                }
            }
        }
    );
}

fn main() -> Result<(), CloneableError> {
    simple_logging::log_to_file("./log.txt", LevelFilter::Trace).expect("Failed to configure file logging");
    let out_dir = PathBuf::from("./out");
    let out_file = out_dir.join(format!("OcHD-{}x{}.zip", *TILE_SIZE, *TILE_SIZE));
    info!("Writing output to {}", absolute(&out_file)?.to_string_lossy());
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {:?} pixels per tile", tile_size);
    let start_time = Instant::now();
    rayon::join(|| {
        create_dir_all(out_dir).expect("Failed to create output directory");
        copy_metadata(&METADATA_DIR);
    }, || {
        let components = build_task_vector();
        components.into_par_iter()
            .map(|task| task.into_result())
            .for_each(|result| {
                result.expect("Error running a task");
            });
    });
    let mut zip = ZIP.lock()?;
    let zip_writer = zip.deref_mut();
    let zip_contents = zip_writer.finish()
        .expect("Failed to finalize ZIP file").into_inner();
    info!("ZIP file size is {} bytes", zip_contents.len());
    fs::write(out_file.as_path(), zip_contents)?;
    info!("Finished after {} ns", start_time.elapsed().as_nanos());
    Ok(())
}

fn build_task_vector() -> Vec<CloneableLazyTask<()>> {
    let output_tasks = materials::ALL_MATERIALS.get_output_tasks();
    let ctx: RefCell<TaskGraphBuildingContext<(), DefaultIx>>
        = RefCell::new(TaskGraphBuildingContext::new());
    for task in output_tasks {
        task.add_to(&ctx);
    }
    // Split the graph into weakly-connected components (WCCs, groups that don't share any subtasks).
    // Used to save memory by minimizing the number of WCCs caching their subtasks at once.
    // Adapted from https://docs.rs/petgraph/latest/src/petgraph/algo/mod.rs.html#87-102.
    let graph = &ctx.borrow().graph;
    let mut vertex_sets = UnionFind::new(graph.node_bound());
    for edge in graph.edge_references() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        vertex_sets.union(a, b);
    }
    let mut component_map: HashMap<NodeIndex<DefaultIx>, Vec<Option<&CloneableLazyTask<()>>>>
        = HashMap::with_capacity(graph.node_bound());
    let task_to_future_map = &ctx.borrow().output_task_to_future_map;
    let labeling = vertex_sets.into_labeling();
    for (index, task) in graph.node_references() {
        let representative = labeling[index.index()];
        let future_if_output = if let TaskSpec::FileOutput(sink_task_spec) = task {
            let (_, future) = task_to_future_map.get(sink_task_spec)
                .unwrap_or_else(|| panic!("Missing output_task_to_future_map entry for {}", sink_task_spec));
            Some(future)
        } else {
            None
        };
        match component_map.get_mut(&representative) {
            Some(existing) => {
                existing.push(future_if_output);
            },
            None => {
                let mut vec = Vec::with_capacity(1024);
                vec.push(future_if_output);
                component_map.insert(representative, vec);
            }
        };
    }
    // Run small WCCs first so that their data can leave the heap before the big WCCs run
    let mut components: Vec<Vec<Option<&CloneableLazyTask<()>>>> = component_map.into_values().collect();
    components.sort_by_key(Vec::len);
    info!("Connected component sizes: {}", components.iter().map(Vec::len).join(","));
    components.into_iter().flatten().flatten().map(CloneableLazyTask::to_owned).collect()
}