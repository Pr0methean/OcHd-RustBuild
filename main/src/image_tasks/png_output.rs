use std::os::unix::fs::symlink;
use std::path::{PathBuf};
use tiny_skia::Pixmap;

pub fn png_output(image: Pixmap, files: Vec<PathBuf>) -> Result<(), anyhow::Error> {
    let (first_file, extra_files) = files.split_first()
            .expect("Tried to write PNG to empty list of files");
    image.save_png(first_file)?;
    drop(image);
    for file in extra_files {
        symlink(first_file, file)?;
    }
    return Ok(());
}