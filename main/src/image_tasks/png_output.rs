use std::os::unix::fs::symlink;
use std::path::Path;
use anyhow::anyhow;
use tiny_skia::Pixmap;

pub fn png_output(image: Pixmap, files: &Vec<&Path>) -> Result<(), anyhow::Error> {
    let mut files_iter = files.iter();
    let first_file = files_iter.next()
        .ok_or(anyhow!("Tried to write PNG to empty list of files"))?;
    image.save_png(first_file)?;
    drop(image);
    for file in files_iter {
        symlink(first_file, file)?;
    }
    return Ok(());
}