#![feature(absolute_path)]
#![feature(arc_unwrap_or_clone)]
#![feature(const_type_id)]
#![feature(let_chains)]
#![feature(macro_metavar_expr)]
#![feature(const_trait_impl)]
use std::path::{absolute, PathBuf};
use std::time::Instant;

use log::{info, LevelFilter};
use texture_base::material::Material;
use rayon::prelude::*;

use crate::image_tasks::task_spec::{TaskGraphBuildingContext, TaskSpecTraits, METADATA_DIR, CloneableError};

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
use rayon::current_num_threads;
use tikv_jemallocator::Jemalloc;
use crate::image_tasks::png_output::{copy_in_to_out, prewarm_png_buffer_pool, ZIP};
use crate::image_tasks::prewarm_pixmap_pool;
use crate::image_tasks::repaint::prewarm_mask_pool;

#[cfg(not(any(test,clippy)))]
lazy_static! {
    static ref ARGS: Vec<String> = env::args().collect();
    static ref TILE_SIZE: u32 = {
        ARGS.get(1).expect("Usage: OcHd-RustBuild <tile-size>").parse::<u32>()
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
    simple_logging::log_to_file("./log.txt", LevelFilter::Info).expect("Failed to configure file logging");
    let out_dir = PathBuf::from("./out");
    let out_file = out_dir.join(format!("OcHD-{}x{}.zip", *TILE_SIZE, *TILE_SIZE));
    info!("Writing output to {}", absolute(&out_file)?.to_string_lossy());
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {} pixels per tile", tile_size);
    info!("Rayon thread pool has {} threads", current_num_threads());
    let start_time = Instant::now();
    rayon::join(
        || rayon::join(
        || {
            prewarm_pixmap_pool();
            prewarm_mask_pool();
            prewarm_png_buffer_pool();
            info!("Caches prewarmed");
            create_dir_all(out_dir).expect("Failed to create output directory");
            info!("Output directory built");
        },
        || {
            copy_metadata(&METADATA_DIR);
            info!("Metadata copied");
        }),
    || {
        let mut ctx: TaskGraphBuildingContext = TaskGraphBuildingContext::new();
        let out_tasks = materials::ALL_MATERIALS.get_output_tasks();
        let mut planned_tasks = Vec::with_capacity(out_tasks.len());
        for task in out_tasks {
            planned_tasks.push(task.add_to(&mut ctx));
        }
        drop(ctx);
        planned_tasks.into_par_iter().for_each(move |task| {
            let name = task.name.to_owned();
            task.into_result()
                .unwrap_or_else(|err| panic!("Error running task {}: {:?}", name, err));
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
