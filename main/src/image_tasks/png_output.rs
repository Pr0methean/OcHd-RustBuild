use std::os::unix::fs::symlink;
use std::path::{PathBuf};
use std::sync::Arc;
use tiny_skia::Pixmap;
use crate::anyhoo;
use crate::image_tasks::task_spec::TaskResult;
use crate::image_tasks::task_spec::TaskResult::Empty;

pub fn png_output(image: Pixmap, files: &Vec<PathBuf>) -> TaskResult {
    let (first_file, extra_files) = files.split_first()
            .expect("Tried to write PNG to empty list of files");
    image.save_png(first_file).map_err(|error| anyhoo!(error))?;
    drop(image);
    for file in extra_files {
        symlink(first_file, file).map_err(|error| anyhoo!(error))?;
    }
    return Empty {};
}