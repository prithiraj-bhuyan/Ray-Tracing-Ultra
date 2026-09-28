# Homework 3: A Safe, Parallel Ray Tracer

See `hw3.md` for the full assignment description. Quick start:

```bash
cargo test                                          # run the provided tests
cargo run --release -- sequential small 1 out.png   # render the small test scene
```

`cargo run --release -- <mode> <scene> <threads> <output.png> [tile_size]`,
where `<mode>` is `sequential`, `parallel`, or `dynamic`, and `<scene>` is
`small`, `benchmark`, or `uneven` (see `src/scenes.rs`).

## Phase 1 and Phase 2 status

Phase 1 implements `Triangle::hit`, `Dielectric::scatter`, `refract`, and
`reflectance`. All seven provided `hittable_tests` passed.

Phase 2 implements `render_parallel` with scoped threads and disjoint mutable
row chunks. Every pixel is computed with the shared `render_pixel` function,
which seeds its RNG from the pixel coordinates and the render seed. The
provided render tests confirmed pixel-for-pixel equality with
`render_sequential` at the tested thread counts and reproducibility across
repeated parallel runs. No `unsafe` is used in Phase 2.

## Phase 2 benchmark

I measured the `benchmark` scene on a MacBook Air with 10 logical CPUs. Both
modes used a release build and the scene's fixed settings: 640 × 360 pixels,
32 samples per pixel, maximum depth 16, and seed 1234. The sequential mode
used one thread; the parallel mode used 10 threads. I ran each mode three
times and used the median of the program's reported wall-clock render times.
The reported timing measures rendering and excludes PNG saving and Cargo
compilation.

| Mode | Threads | Render times (s) | Median (s) |
| --- | ---: | --- | ---: |
| Sequential | 1 | 4.990, 4.871, 4.851 | 4.871 |
| Parallel | 10 | 1.320, 1.284, 1.329 | 1.320 |

Speedup: **4.871 / 1.320 = 3.69×**.

## Phase 3: dynamic tiles

The static renderer assigns whole groups of rows to threads up front. On the
`uneven` scene, some rows take longer because rays bounce through the glass
and metal objects. The dynamic renderer instead divides the image into tiles.
Each worker takes another tile when it finishes its current one, so a worker
does not have to wait for another thread's expensive rows.

The load-balancing code is in `src/dynamic.rs`, but the unsafe boundary is
only `src/scheduler.rs`. The latter owns the tile counter and the frame
buffer. The renderer can request a claim, read its bounds, and submit pixel
colors through a safe method; it cannot choose an arbitrary tile index or
write directly into the shared buffer.

## Phase 3 benchmark

I compared the static `parallel` renderer with `dynamic` on the provided
`uneven` scene, using a release build and 10 threads on the same MacBook Air
as above. The scene settings were 480 × 320 pixels, 24 samples per pixel,
and maximum depth 50. Dynamic tiles were 32 × 32 pixels. I ran each mode
five times and used the median of the program's reported render times;
compilation and PNG saving are excluded.

| Mode | Tile size | Render times (s) | Median (s) |
| --- | ---: | --- | ---: |
| Parallel (static rows) | — | 0.345, 0.343, 0.344, 0.344, 0.346 | 0.344 |
| Dynamic | 32 | 0.307, 0.305, 0.296, 0.300, 0.360 | 0.305 |

Speedup on this scene: **0.344 / 0.305 = 1.13×**. The slower 0.360 s dynamic
run is included, not discarded. I also tried 16-pixel tiles; their five-run
median was 0.327 s, so I used 32-pixel tiles for this comparison. The
parallel and dynamic PNGs matched byte-for-byte in this benchmark, and the
provided render tests passed.

## Phase 3 safety argument

**Hazard.** The frame buffer contains `UnsafeCell<[u8; 3]>` pixels and is
shared across threads via `unsafe impl Sync`. If two threads could write the
same cell at once, that would be a data race. An invalid tile could also
produce an out-of-bounds pixel position. These are the obligations behind
the unsafe pointer write in `TileClaim::write`.

**Invariant.** Let `N = n_tiles_across * n_tiles_down`, let `next` be the
atomic counter's current value, and let `Issued` be the set of tile indices
reserved by successful calls to `claim_tile`. At every point:

1. `0 <= next <= N` and `Issued = {0, 1, ..., next - 1}`. Each index in
   `Issued` was reserved exactly once; no index outside it was reserved.
2. For each issued index, there is at most one associated `TileClaim` (or
   call to its consuming `write`) in progress. Once that claim is used or
   dropped, the index cannot be issued again. No caller can manufacture or
   duplicate a claim through the safe API.
3. Every issued tile maps to pixels within the fixed `width * height` frame
   buffer, and the pixel sets of any two distinct tiles are disjoint. The
   buffer's storage is not resized while claims may use it.

This condition is sufficient for safety: any two concurrent writes must
come from distinct claims. They cannot have the same tile index by (1) and
(2), and distinct tiles cannot touch the same pixel by (3). Every indexed
cell also exists by (3). This is an *at-most-once* write guarantee, not a
promise that a caller will actually write every claimed tile. Dropping a
claim could make an incomplete image, but cannot make the buffer unsafe.

**Establish.** `TileScheduler::new` clamps the tile size to at least one,
computes the tile grid, allocates one `UnsafeCell` per image pixel, and sets
`next` to zero. Thus `Issued` is empty and the first two clauses hold.
`TileClaim::bounds` uses the tile index's quotient and remainder to select a
row and column of the grid, then trims right-edge and bottom-edge tiles to
the image dimensions. That gives in-bounds, non-overlapping rectangles for
all indices `0..N`, establishing the third clause before workers start.

**Maintain.** `claim_tile` uses one atomic `fetch_update`. If `next < N`,
exactly one successful caller receives the old value `next`, and the counter
advances to `next + 1`; that adds precisely the next unused index to
`Issued`. If `next == N`, no update occurs and it returns `None`. The
counter never hands out an index twice or advances past `N`. Relaxed atomic
ordering is enough for this unique-number allocation; the code does not
use the counter to publish pixel data.

`TileClaim` has private fields and is neither `Clone` nor `Copy`, so safe
code outside `scheduler.rs` cannot fabricate or duplicate it. `bounds`
only reads its fixed rectangle. `write(self, pixels)` consumes the claim,
checks that the input contains exactly the tile's number of pixels, and
writes only those cells; a second write with that claim is impossible.
Dropping a claim also cannot create another one. Finally,
`into_image(self)` consumes the scheduler; Rust will not allow it while
claims still borrow the scheduler. These are all of the safe operations
exposed by the module, and each preserves the invariant even if
`render_dynamic` uses them incorrectly.
