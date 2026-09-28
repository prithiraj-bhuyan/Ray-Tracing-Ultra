use image::RgbImage;

use crate::camera::Camera;
use crate::render::{render_pixel, RenderOptions};
use crate::scene::Scene;
use crate::scheduler::{TileScheduler};

/// Renders tiles on scoped threads, assigning the next available tile to
/// each worker as it finishes. This balances uneven per-pixel work without
/// putting unsafe code in the renderer.
pub fn render_dynamic(
    scene: &Scene,
    camera: &Camera,
    opts: &RenderOptions,
    threads: usize,
    tile_size: u32,
) -> RgbImage {
    let scheduler = TileScheduler::new(opts.width, opts.height, tile_size as usize);
    let effective_threads = match threads{
        0 => 1,
        _ => threads
    };
    
    let scheduler_ref = &scheduler;
    std::thread::scope(|s| {
        for _ in 0..effective_threads {
            s.spawn(move || {
                while let Some(claim) = scheduler_ref.claim_tile() {
                    let bounds = claim.bounds();
                    let start_x = bounds[0];
                    let start_y = bounds[1];
                    let tile_width = bounds[2];
                    let tile_height = bounds[3];
                    // Match TileClaim::write's row-major order.
                    let mut rgb_arr = Vec::new();
                    for local_y in 0..tile_height {
                        let y = start_y + local_y;
                        for local_x in 0..tile_width {
                            let x = start_x + local_x;
                            let pixel = render_pixel(scene, camera, opts, x as u32, y as u32);
                            rgb_arr.push(pixel); 
                        }
                    }
                    claim.write(&rgb_arr);
                }
            });
        }
    });
    scheduler.into_image()
}
