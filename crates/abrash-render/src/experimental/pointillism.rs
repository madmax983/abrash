//! Pointillism Post-Processing Filter
//!
//! A post-processing effect that converts the image into a pattern of colored dots,
//! simulating the Pointillism painting technique.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Fast procedural hash function for generating a pseudo-random number based on 2D coordinates
/// and a seed. This avoids the overhead of external random number generator libraries in tight loops.
#[inline(always)]
fn fast_hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = seed.wrapping_add(x.wrapping_mul(374_761_393));
    h = (h << 13) ^ h;
    h = h.wrapping_add(y.wrapping_mul(668_265_263));
    h = (h << 13) ^ h;
    h = h.wrapping_mul(1_274_126_177);

    // Normalize to 0.0 - 1.0
    (h as f32) / (u32::MAX as f32)
}

/// Applies a Pointillism stylization filter to the framebuffer.
///
/// Converts the image into a pattern of colored dots on a dark background.
/// Uses a highly parallelizable procedural hash with localized per-cell coordinate evaluation
/// to splat randomized overlapping dots without issuing traditional draw commands or using
/// heap-allocated sorting structures.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `dot_size` - The radius of the painted dots.
/// * `density` - Determines how densely the dots are packed (e.g., 1.0 is full density).
pub fn apply_pointillism(fb: &mut Framebuffer, dot_size: f32, density: f32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || dot_size <= 0.0 {
        return;
    }

    let cell_size = (dot_size / density.max(0.1)).max(1.0);
    let inv_cell_size = 1.0 / cell_size;
    let dot_radius_sq = dot_size * dot_size;

    thread_local! {
        static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    SOURCE_BUFFER.with(|buf| {
        let mut source_vec = buf.borrow_mut();
        let size = width * height;
        if source_vec.len() < size {
            source_vec.resize(size, 0);
        }

        let source_pixels = &mut source_vec[..size];
        source_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        // Define an off-white background color, like a canvas
        let canvas_color = 0xFF_F0_F0_EE;

        #[cfg(feature = "parallel")]
        let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            for (x, pixel) in row.iter_mut().enumerate() {
                let px = x as f32;
                let py = y as f32;

                // Determine which grid cell this pixel belongs to
                let cell_x = (px * inv_cell_size).floor() as i32;
                let cell_y = (py * inv_cell_size).floor() as i32;

                let mut best_dist_sq = f32::MAX;
                let mut best_color = canvas_color;

                // Look at neighboring cells (3x3 grid around current cell) to see if their
                // randomized dots overlap this pixel. This local evaluation enables the
                // procedural painter's algorithm without global sorting.
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = cell_x + dx;
                        let ny = cell_y + dy;

                        let hash_x = nx as u32;
                        let hash_y = ny as u32;

                        // Procedural offset for dot center within the cell
                        let offset_x = fast_hash(hash_x, hash_y, 0x1234) * cell_size;
                        let offset_y = fast_hash(hash_x, hash_y, 0x5678) * cell_size;

                        let dot_center_x = (nx as f32 * cell_size) + offset_x;
                        let dot_center_y = (ny as f32 * cell_size) + offset_y;

                        let dist_x = px - dot_center_x;
                        let dist_y = py - dot_center_y;
                        let dist_sq = dist_x * dist_x + dist_y * dist_y;

                        if dist_sq <= dot_radius_sq {
                            // "Painter's algorithm" approach for overlapping dots:
                            // we use a consistent depth ordering based on a third hash
                            // to determine which dot is on top if they overlap.
                            let _depth = fast_hash(hash_x, hash_y, 0x9ABC);

                            // But instead of actual depth sorting, we just take the nearest dot center
                            // to simulate round daubs of paint that don't perfectly overlap,
                            // or we can use depth if we wanted strict Z-ordering.
                            // For pointillism, distance to center creates nice Voronoi-like boundaries
                            // between overlapping dots, enhancing the painted look.

                            if dist_sq < best_dist_sq {
                                best_dist_sq = dist_sq;

                                // Sample the source color at the dot center to color the whole dot
                                let sample_x =
                                    (dot_center_x as i32).clamp(0, width as i32 - 1) as usize;
                                let sample_y =
                                    (dot_center_y as i32).clamp(0, height as i32 - 1) as usize;

                                best_color = source_pixels[sample_y * width + sample_x];
                            }
                        }
                    }
                }

                *pixel = best_color;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pointillism() {
        let mut fb = Framebuffer::new(100, 100).unwrap();

        // Fill half red, half blue to test color sampling
        let pixels = fb.as_mut_slice();
        for y in 0..100 {
            for x in 0..100 {
                if x < 50 {
                    pixels[y * 100 + x] = 0xFFFF_0000; // Red
                } else {
                    pixels[y * 100 + x] = 0xFF00_00FF; // Blue
                }
            }
        }

        apply_pointillism(&mut fb, 5.0, 1.0);

        let canvas_color = 0xFF_F0_F0_EE;

        // At least some pixels should be canvas background, some red, some blue.
        let mut has_red = false;
        let mut has_blue = false;
        let mut has_canvas = false;

        for &p in fb.as_slice() {
            if p == 0xFFFF_0000 {
                has_red = true;
            }
            if p == 0xFF00_00FF {
                has_blue = true;
            }
            if p == canvas_color {
                has_canvas = true;
            }
        }

        assert!(has_red, "Should retain some red paint dots");
        assert!(has_blue, "Should retain some blue paint dots");
        assert!(
            has_canvas,
            "Should expose some canvas background between dots"
        );
    }
}
