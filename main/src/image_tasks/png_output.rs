use std::os::unix::fs::symlink;
use std::path::Path;
use anyhow::anyhow;
use tiny_skia::Pixmap;

fn png_output(image: Pixmap, mut files: Box<dyn Iterator<Item=&Path>>) -> Result<(), anyhow::Error> {
    let first_file = files.next()
        .ok_or(anyhow!("Tried to write PNG to empty list of files"))?;
    image.save_png(first_file)?;
    drop(image);
    for file in files {
        symlink(first_file, file)?;
    }
    return Ok(());
}