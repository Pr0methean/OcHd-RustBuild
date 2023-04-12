use std::fs::create_dir_all;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use tiny_skia::Pixmap;
use tracing::instrument;

use crate::anyhoo;
use crate::image_tasks::task_spec::TaskResult;
use crate::image_tasks::task_spec::TaskResult::Empty;

#[instrument]
pub fn png_output<'a>(image: &Pixmap, files: &Vec<PathBuf>) -> TaskResult<'a> {
    let (first_file, extra_files) = files.split_first()
            .expect("Tried to write PNG to empty list of files");
    create_dir_all(first_file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
    image.save_png(first_file).map_err(|error| anyhoo!(error))?;
    for file in extra_files {
        create_dir_all(first_file.parent().unwrap()).map_err(|error| anyhoo!(error))?;
        symlink(first_file, file).map_err(|error| anyhoo!(error))?;
    }
    Empty {}
}