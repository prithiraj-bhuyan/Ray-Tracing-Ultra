//! Dynamic tile scheduling and the shared frame buffer.
//!
//! This is the only unsafe module in the renderer. Callers receive unique,
//! non-copyable tile claims and cannot choose buffer positions directly.
//! The full safety argument is in `README.md`.

use std::{assert_eq, cell::UnsafeCell, ops::Rem, sync::atomic::{ AtomicUsize, Ordering }};

use image::RgbImage;

struct FrameBuffer {t: Vec<UnsafeCell<[u8; 3]>>}

// SAFETY: The buffer and its cells are private to this module. Shared access
// can write pixels only through TileClaim::write, which consumes a claim.
// claim_tile atomically issues each in-range tile index at most once, and
// bounds maps distinct tile indices to disjoint pixel rectangles. Therefore
// concurrent claims cannot write the same UnsafeCell. The vector is fully
// allocated during construction and is never resized while claims exist.
unsafe impl Sync for FrameBuffer {}

pub(crate) struct TileScheduler {
    width: u32,
    height: u32,
    tile_size: usize,
    n_tiles_across: usize,
    n_tiles_down: usize,
    counter: AtomicUsize,
    buffer: FrameBuffer
}

impl TileScheduler {
    pub fn new(width: u32, height: u32, tile_size:usize) -> Self {
        let effective_tile_size = tile_size.max(1);
        let n_tiles_across = (width as usize).div_ceil(effective_tile_size);
        let n_tiles_down = (height as usize).div_ceil(effective_tile_size);
        let counter = AtomicUsize::new(0);
        let mut buffer= Vec::new();
        for _ in 0..height {
            for _ in 0..width {
                buffer.push(UnsafeCell::new([0, 0, 0]));
            }
        }
        TileScheduler{ width, height, tile_size:effective_tile_size, n_tiles_across, n_tiles_down, counter, buffer: FrameBuffer { t: buffer } }
    }

    pub(crate) fn total_tiles(&self) -> usize {
        self.n_tiles_across * self.n_tiles_down
    }

    pub fn claim_tile(&self) -> Option<TileClaim<'_>> {
        // The atomic update reserves one index only while it is in range.
        // Relaxed ordering suffices: the counter assigns work, not pixel data.
        let tile_idx = self.counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |counter| {
            if counter < self.total_tiles() {
                Some(counter + 1)
            }
            else {
                None
            }
        });
        return match tile_idx {
            Ok(n) => Some(TileClaim { tile_idx: n, tile_scheduler: self }),
            Err(_) => None
        }
    }

    pub fn into_image(self) -> RgbImage {
        // Consuming the scheduler requires all borrowed tile claims to be gone.
        let mut result = Vec::new();
        for arr in self.buffer.t {
            let rgb_arr = arr.into_inner();
            result.push(rgb_arr[0]);
            result.push(rgb_arr[1]);
            result.push(rgb_arr[2]);
        }
        RgbImage::from_raw(self.width, self.height, result).expect("frame-buffer length must match image dimensions")
    }
}

pub(crate) struct TileClaim<'a> {
    tile_idx: usize,
    tile_scheduler: &'a TileScheduler
}

impl<'a> TileClaim<'a> {
    pub(crate) fn bounds(&self) -> [usize; 4] {
        // Convert the flat tile index to a grid position, then clip edges.
        let tile_column = self.tile_idx.rem(self.tile_scheduler.n_tiles_across);
        let tile_row = self.tile_idx / self.tile_scheduler.n_tiles_across;
        let start_x = self.tile_scheduler.tile_size * tile_column;
        let start_y = self.tile_scheduler.tile_size * tile_row;
        let tile_width = self.tile_scheduler.tile_size.min(self.tile_scheduler.width as usize - start_x);
        let tile_height = self.tile_scheduler.tile_size.min(self.tile_scheduler.height as usize - start_y);
        [ start_x, start_y, tile_width, tile_height ]
    }

    pub(crate) fn write(self, slice: &[[u8; 3]]) {
        let bounds = self.bounds();
        let start_x = bounds[0];
        let start_y = bounds[1];
        let tile_width = bounds[2];
        let tile_height = bounds[3];
        let required_pixels = tile_width * tile_height;
        assert_eq!(slice.len(), required_pixels);
        for local_y in 0..tile_height {
            let y = start_y + local_y;
            for local_x in 0..tile_width {
                let x = start_x + local_x; 
                let local_pos = local_y * tile_width + local_x;
                let buffer_pos = y * self.tile_scheduler.width as usize + x;
                let cell = &self.tile_scheduler.buffer.t[buffer_pos];
                let raw_ptr = cell.get();
                // SAFETY: buffer_pos identifies a cell allocated for this
                // image; indexing above checks the bound. This tile claim is
                // unique and consumed by write, and different tiles have
                // disjoint pixels, so no other writer can access this cell
                // concurrently. The vector is not resized during writes.
                unsafe {
                    *raw_ptr = slice[local_pos];
                }
            }
        }
    }
}
