use image::RgbImage;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use crate::camera::Camera;
use crate::hittable::Hittable;
use crate::ray::Ray;
use crate::scene::Scene;
use crate::vec3::Vec3;

/// Parameters controlling a render, independent of the scene/camera and
/// independent of how the work is distributed across threads.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub max_depth: u32,
    /// Base seed for per-pixel RNG seeding (see `pixel_seed`). Two renders
    /// with the same `RenderOptions`, scene, and camera must always agree,
    /// regardless of thread count -- this seed is what makes that possible.
    pub seed: u64,
}

/// Deterministically derives a per-pixel RNG seed from the run's base seed
/// and the pixel's coordinates. Every renderer (sequential, parallel,
/// dynamic) must use this and only this to seed the RNG for pixel (x, y),
/// so that which thread computes a pixel never affects its result.
fn pixel_seed(opts: &RenderOptions, x: u32, y: u32) -> u64 {
    opts.seed.wrapping_add((y as u64) * (opts.width as u64) + (x as u64))
}

/// Recursively traces `ray` through `world`, returning the color seen along
/// it. Terminates recursion at `depth == 0` (contributing black, i.e. no
/// further light) or when the ray escapes to the background sky gradient.
pub fn ray_color(ray: &Ray, world: &dyn Hittable, depth: u32, rng: &mut SmallRng) -> Vec3 {
    if depth == 0 {
        return Vec3::new(0.0, 0.0, 0.0);
    }

    if let Some(hit) = world.hit(ray, 0.001, f64::INFINITY) {
        return match hit.material.scatter(ray, &hit, rng) {
            Some((attenuation, scattered)) => attenuation * ray_color(&scattered, world, depth - 1, rng),
            None => Vec3::new(0.0, 0.0, 0.0),
        };
    }

    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    Vec3::new(1.0, 1.0, 1.0) * (1.0 - t) + Vec3::new(0.5, 0.7, 1.0) * t
}

fn to_byte(channel: f64) -> u8 {
    // gamma-2 correction, as in Ray Tracing in One Weekend.
    let gamma_corrected = channel.max(0.0).sqrt().clamp(0.0, 0.999);
    (gamma_corrected * 256.0) as u8
}

/// Renders exactly one pixel, averaging `samples_per_pixel` jittered
/// camera samples. This is the single source of truth for "what a pixel's
/// value is" -- `render_sequential`, `render_parallel`, and
/// `render_dynamic` all call this and nothing else to compute a pixel, so
/// agreement between them reduces to "did every pixel get computed exactly
/// once".
pub fn render_pixel(scene: &Scene, camera: &Camera, opts: &RenderOptions, x: u32, y: u32) -> [u8; 3] {
    let mut rng = SmallRng::seed_from_u64(pixel_seed(opts, x, y));
    let mut color = Vec3::new(0.0, 0.0, 0.0);

    for _ in 0..opts.samples_per_pixel {
        let u = (x as f64 + rng.random::<f64>()) / (opts.width - 1) as f64;
        // Flip vertically: image row 0 is the top of the frame, but
        // Camera's `t` coordinate increases upward.
        let v = ((opts.height - 1 - y) as f64 + rng.random::<f64>()) / (opts.height - 1) as f64;
        let ray = camera.get_ray(u, v);
        color = color + ray_color(&ray, scene, opts.max_depth, &mut rng);
    }
    color = color / opts.samples_per_pixel as f64;

    [to_byte(color.x), to_byte(color.y), to_byte(color.z)]
}

/// The known-correct baseline renderer. `render_parallel` and
/// `render_dynamic` must reproduce this exactly, pixel for pixel.
pub fn render_sequential(scene: &Scene, camera: &Camera, opts: &RenderOptions) -> RgbImage {
    let mut img = RgbImage::new(opts.width, opts.height);
    for y in 0..opts.height {
        for x in 0..opts.width {
            let px = render_pixel(scene, camera, opts, x, y);
            img.put_pixel(x, y, image::Rgb(px));
        }
    }
    img
}
