use std::fs::create_dir_all;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use log::info;

use tiny_skia::Pixmap;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::task_spec::TaskResult;
use crate::image_tasks::task_spec::TaskResult::Empty;

#[instrument]
pub fn png_output(image: &Pixmap, files: &Vec<PathBuf>) -> TaskResult {
    let file_strings: Vec<String> = files.iter().map(|path| path.to_string_lossy().to_string()).collect();
    let files_string = file_strings.join(", ");
    drop(file_strings);
    info!("Starting task: write {}", files_string);
    let (first_file, extra_files) = files.split_first()
            .expect("Tried to write PNG to empty list of files");
    create_dir_all(first_file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
    image.save_png(first_file).map_err(|error| anyhoo!(error))?;
    for file in extra_files {
        create_dir_all(first_file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
        symlink(first_file, file).map_err(|error| anyhoo!(error))?;
    }
    info!("Finishing task: write {}", files_string);
    Empty {}
}