#![feature(absolute_path)]
#![feature(arc_unwrap_or_clone)]
#![feature(const_type_id)]
#![feature(let_chains)]
#![feature(macro_metavar_expr)]
#![feature(const_trait_impl)]
#![feature(lazy_cell)]

use std::path::{absolute, PathBuf};
use std::time::Instant;

use log::{info, LevelFilter, warn};
use texture_base::material::Material;

use crate::image_tasks::task_spec::{FileOutputTaskSpec, METADATA_DIR, TaskGraphBuildingContext, TaskSpecTraits};

mod image_tasks;
mod texture_base;
mod materials;
#[cfg(not(any(test,clippy)))]
use std::env;
use std::fs;
use std::fs::create_dir_all;
use std::hint::unreachable_unchecked;
use std::ops::DerefMut;
use include_dir::{Dir, DirEntry};
use rayon::{scope_fifo, ThreadPoolBuilder, yield_local};
use tikv_jemallocator::Jemalloc;
use image_tasks::cloneable::{CloneableError};
use crate::image_tasks::png_output::{copy_in_to_out, ZIP};
use crate::image_tasks::prewarm_pixmap_pool;
use crate::image_tasks::repaint::prewarm_mask_pool;

#[cfg(not(any(test,clippy)))]
use once_cell::sync::Lazy;
use rayon::Yield::Executed;

const GRID_SIZE: u32 = 32;

#[cfg(not(any(test,clippy)))]
static ARGS: Lazy<Vec<String>> = Lazy::new(|| env::args().collect());

#[cfg(not(any(test,clippy)))]
static TILE_SIZE: Lazy<u32> = Lazy::new(||
        ARGS.get(1).expect("Usage: OcHd-RustBuild <tile-size> <iterations>").parse::<u32>()
            .expect("Tile size (first command-line argument) must be an integer"));

#[cfg(any(test,clippy))]
const TILE_SIZE: &u32 = &128;

#[cfg(not(any(test,clippy)))]
static ZOPFLI_ITERS: Lazy<u64> = Lazy::new(||
    ARGS.get(2).expect("Usage: OcHd-RustBuild <tile-size> <iterations>").parse::<u64>()
        .expect("Iterations (second command-line argument) must be an integer"));

#[cfg(any(test,clippy))]
const ZOPFLI_ITERS: &u64 = &15;

#[global_allocator]
static ALLOCATOR: Jemalloc = Jemalloc;

#[allow(unreachable_code)]
pub const fn debug_assert_unreachable<T>() -> T {
    #[cfg(debug_assertions)]
    unreachable!();
    unsafe {unreachable_unchecked()}
}

fn copy_metadata(source_dir: &Dir) {
    source_dir.entries().iter().for_each(
        |entry| {
            match entry {
                DirEntry::Dir(dir) => {
                    copy_metadata(dir);
                }
                DirEntry::File(file) => {
                    copy_in_to_out(file, file.path().to_string_lossy().into())
                        .expect("Failed to copy a file");
                }
            }
        }
    );
}

fn main() -> Result<(), CloneableError> {
    simple_logging::log_to_file("./log.txt", LevelFilter::Debug).expect("Failed to configure file logging");
    let out_dir = PathBuf::from("./out");
    let out_file = out_dir.join(format!("OcHD-{}x{}.zip", *TILE_SIZE, *TILE_SIZE));
    info!("Writing output to {}", absolute(&out_file)?.to_string_lossy());
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {} pixels per tile", tile_size);
    let mut cpus = num_cpus::get();
    if (cpus as u64 + 1).count_ones() <= 1 {
        warn!("Adjusting CPU count from {} to {}", cpus, cpus + 1);
        cpus += 1;
        // Compensate for missed CPU core on m7g.16xlarge
        ThreadPoolBuilder::new().num_threads(cpus).build_global()?;
    }
    info!("Rayon thread pool has {} threads", cpus);
    let start_time = Instant::now();
    rayon::join(
        || rayon::join(
        || {
            prewarm_pixmap_pool();
            prewarm_mask_pool();
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
        let mut large_tasks = Vec::with_capacity(out_tasks.len());
        let mut small_tasks = Vec::with_capacity(out_tasks.len());
        for task in out_tasks.into_iter() {
            let new_task = task.add_to(&mut ctx, tile_size);
            if tile_size > GRID_SIZE
                    && let FileOutputTaskSpec::PngOutput {base, .. } = task
                    && !base.is_grid_perfect(&mut ctx) {
                large_tasks.push(new_task);
            } else {
                small_tasks.push(new_task);
            }
        }
        drop(ctx);
        scope_fifo(move |scope| {
            for task in large_tasks {
                let name = task.to_string();
                scope.spawn_fifo(move |_| {
                    // Work around https://github.com/rayon-rs/rayon/issues/1064
                    while yield_local() == Some(Executed) {}
                    *task.into_result()
                        .unwrap_or_else(|err| panic!("Error running task {}: {:?}", name, err))
                });
            }
            for task in small_tasks {
                let name = task.to_string();
                scope.spawn_fifo(move |_| {
                    // Work around https://github.com/rayon-rs/rayon/issues/1064
                    while yield_local() == Some(Executed) {}
                    *task.into_result()
                        .unwrap_or_else(|err| panic!("Error running task {}: {:?}", name, err))
                });
            }
        });
    });
    let zip_contents = ZIP.lock()?.deref_mut().finish()
        .expect("Failed to finalize ZIP file").into_inner();
    info!("ZIP file size is {} bytes", zip_contents.len());
    fs::write(out_file.as_path(), zip_contents)?;
    info!("Finished after {} ns", start_time.elapsed().as_nanos());
    Ok(())
}
