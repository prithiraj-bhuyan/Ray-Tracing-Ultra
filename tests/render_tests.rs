//! Phase 2/3 correctness tests: `render_parallel` and `render_dynamic` must
//! reproduce `render_sequential`'s output exactly, pixel for pixel, no
//! matter how many threads (or what tile size) are used. Any mismatch is
//! almost certainly a data race, not acceptable floating-point noise --
//! see "Determinism and seeding" in hw3.md.

use hw3_raytracer::scenes::small_test_scene;
use hw3_raytracer::{render_dynamic, render_parallel, render_sequential};

fn assert_images_equal(expected: &hw3_raytracer::RgbImage, actual: &hw3_raytracer::RgbImage, label: &str) {
    assert_eq!(expected.dimensions(), actual.dimensions(), "{label}: dimensions differ");
    assert_eq!(
        expected.as_raw(),
        actual.as_raw(),
        "{label}: pixel data differs from render_sequential -- this almost always means a data race"
    );
}

#[test]
fn parallel_matches_sequential_at_various_thread_counts() {
    let setup = small_test_scene();
    let reference = render_sequential(&setup.scene, &setup.camera, &setup.opts);

    for threads in [1usize, 2, 3, 4, 8] {
        let actual = render_parallel(&setup.scene, &setup.camera, &setup.opts, threads);
        assert_images_equal(&reference, &actual, &format!("render_parallel(threads={threads})"));
    }
}

#[test]
fn dynamic_matches_sequential_at_various_thread_and_tile_sizes() {
    let setup = small_test_scene();
    let reference = render_sequential(&setup.scene, &setup.camera, &setup.opts);

    for threads in [1usize, 2, 4, 8] {
        for tile_size in [8u32, 16, 33] {
            let actual = render_dynamic(&setup.scene, &setup.camera, &setup.opts, threads, tile_size);
            assert_images_equal(
                &reference,
                &actual,
                &format!("render_dynamic(threads={threads}, tile_size={tile_size})"),
            );
        }
    }
}

#[test]
fn repeated_parallel_renders_are_themselves_reproducible() {
    // A cheap extra safety net: even without a sequential baseline, two
    // runs of the same parallel renderer must agree with each other.
    let setup = small_test_scene();
    let first = render_parallel(&setup.scene, &setup.camera, &setup.opts, 6);
    let second = render_parallel(&setup.scene, &setup.camera, &setup.opts, 6);
    assert_images_equal(&first, &second, "render_parallel run twice");
}
