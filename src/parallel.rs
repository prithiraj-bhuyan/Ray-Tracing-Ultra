use image::{RgbImage};

use crate::camera::Camera;
use crate::render::{render_pixel, RenderOptions};
use crate::scene::Scene;

/// Renders disjoint groups of rows on scoped threads using only safe Rust.
/// Each worker owns a separate mutable slice of the image buffer.
pub fn render_parallel(scene: &Scene, camera: &Camera, opts: &RenderOptions, threads: usize) -> RgbImage {
    let mut img = RgbImage::new(opts.width, opts.height);
    if opts.width == 0 || opts.height == 0 {
        return img;
    }
    let img_bytes: &mut [u8] = img.as_mut();
    let row_bytes = opts.width as usize * 3;
    let effective_threads = match threads{
        0 => 1,
        _ => threads
    };
    let row_per_thread = (opts.height as usize).div_ceil(effective_threads);
    let bytes_per_thread = row_per_thread * row_bytes;
    // Chunk by complete rows so no two workers can access the same pixel.
    let slices = img_bytes.chunks_mut(bytes_per_thread);

    std::thread::scope(|s| {
        for (idx, slice) in slices.enumerate() {
            let start_row = (idx * row_per_thread) as u32;
            s.spawn(move || {
                for (local_row_index, byte_slice) in slice.chunks_mut(row_bytes).enumerate() {
                    let y = start_row + local_row_index as u32;
                    for x in 0..opts.width {
                        let px = render_pixel(&scene, &camera, &opts, x, y);
                        let offset = x as usize * 3;
                        byte_slice[offset] = px[0];
                        byte_slice[offset + 1] = px[1];
                        byte_slice[offset + 2] = px[2];
                    }
                }
            });       
        }
    });
    img
}
