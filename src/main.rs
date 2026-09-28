use std::env;
use std::time::Instant;

use hw3_raytracer::scenes::{benchmark_scene, benchmark_scene_uneven, small_test_scene};
use hw3_raytracer::{render_dynamic, render_parallel, render_sequential};

fn usage() -> ! {
    eprintln!(
        "usage: hw3-raytracer <mode> <scene> <threads> <output.png> [tile_size]\n\
         \x20 mode:  sequential | parallel | dynamic\n\
         \x20 scene: small | benchmark | uneven\n\
         \x20 tile_size: only used by 'dynamic' mode (default 16)"
    );
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        usage();
    }
    let mode = args[1].as_str();
    let scene_name = args[2].as_str();
    let threads: usize = args[3].parse().unwrap_or_else(|_| usage());
    let output = &args[4];
    let tile_size: u32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(16);

    let setup = match scene_name {
        "small" => small_test_scene(),
        "benchmark" => benchmark_scene(),
        "uneven" => benchmark_scene_uneven(),
        _ => usage(),
    };

    let start = Instant::now();
    let image = match mode {
        "sequential" => render_sequential(&setup.scene, &setup.camera, &setup.opts),
        "parallel" => render_parallel(&setup.scene, &setup.camera, &setup.opts, threads),
        "dynamic" => render_dynamic(&setup.scene, &setup.camera, &setup.opts, threads, tile_size),
        _ => usage(),
    };
    let elapsed = start.elapsed();

    image.save(output).expect("failed to write output image");
    println!(
        "{mode} scene={scene_name} threads={threads} {}x{} samples={} depth={} -> {output} in {:.3}s",
        setup.opts.width,
        setup.opts.height,
        setup.opts.samples_per_pixel,
        setup.opts.max_depth,
        elapsed.as_secs_f64()
    );
}
