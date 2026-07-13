//! Stained Glass Filter
//!
//! A procedural post-processing effect that transforms an image into a stained glass window.
//! It uses a procedural Voronoi pattern (based on cell grids and hash functions) to partition
//! the screen into irregular "glass shards". Each shard is assigned the average or center
//! color of the underlying image, and thick, dark "leadings" (borders) are drawn between the shards.

use abrash_core::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static SOURCE_CACHE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// A simple procedural hash function to generate deterministic pseudo-random
/// values for a given 2D grid coordinate.
#[inline(always)]
fn hash_2d(x: i32, y: i32) -> f32 {
    let mut h = (x.wrapping_mul(374_761_393)) ^ (y.wrapping_mul(668_265_261));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    let h_f = (h ^ (h >> 16)) as f32;
    // Normalize to 0.0 .. 1.0
    (h_f.abs() / (std::i32::MAX as f32)).clamp(0.0, 1.0)
}

/// Applies a stained glass effect to the given framebuffer in-place.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `cell_size` - The approximate pixel size of each stained glass shard (e.g., 20.0).
/// * `border_thickness` - The thickness of the black "leading" lines between shards (e.g., 2.0).
/// * `border_color` - The ARGB color of the borders (usually black or dark grey).
///
/// # Panics
///
/// Panics if memory allocation for the temporary source cache fails.
pub fn apply_stained_glass(
    fb: &mut Framebuffer,
    cell_size: f32,
    border_thickness: f32,
    border_color: u32,
) {
    if cell_size <= 1.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    SOURCE_CACHE.with(|cache_ref| {
        let mut cache = cache_ref.borrow_mut();
        cache.clear();
        cache.reserve_exact(width * height);
        cache.extend_from_slice(fb.as_slice());

        let pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            // To satisfy Rayon, we must pass a slice (which is Sync) rather than the RefMut.
            let cache_slice: &[u32] = &cache;
            pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    process_row(
                        row,
                        cache_slice,
                        width,
                        y,
                        cell_size,
                        border_thickness,
                        border_color,
                    );
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            let cache_slice: &[u32] = &cache;
            for y in 0..height {
                let start = y * width;
                let end = start + width;
                let row = &mut pixels[start..end];
                process_row(
                    row,
                    cache_slice,
                    width,
                    y,
                    cell_size,
                    border_thickness,
                    border_color,
                );
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn process_row(
    row: &mut [u32],
    cache: &[u32],
    width: usize,
    y: usize,
    cell_size: f32,
    border_thickness: f32,
    border_color: u32,
) {
    let py = y as f32;
    let inv_cell_size = 1.0 / cell_size;
    let height = cache.len() / width;

    for x in 0..width {
        let px = x as f32;

        let grid_x = (px * inv_cell_size).floor() as i32;
        let grid_y = (py * inv_cell_size).floor() as i32;

        let mut min_dist = f32::MAX;
        let mut second_min_dist = f32::MAX;
        let mut closest_center_x = 0;
        let mut closest_center_y = 0;

        // Check the 3x3 neighborhood of cells
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cell_x = grid_x + dx;
                let cell_y = grid_y + dy;

                // Hash the cell coordinates to get a pseudo-random point within the cell
                let hx = hash_2d(cell_x, cell_y);
                let hy = hash_2d(cell_y, cell_x); // Swapped for different distribution

                let point_x = (cell_x as f32 + hx) * cell_size;
                let point_y = (cell_y as f32 + hy) * cell_size;

                let diff_x = px - point_x;
                let diff_y = py - point_y;
                let dist_sq = diff_x * diff_x + diff_y * diff_y;

                if dist_sq < min_dist {
                    second_min_dist = min_dist;
                    min_dist = dist_sq;
                    closest_center_x = point_x.max(0.0) as usize;
                    closest_center_y = point_y.max(0.0) as usize;
                } else if dist_sq < second_min_dist {
                    second_min_dist = dist_sq;
                }
            }
        }

        let dist = min_dist.sqrt();
        let second_dist = second_min_dist.sqrt();

        if second_dist - dist < border_thickness {
            row[x] = border_color;
        } else {
            // Clamp sample point to screen bounds
            let sample_x = closest_center_x.clamp(0, width.saturating_sub(1));
            let sample_y = closest_center_y.clamp(0, height.saturating_sub(1));

            // Safety: bounded by clamp above
            let source_color = unsafe { *cache.get_unchecked(sample_y * width + sample_x) };

            // Extract components to add some pseudo-random variation based on cell
            let h_val = hash_2d(closest_center_x as i32, closest_center_y as i32);
            let variation = (h_val * 40.0) as i32 - 20; // -20 to +20 brightness

            let a = source_color & 0xFF00_0000;
            let r = (((source_color >> 16) & 0xFF) as i32 + variation).clamp(0, 255) as u32;
            let g = (((source_color >> 8) & 0xFF) as i32 + variation).clamp(0, 255) as u32;
            let b = ((source_color & 0xFF) as i32 + variation).clamp(0, 255) as u32;

            row[x] = a | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stained_glass_basic() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF_FF0000);

        apply_stained_glass(&mut fb, 10.0, 1.0, 0xFF_000000);

        // Spot check - some pixels should be black (border), some should be roughly red
        let mut has_border = false;
        let mut has_color = false;

        for y in 0..32 {
            for x in 0..32 {
                let pixel = fb.get_pixel(x, y).unwrap();
                if pixel == 0xFF_000000 {
                    has_border = true;
                } else if pixel != 0xFF_000000 {
                    has_color = true;
                }
            }
        }

        assert!(has_border, "Expected to find border pixels");
        assert!(has_color, "Expected to find colored pixels inside cells");
    }
}
