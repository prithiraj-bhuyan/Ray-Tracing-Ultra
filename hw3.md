# Homework 3: A Safe, Parallel Ray Tracer

## Overview

In this assignment, you will take a working single-threaded ray tracer and
make it fast using safe, and then carefully encapsulated unsafe,
concurrency. Ray tracing renders a 3D scene by, for every output pixel,
firing one or more rays from a camera into the scene and computing the color
seen along each ray — including recursive bounces off reflective and
refractive surfaces. Every pixel's color is independent of every other
pixel's, which makes ray tracing an embarrassingly parallel
workload: both an ideal setting to practice safe concurrency, and a
well-motivated place to use `unsafe` for performance, as
you did for interpreter dispatch in Homework 2.

You will:

1. Extend the provided scene representation with one new shape and one new
   material, using traits (Phase 1).
2. Parallelize rendering across threads using only safe Rust, and measure
   the speedup (Phase 2).
3. Replace static work assignment with dynamic load balancing.  Load
   can be implemented with a global `Mutex`, but performance will be poor
   due to contention.  You will use a small, carefully encapsulated
   `unsafe` abstraction to do better.  Write a safety argument for
   whichever unsafe design you choose, in the same Hazard/Invariant/
   Establish/Maintain style as Homework 2 (Phase 3).

**How much code is this?** Less than you might expect, and in fact less than
Homework 2. Our reference solution adds fewer than 300 lines total across
all three phases, and over half of that is Phase 3's `unsafe` module —
which is small by nature.  A well-designed unsafe abstraction here
is a compact, carefully-reasoned-about kernel, not a large
one. If you find yourself writing hundreds of lines for Phase 2, or for
Phase 1's new shape and material, you are probably overcomplicating
something — go back to how `Sphere`/`Lambertian`/`Metal` are written and
match their scale.

As with Homework 1/2, you will be using primarily safe abstractions
with a small, well-motivated unsafe core, justified by a concrete
performance argument and a careful safety argument.

## Starter Code

The starter project is a Rust Cargo project containing a complete,
correct, **single-threaded** ray tracer, in the style of Shirley et al.'s
*Ray Tracing in One Weekend*. It includes:

- `Vec3` and `Ray` types with the arithmetic you need.
- The `Hittable` trait, `HitRecord`, and a `Scene` container (a flat list of
  scene objects that is itself `Hittable`).
- The `Material` trait.
- `Sphere` (a `Hittable`), and `Lambertian` and `Metal` (both `Material`).
- `Camera`, including anti-aliasing (multiple jittered samples per pixel).
- `render_sequential`, a reference single-threaded renderer producing an
  in-memory image buffer.
- PNG output via the `image` crate.
- A CLI driver in `src/main.rs`.
- Fixed test scenes and their test harness in `tests/`.

You do not need to modify `render_sequential`, `Camera`, `Sphere`,
`Lambertian`, or `Metal`, but you will read them closely, since Phase 1
follows their pattern and Phases 2-3 call into the rendering logic they
contain.

## Required Types and API

Your implementation must provide the following public declarations with these
names and signatures, because the provided tests call them directly. You
may add any private helper functions, modules, or data structures you find
useful.

| Item | Required for |
|---|---|
| `struct Triangle { p0: Vec3, p1: Vec3, p2: Vec3, material: Arc<dyn Material> }`, implementing `Hittable` | Phase 1 |
| `struct Dielectric { refraction_index: f64 }`, implementing `Material` | Phase 1 |
| `fn refract(uv: Vec3, n: Vec3, etai_over_etat: f64) -> Vec3` | Phase 1 |
| `fn reflectance(cosine: f64, refraction_index: f64) -> f64` | Phase 1 |
| `fn render_parallel(scene: &Scene, camera: &Camera, opts: &RenderOptions, threads: usize) -> RgbImage` | Phase 2 |
| `fn render_dynamic(scene: &Scene, camera: &Camera, opts: &RenderOptions, threads: usize, tile_size: u32) -> RgbImage` | Phase 3 |

`RgbImage` is the `image` crate's image buffer type; `RenderOptions` (already
defined in the starter code) bundles image width/height, samples per pixel,
max recursion depth, and a random seed. `Scene` and `Camera` are also
already defined in the starter code.

## Determinism and seeding — read this before you start Phase 2

Every render must be **bit-for-bit reproducible**: the same scene and the same
`RenderOptions` must always produce the same output image, and
`render_parallel` and `render_dynamic` must produce **exactly** the same image
as `render_sequential` given the same `RenderOptions`, regardless of how many
threads you use.

To make this achievable, seed each pixel's random number generator
deterministically from its coordinates and the run's base seed, never from
a shared global RNG or the system clock:

```rust
let mut rng = SmallRng::seed_from_u64(
    opts.seed.wrapping_add(y as u64 * opts.width as u64 + x as u64)
);
```

`render_sequential` in the starter code already does this — read it and
follow the same scheme. As long as each pixel's full set of samples is
computed by a single thread, in the same order the sequential version would
use, your parallel renderers will match it exactly; there is no need for a
floating-point tolerance anywhere in this assignment.

This determinism requirement helps catch bugs: a `render_parallel`
or `render_dynamic` that fails to reproduce `render_sequential`'s output is
almost always a symptom of a data race. Treat any mismatch as a race to be
fixed.

## Phase 1: Extend the scene

Add `Triangle` (a `Hittable`) and `Dielectric` (a `Material`) as specified in
the table above, following the conventions already established by `Sphere`,
`Lambertian`, and `Metal` (in particular, `HitRecord`'s `front_face`
convention: the outward normal should be flipped to face the incoming ray,
exactly as `Sphere::hit` already does).

`Dielectric` models glass-like refraction. For a ray hitting the surface at
angle `cosine` from the normal, use Schlick's approximation to decide the
probability of reflection versus refraction, and Snell's law (via `refract`)
to compute the refracted direction; fall back to reflection when the angle
exceeds the critical angle for total internal reflection. `refract` and
`reflectance` are required as standalone public functions (not just inlined
into `scatter`) because the tests below check them directly, independent of
any randomness in the reflect-vs-refract choice.

**Tests provided in `tests/hittable_tests.rs`** check `Triangle::hit` and
the `refract`/`reflectance` helpers, for example:

- A ray fired at the centroid of a triangle with vertices `(0,0,-1)`,
  `(1,0,-1)`, `(0,1,-1)` must report a hit at `t = 1.0`, hit point
  `(1/3, 1/3, -1)`, and outward normal `(0, 0, 1)`.
- A ray aimed outside the triangle's bounds, and a ray parallel to the
  triangle's plane, must both report no hit (not a panic).
- `refract(-n, n, etai_over_etat)` (a ray along the normal) must return `-n`
  unchanged, for any `etai_over_etat` — no bending at normal incidence.
- `reflectance(1.0, 1.5)` (normal incidence, glass) must equal `0.04`
  (Schlick's `r0` at `cosine = 1.0`), within `1e-9`.

These tests use approximate equality to account for small variations in
floating point computations.

You do not need a scene-rendering test for Phase 1 — these are unit
tests on your new types, in the same spirit as Homework 1/2's function
tests.

## Phase 2: Safe parallelization

Implement `render_parallel`, splitting the image into disjoint row or tile
ranges (e.g. with `chunks_mut` on the output buffer) and rendering each
range on its own thread via `std::thread::scope`. This should require no
`unsafe` at all — the slice API gives you a compile-time proof that the
chunks are disjoint.

Requirements:

- `render_parallel` must exactly match `render_sequential`'s output (see
  "Determinism and seeding" above) for every thread count from 1 up to at
  least `std::thread::available_parallelism()`. This is checked by
  `tests/render_tests.rs`, which renders the small fixed scene from
  `src/scenes.rs` at low resolution with several different thread counts
  and asserts pixel-for-pixel equality against `render_sequential`.
- Report a measured wall-clock speedup in your `README.md`: render the
  larger benchmark scene in `src/scenes.rs` (`benchmark_scene()`) at the
  resolution/sample count given there, once single-threaded and once with
  your parallel renderer at your machine's full core count, and report both
  times, the core count, and the resulting speedup.

## Phase 3: Dynamic load balancing and one motivated unsafe module

Static chunking by row or tile assigns each thread a fixed, equal share of the image
up front. But per-pixel cost varies a lot in a ray tracer: pixels that hit
`Dielectric` or `Metal` surfaces recurse through several bounces, while
pixels that miss everything terminate almost immediately. A scene with
reflective/refractive objects clustered in one region of the image will
leave some threads idle near the end of a static-chunked render while
others are still working through expensive rows.

Replace static chunking with **dynamic tile assignment**: divide the image
into many more tiles than you have threads (e.g. 16x16-pixel tiles), and
have each thread repeatedly claim the next unclaimed tile and render it,
until no tiles remain. This is a standard work-queue/work-stealing pattern.

Here is the design problem this creates: with static chunking, `chunks_mut`
let the compiler *prove* your row ranges were disjoint, so writing to them
concurrently was safe. With tiles claimed dynamically at runtime, no such
static proof is available — from the compiler's point of view, any thread
could in principle be handed any tile, so it cannot verify two threads never
get overlapping regions. A fully safe implementation would need a `Mutex`
around the whole output buffer, which reintroduces exactly the contention
you were trying to eliminate.

You must design a small abstraction — for example, a `FrameBuffer` type
wrapping the pixel storage in an `UnsafeCell` (or a raw pointer) with a
manually-implemented `unsafe impl Sync`, exposing a safe method such as
`fn write_tile(&self, tile: TileId, pixels: &[u8])` — whose soundness
depends on an invariant maintained by your tile scheduler: **each tile index
is handed to exactly one thread**, and every tile's pixel range lies within
the buffer. The exact interface is up to you (this mirrors Homework 2, where
the threaded-dispatch design was likewise not fully prescribed); the point
is that the unsafe code lives in one small, identifiable module, behind a
safe API, motivated by a specific performance problem.

Requirements:

- `render_dynamic` must exactly match `render_sequential`'s output, exactly
  as `render_parallel` did in Phase 2, and checked by the
  `tests/render_tests.rs` harness.
- Report a measured speedup of `render_dynamic` over `render_parallel` on a
  scene specifically designed to have uneven per-pixel cost (provided as
  `benchmark_scene_uneven()` in `src/scenes.rs`, which clusters glass and
  metal objects in one corner of the frame). Static chunking should visibly
  underperform dynamic tiling on this scene; if it doesn't, look at your
  tile size and thread count.
- A **safety argument** in `README.md`, in the same Hazard/Invariant/
  Establish/Maintain structure used in `hw2-threaded/README.md`. Identify
  the Rust module that implements your unsafe abstraction, explain how it
  is motivated by the load-balancing performance goal above (not by
  dispatch, as in Homework 2), and address:
  - **Hazards**: what could go wrong if the abstraction's invariant were
    violated — for instance, two threads handed the same tile, or a tile
    index outside the buffer's bounds.
  - **Invariant**: the precise condition that rules those hazards out.
  - **Establish**: how your tile scheduler's construction guarantees every
    valid tile index is produced, and produced exactly once, before any
    thread starts claiming tiles.
  - **Maintain**: how every operation on your abstraction preserves that
    invariant, or how the type system (e.g., consuming a tile index so it
    cannot be claimed twice) makes violating it impossible.

**Important**: two points about the safety argument:
  - It must be *specific* to your implementation, as in Homework 2.  Repeating
  the four headings with a generic argument that does not connect them to your
  actual code is not sufficient.
  - It must be *isolated* to a single, small module within your implementation.
  The safety boundary must be the boundary of that module--specify which module
  in your safety argument.  The module should be small, and in particular
  should not include the `render_dynamic` method--that method should be
  external and not part of the safety argument.  You must argue that it is
  impossible to violate the safety of the abstraction from outside that single
  module, even if for example `render_dynamic` was written incorrectly.

## Testing and benchmarking

```bash
cargo test              # correctness: Phase 1 unit tests + Phase 2/3 exact-match tests
# manual/demo rendering: <mode> is sequential | parallel | dynamic,
# <scene> is small | benchmark | uneven, [tile_size] only matters for dynamic.
cargo run --release -- <mode> <scene> <threads> <output.png> [tile_size]
```

`cargo test` is what the autograder runs; it does not measure performance.
Report your speedup measurements in `README.md` as described in Phases 2
and 3 — re-run `cargo run --release` yourself to gather these numbers, the
test suite will not do it for you.

## Point Allocation

- 15% — Phase 1: `Triangle` and `Dielectric`, passing the provided unit tests
- 20% — Phase 2: `render_parallel` correctness (exact match, all thread counts)
- 15% — Phase 2: measured speedup, reported with methodology
- 20% — Phase 3: `render_dynamic` correctness (exact match, all thread counts) and measured improvement over Phase 2 on the uneven-cost scene
- 20% — Phase 3: the unsafe `FrameBuffer`/scheduler design and its safety argument
- 10% — Extra credit, as specified below

## Extra Credit

We will offer 10% extra credit to two implementations from the class:

- The fastest render time on a large, fixed benchmark scene at a resolution
  and sample count we will write.
- The best measured *scaling efficiency* (speedup ÷ thread count) on the
  uneven-cost benchmark scene — this rewards good load balancing
  specifically, not just raw throughput on a fast machine.

## Submission

Turn in a zip file of your solution, including the `README.md` safety
argument and speedup measurements, via Gradescope.

Your submission should include:

- Your Rust implementation.
- Any additional test scenes or benchmarks you added.
- The `README.md` safety argument and performance measurements.
- Any other source files required to build and run your solution.

## Reference

- Peter Shirley, Trevor D. Black, Steve Hollasch. *Ray Tracing in One
  Weekend*. <https://raytracing.github.io/books/RayTracingInOneWeekend.html>
  — the primary reference for the math and structure of this ray tracer;
  the starter code follows its conventions closely.
- The Rust `std::thread`, `std::thread::scope`, and `std::sync` module
  documentation.
