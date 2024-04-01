#![feature(absolute_path)]
#![feature(const_type_id)]
#![feature(let_chains)]
#![feature(macro_metavar_expr)]
#![feature(const_trait_impl)]
#![feature(lazy_cell)]
#![feature(async_closure)]
#![feature(future_join)]

use std::path::{absolute, PathBuf};
use std::time::{Duration, Instant};

use log::{info, warn};
use texture_base::material::Material;
use tokio::runtime::{Builder, Handle};

use crate::image_tasks::task_spec::{TaskGraphBuildingContext, METADATA_DIR, TaskSpecTraits, FileOutputTaskSpec};

mod image_tasks;
mod materials;
mod texture_base;
mod u8set;

use crate::image_tasks::png_output::{copy_in_to_out, ZIP};
use crate::image_tasks::prewarm_pixmap_pool;
use crate::image_tasks::repaint::prewarm_mask_pool;
use futures_util::FutureExt;
use image_tasks::cloneable::CloneableError;
use include_dir::{Dir, DirEntry};
#[cfg(not(any(test, clippy)))]
use once_cell::sync::Lazy;
#[cfg(not(any(test, clippy)))]
use std::env;
use std::fs;
use std::fs::{create_dir_all, File};
use std::hint::unreachable_unchecked;
use std::ops::DerefMut;
use std::thread::available_parallelism;

use tikv_jemallocator::Jemalloc;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

use tracing_subscriber::fmt::format::FmtSpan;

const GRID_SIZE: u32 = 32;

#[cfg(not(any(test, clippy)))]
static ARGS: Lazy<Vec<String>> = Lazy::new(|| env::args().collect());

#[cfg(not(any(test, clippy)))]
static TILE_SIZE: Lazy<u32> = Lazy::new(|| {
    ARGS.get(1)
        .expect("Usage: OcHd-RustBuild <tile-size>")
        .parse::<u32>()
        .expect("Tile size (first command-line argument) must be an integer")
});

#[cfg(any(test, clippy))]
const TILE_SIZE: &u32 = &128;

#[global_allocator]
static ALLOCATOR: Jemalloc = Jemalloc;

#[allow(unreachable_code)]
#[allow(unused_variables)]
#[inline(always)]
pub const fn debug_assert_unreachable(msg: &'static str) -> ! {
    if cfg!(debug_assertions) {
        panic!("{}", msg);
    }
    unsafe { unreachable_unchecked() }
}

fn copy_metadata(source_dir: &Dir) {
    source_dir.entries().iter().for_each(|entry| match entry {
        DirEntry::Dir(dir) => {
            copy_metadata(dir);
        }
        DirEntry::File(file) => {
            copy_in_to_out(file, file.path().to_string_lossy().into())
                .expect("Failed to copy a file");
        }
    });
}

const MIN_METRICS_INTERVAL: Duration = Duration::from_secs(5);

fn main() -> Result<(), CloneableError> {
    tracing_subscriber::fmt()
        .with_writer(File::create("./log.txt")?)
        .with_span_events(FmtSpan::ACTIVE)
        .init();
    let out_dir = PathBuf::from("./out");
    let out_file = out_dir.join(format!("OcHD-{}x{}.zip", *TILE_SIZE, *TILE_SIZE));
    info!(
        "Writing output to {}",
        absolute(&out_file)?.to_string_lossy()
    );
    let tile_size: u32 = *TILE_SIZE;
    info!("Using {} pixels per tile", tile_size);
    let mut runtime = Builder::new_multi_thread();
    runtime.enable_time();
    match available_parallelism() {
        Ok(parallelism) => {
            let adjusted_parallelism = parallelism.get() + 1;
            if adjusted_parallelism.count_ones() <= 1 {
                warn!(
                    "Adjusting CPU count from {} to {}",
                    parallelism, adjusted_parallelism
                );
                // Compensate for missed CPU core on m7g.16xlarge
                runtime.worker_threads(adjusted_parallelism);
            } else {
                info!("Rayon thread pool has {} threads", parallelism);
            }
        }
        Err(e) => warn!("Unable to get available parallelism: {}", e),
    }
    let runtime = runtime.build()?;
    runtime.spawn(async move {
        loop {
            sleep(MIN_METRICS_INTERVAL).await;
            let m = Handle::current().metrics();
            macro_rules! log_metric {
                ($metrics:expr, $metric:ident) => {
                    info!("{:30}: {:5}", stringify!($metric), $metrics.$metric());
                };
            }
            macro_rules! log_metric_per_worker {
                ($metrics:expr, $metric:ident) => {
                    info!(
                        "{:30}: {:?}",
                        stringify!($metric),
                        (0..$metrics.num_workers())
                            .map(|i| $metrics.$metric(i))
                            .collect::<Vec<_>>()
                    );
                };
            }
            log_metric!(m, active_tasks_count);
            log_metric!(m, blocking_queue_depth);
            log_metric!(m, budget_forced_yield_count);
            log_metric!(m, injection_queue_depth);
            log_metric!(m, num_blocking_threads);
            log_metric!(m, num_idle_blocking_threads);
            log_metric!(m, remote_schedule_count);
            log_metric_per_worker!(m, worker_local_queue_depth);
            log_metric_per_worker!(m, worker_local_schedule_count);
            log_metric_per_worker!(m, worker_mean_poll_time);
            log_metric_per_worker!(m, worker_noop_count);
            log_metric_per_worker!(m, worker_overflow_count);
            log_metric_per_worker!(m, worker_park_count);
            log_metric_per_worker!(m, worker_poll_count);
            log_metric_per_worker!(m, worker_steal_count);
            log_metric_per_worker!(m, worker_steal_operations);
            log_metric_per_worker!(m, worker_total_busy_duration);
            if let Ok(dump) = timeout(Duration::from_secs(2), Handle::current().dump()).await {
                for (i, task) in dump.tasks().iter().enumerate() {
                    let trace = task.trace();
                    println!("TASK {i}:");
                    println!("{trace}\n");
                }
            }
        }
    });
    let start_time = Instant::now();
    let handle = runtime.handle();
    let _ = handle.enter();
    handle.block_on(async {
        let mut task_futures = JoinSet::new();
        task_futures.spawn(async {
            prewarm_pixmap_pool();
            prewarm_mask_pool();
            info!("Caches prewarmed");
            create_dir_all(out_dir).expect("Failed to create output directory");
            info!("Output directory built");
            copy_metadata(&METADATA_DIR);
            info!("Metadata copied");
        });
        let mut ctx: TaskGraphBuildingContext = TaskGraphBuildingContext::new();
        let out_tasks = materials::ALL_MATERIALS.get_output_tasks();
        let mut small_tasks = Vec::with_capacity(out_tasks.len());
        for task in out_tasks.into_vec().into_iter() {
            let small = match task {
                FileOutputTaskSpec::PngOutput { ref base, .. } =>
                    tile_size > GRID_SIZE && base.is_grid_perfect(&mut ctx),
                FileOutputTaskSpec::Copy { .. } => true
            };
            if small {
                small_tasks.push(task);
            } else {
                task_futures.build_task().name(&task.to_string()).spawn(task.add_to(&mut ctx, tile_size).map(drop)).unwrap();
            }
        }
        info!("All large output tasks added to graph");
        small_tasks.into_iter().for_each(|task| {
            task_futures.spawn(task.add_to(&mut ctx, tile_size).map(drop));
        });
        drop(ctx);
        info!("All small output tasks added to graph");
        remove_finished(&mut task_futures);
        while !task_futures.is_empty() {
            task_futures.join_next().await;
            remove_finished(&mut task_futures);
        }
    });
    drop(runtime); // Aborts any background tasks
    let zip_contents = ZIP
        .lock()
        .deref_mut()
        .finish()
        .expect("Failed to finalize ZIP file")
        .into_inner();
    info!("ZIP file size is {} bytes", zip_contents.len());
    fs::write(out_file.as_path(), zip_contents)?;
    info!("Finished after {} ns", start_time.elapsed().as_nanos());
    Ok(())
}

fn remove_finished<T: 'static>(task_futures: &mut JoinSet<T>) {
    while task_futures.try_join_next().is_some() {
        info!("try_join_next received a finished task");
    }
}
