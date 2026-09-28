//! Fixed scenes shared by the CLI, the correctness tests, and the
//! benchmarks, so everyone renders the same thing. You do not need to
//! modify this file for Phases 2-3; Phase 1 only needs to make `Triangle`
//! and `Dielectric` available for use here (already done below).

use std::sync::Arc;

use crate::camera::Camera;
use crate::material::{Dielectric, Lambertian, Material, Metal};
use crate::render::RenderOptions;
use crate::scene::Scene;
use crate::sphere::Sphere;
use crate::triangle::Triangle;
use crate::vec3::Vec3;

pub struct SceneSetup {
    pub scene: Scene,
    pub camera: Camera,
    pub opts: RenderOptions,
}

/// A tiny, fast scene used by the correctness tests in
/// `tests/render_tests.rs`. Includes a sphere, a triangle, and all three
/// material kinds, so a mismatch anywhere in the rendering path shows up.
pub fn small_test_scene() -> SceneSetup {
    let mut scene = Scene::new();

    let ground: Arc<dyn Material> = Arc::new(Lambertian::new(Vec3::new(0.5, 0.5, 0.5)));
    scene.add(Arc::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0, ground)));

    let diffuse: Arc<dyn Material> = Arc::new(Lambertian::new(Vec3::new(0.7, 0.2, 0.2)));
    scene.add(Arc::new(Sphere::new(Vec3::new(-1.0, 0.0, -1.0), 0.5, diffuse)));

    let metal: Arc<dyn Material> = Arc::new(Metal::new(Vec3::new(0.8, 0.8, 0.8), 0.1));
    scene.add(Arc::new(Sphere::new(Vec3::new(1.0, 0.0, -1.0), 0.5, metal)));

    let glass: Arc<dyn Material> = Arc::new(Dielectric::new(1.5));
    scene.add(Arc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, glass)));

    let triangle_material: Arc<dyn Material> = Arc::new(Lambertian::new(Vec3::new(0.2, 0.4, 0.8)));
    scene.add(Arc::new(Triangle::new(
        Vec3::new(-0.6, -0.5, 0.4),
        Vec3::new(0.6, -0.5, 0.4),
        Vec3::new(0.0, 0.5, 0.1),
        triangle_material,
    )));

    let width = 96;
    let height = 72;
    let camera = Camera::new(
        Vec3::new(0.0, 0.6, 3.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        50.0,
        width as f64 / height as f64,
    );

    let opts = RenderOptions {
        width,
        height,
        samples_per_pixel: 16,
        max_depth: 8,
        seed: 42,
    };

    SceneSetup { scene, camera, opts }
}

/// A moderate-size scene with roughly evenly distributed cost, for
/// measuring Phase 2's general parallel speedup.
pub fn benchmark_scene() -> SceneSetup {
    let mut scene = Scene::new();

    let ground: Arc<dyn Material> = Arc::new(Lambertian::new(Vec3::new(0.5, 0.5, 0.5)));
    scene.add(Arc::new(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, ground)));

    // A deterministic grid of small spheres with varied materials, spread
    // evenly across the frame so no thread's row-band is systematically
    // more expensive than another's.
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        // A tiny deterministic xorshift, only used to place/color scene
        // objects at scene-construction time (single-threaded, before any
        // rendering starts) -- not part of the per-pixel RNG contract.
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };

    for a in -6..6 {
        for b in -6..6 {
            let center = Vec3::new(a as f64 + 0.9 * next(), 0.2, b as f64 + 0.9 * next());
            if (center - Vec3::new(4.0, 0.2, 0.0)).length() <= 0.9 {
                continue;
            }
            let choice = next();
            let material: Arc<dyn Material> = if choice < 0.6 {
                Arc::new(Lambertian::new(Vec3::new(next() * next(), next() * next(), next() * next())))
            } else if choice < 0.9 {
                Arc::new(Metal::new(
                    Vec3::new(0.5 * (1.0 + next()), 0.5 * (1.0 + next()), 0.5 * (1.0 + next())),
                    0.5 * next(),
                ))
            } else {
                Arc::new(Dielectric::new(1.5))
            };
            scene.add(Arc::new(Sphere::new(center, 0.2, material)));
        }
    }

    let glass: Arc<dyn Material> = Arc::new(Dielectric::new(1.5));
    scene.add(Arc::new(Sphere::new(Vec3::new(0.0, 1.0, 0.0), 1.0, glass)));
    let diffuse: Arc<dyn Material> = Arc::new(Lambertian::new(Vec3::new(0.4, 0.2, 0.1)));
    scene.add(Arc::new(Sphere::new(Vec3::new(-4.0, 1.0, 0.0), 1.0, diffuse)));
    let metal: Arc<dyn Material> = Arc::new(Metal::new(Vec3::new(0.7, 0.6, 0.5), 0.0));
    scene.add(Arc::new(Sphere::new(Vec3::new(4.0, 1.0, 0.0), 1.0, metal)));

    let width = 640;
    let height = 360;
    let camera = Camera::new(
        Vec3::new(13.0, 2.0, 3.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        20.0,
        width as f64 / height as f64,
    );

    let opts = RenderOptions {
        width,
        height,
        samples_per_pixel: 32,
        max_depth: 16,
        seed: 1234,
    };

    SceneSetup { scene, camera, opts }
}

/// A scene deliberately built so per-pixel cost is *not* evenly
/// distributed: a dense cluster of reflective/refractive spheres sits in
/// the top band of the frame (many inter-reflections, deep recursion),
/// while the rest of the frame is a nearly-empty ground plane (rays miss
/// or bounce once). `render_parallel`'s static row-chunking gives the
/// thread(s) covering the top band far more work than the rest, while
/// `render_dynamic`'s tile scheduler spreads that cost across all threads
/// over the course of the render. Used to measure Phase 3's improvement
/// over Phase 2.
pub fn benchmark_scene_uneven() -> SceneSetup {
    let mut scene = Scene::new();

    let ground: Arc<dyn Material> = Arc::new(Lambertian::new(Vec3::new(0.5, 0.5, 0.5)));
    scene.add(Arc::new(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, ground)));

    // A tightly packed cluster of small metal/glass spheres, high up (in
    // both world Y and, given the camera below, the top rows of the
    // image), so rays into this region bounce among each other many times.
    let spacing = 0.42;
    let half = 5;
    for i in -half..=half {
        for j in -half..=half {
            let center = Vec3::new(i as f64 * spacing, 2.6, j as f64 * spacing - 1.0);
            let material: Arc<dyn Material> = if (i + j) % 2 == 0 {
                Arc::new(Metal::new(Vec3::new(0.9, 0.9, 0.95), 0.02))
            } else {
                Arc::new(Dielectric::new(1.5))
            };
            scene.add(Arc::new(Sphere::new(center, 0.2, material)));
        }
    }

    let width = 480;
    let height = 320;
    let camera = Camera::new(
        Vec3::new(0.0, 1.6, 10.0),
        Vec3::new(0.0, 1.6, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        40.0,
        width as f64 / height as f64,
    );

    let opts = RenderOptions {
        width,
        height,
        samples_per_pixel: 24,
        max_depth: 50,
        seed: 777,
    };

    SceneSetup { scene, camera, opts }
}
