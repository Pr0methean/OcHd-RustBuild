mod tasks;
mod image_tasks;
mod texture_base;
mod materials;


use std::env;


fn main() {
    let args: Vec<String> = env::args().collect();
    let tile_size: u32 = args[1]
        .parse::<u32>()
        .expect("Tile size must be an integer");
    /*
    let mut g = Graph::new();
    let svgs = fs::read_dir("svg").unwrap();
    */
    println!("Hello, world! I'll use {} pixels per tile", tile_size);
}