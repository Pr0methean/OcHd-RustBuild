#![feature(absolute_path)]
#![feature(arc_unwrap_or_clone)]
#![feature(const_type_id)]
#![feature(let_chains)]
#![feature(macro_metavar_expr)]

use std::cmp::Ordering;
use std::collections::{HashMap};
use std::path::{absolute, PathBuf};
use std::time::Instant;

use itertools::Itertools;
use daggy::petgraph::visit::{EdgeRef, IntoNodeReferences, IntoEdgeReferences, NodeIndexable};
use daggy::petgraph::unionfind::UnionFind;
use log::{info, LevelFilter};
use daggy::petgraph::graph::{DefaultIx, NodeIndex};
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
use std::ops::{DerefMut};
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
    info!("Using {} pixels per tile", tile_size);
    let start_time = Instant::now();
    rayon::join(|| {
        create_dir_all(out_dir).expect("Failed to create output directory");
        copy_metadata(&METADATA_DIR);
    }, || {
        build_task_vector().into_par_iter()
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

struct ConnectedComponent<'a> {
    output_tasks: Vec<&'a CloneableLazyTask<()>>,
    total_tasks: usize,
    max_ref_count: usize
}

impl <'a> PartialEq<Self> for ConnectedComponent<'a> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl <'a> Eq for ConnectedComponent<'a> {}

impl <'a> PartialOrd<Self> for ConnectedComponent<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl <'a> Ord for ConnectedComponent<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Run connected components with widely-used tasks first, so that cloning their result
        // doesn't become a bottleneck. Also favor running small ones first, to free up memory for
        // the large ones.
        match self.max_ref_count.cmp(&other.max_ref_count) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.total_tasks.cmp(&other.total_tasks)
        }
    }
}

fn build_task_vector() -> Vec<CloneableLazyTask<()>> {
    let output_tasks = materials::ALL_MATERIALS.get_output_tasks();
    let mut output_task_ids = Vec::with_capacity(output_tasks.len());
    let mut ctx: TaskGraphBuildingContext<(), DefaultIx>
        = TaskGraphBuildingContext::new();
    for task in output_tasks.iter() {
        let (output_task_id, _) = task.add_to(&mut ctx);
        output_task_ids.push(output_task_id);
    }
    // Split the graph into weakly-connected components (WCCs, groups that don't share any subtasks).
    // Used to save memory by minimizing the number of WCCs caching their subtasks at once.
    let mut vertex_sets = UnionFind::new(ctx.graph.node_bound());
    for edge in ctx.graph.edge_references() {
        let (a, b) = (edge.source(), edge.target());

        // union the two vertices of the edge
        vertex_sets.union(a, b);
    }
    let mut component_map: HashMap<NodeIndex<DefaultIx>, ConnectedComponent>
        = HashMap::with_capacity(ctx.graph.node_bound());
    let labeling = vertex_sets.into_labeling();
    for (index, task) in ctx.graph.node_references() {
        let representative = labeling[index.index()];
        let ref_count = ctx.get_ref_count(task).unwrap();
        let component = match component_map.get_mut(&representative) {
            Some(existing) => existing,
            None => {
                let new_component = ConnectedComponent {
                    output_tasks: Vec::new(),
                    total_tasks: 0,
                    max_ref_count: 0,
                };
                component_map.insert(representative, new_component);
                component_map.get_mut(&representative).unwrap()
            }
        };
        component.total_tasks += 1;
        component.max_ref_count = component.max_ref_count.max(ref_count);
        if let TaskSpec::FileOutput(output_task) = task {
            let (_, output_future) = ctx.output_task_to_future_map.get(output_task).unwrap();
            component.output_tasks.push(output_future);
        }
    }
    let mut components: Vec<ConnectedComponent> = component_map.into_values().collect();
    components.sort();
    info!("Connected component sizes: {}", components.iter().map(|component| component.total_tasks).join(","));
    components
        .into_iter()
        .flat_map(|component| component.output_tasks)
        .map(CloneableLazyTask::to_owned)
        .collect()
}