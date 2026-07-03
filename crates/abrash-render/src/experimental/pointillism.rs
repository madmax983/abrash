//! Pointillism Filter
//!
//! A post-processing effect that simulates pointillism (painting with small, distinct dots).
//! This uses a procedural painter's algorithm to splat localized, randomized overlapping dots
//! without needing expensive per-frame sorting or rendering traditional 3D primitives.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the pointillism effect
#[derive(Clone, Copy, Debug)]
pub struct PointillismConfig {
    /// The maximum radius of a splatted dot.
    pub max_radius: f32,
    /// Density modifier to determine how many overlapping passes to make.
    pub density: u32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            max_radius: 4.0,
            density: 3,
        }
    }
}

/// A procedural hash function to generate stable pseudo-random values for a specific cell coordinate.
#[inline(always)]
fn cell_hash(x: i32, y: i32, layer: u32, seed: u32) -> f32 {
    let mut state = seed
        .wrapping_add((x as u32).wrapping_mul(374_761_393))
        .wrapping_add((y as u32).wrapping_mul(668_265_263))
        .wrapping_add(layer.wrapping_mul(2_269_547_737));

    // PCG-lite style mixing
    state = (state ^ (state >> 16)).wrapping_mul(0x7FEB_352D);
    state = (state ^ (state >> 15)).wrapping_mul(0x846C_A68B);
    state ^= state >> 16;

    // Return float in [0.0, 1.0)
    (state as f32) / (u32::MAX as f32)
}

/// Applies the procedural pointillism filter.
///
/// It treats the screen as a grid, where each cell can spawn a dot that splats its color.
/// Multiple shifted layers are evaluated to ensure complete coverage and organic overlap.
///
/// # Panics
///
/// Panics if a new framebuffer allocation fails when handling zero-dimension early returns.
#[must_use]
pub fn apply_pointillism(src_fb: &Framebuffer, config: PointillismConfig) -> Framebuffer {
    let width = src_fb.width();
    let height = src_fb.height();

    if width == 0 || height == 0 {
        let mut empty = Framebuffer::new(width, height).unwrap();
        empty.clear(0);
        return empty;
    }

    let mut out_fb = Framebuffer::new(width, height).unwrap();
    // Default to a dark canvas in case dots don't fully cover (though density should handle this)
    out_fb.clear(0xFF_11_11_11);

    let cell_size = (config.max_radius * 1.5).max(2.0) as i32;
    let max_radius_sq = config.max_radius * config.max_radius;
    let half_cell = cell_size as f32 * 0.5;

    let src_pixels = src_fb.as_slice();
    let out_pixels = out_fb.as_mut_slice();

    // To properly support parallel iteration (where we iterate over output pixels and query
    // nearby dots), we use a gather approach instead of a scatter approach.
    // Scatter: For each dot, draw to pixels (requires sorting or atomic locks).
    // Gather: For each pixel, query nearby cells to see if their dots overlap this pixel.
    // We pick the "top-most" dot based on some procedural Z-order (layer + random Z).

    #[cfg(feature = "parallel")]
    let iter = out_pixels.par_iter_mut().enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = out_pixels.iter_mut().enumerate();

    let seed = 0xABCD_1234;

    iter.for_each(|(idx, out_pixel)| {
        let px = (idx as u32 % width) as i32;
        let py = (idx as u32 / width) as i32;

        let mut best_z = -1.0;
        let mut best_color = 0xFF_11_11_11;

        // Evaluate nearby cells across all layers
        for layer in 0..config.density {
            // Shift the grid for each layer to create organic overlaps
            let offset_x = (cell_hash(layer as i32, 0, 0, seed) * cell_size as f32) as i32;
            let offset_y = (cell_hash(0, layer as i32, 1, seed) * cell_size as f32) as i32;

            // To find which cell this pixel belongs to in the shifted grid:
            let shifted_px = px + offset_x;
            let shifted_py = py + offset_y;

            let cell_x = shifted_px / cell_size;
            let cell_y = shifted_py / cell_size;

            // Because dots can be larger than cells (max_radius > cell_size),
            // a pixel might be covered by a dot in an adjacent cell.
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let cx = cell_x + dx;
                    let cy = cell_y + dy;

                    // Procedurally generate the dot for this cell
                    let h1 = cell_hash(cx, cy, layer, seed);
                    let h2 = cell_hash(cx, cy, layer, seed.wrapping_add(1));
                    let h3 = cell_hash(cx, cy, layer, seed.wrapping_add(2));

                    // Jitter the dot center within the cell
                    let dot_cx = (cx * cell_size) as f32 + half_cell + (h1 * 2.0 - 1.0) * half_cell;
                    let dot_cy = (cy * cell_size) as f32 + half_cell + (h2 * 2.0 - 1.0) * half_cell;

                    // Un-shift back to screen space
                    let screen_dot_cx = dot_cx - offset_x as f32;
                    let screen_dot_cy = dot_cy - offset_y as f32;

                    // Check if this pixel is inside the dot
                    let dist_x = px as f32 - screen_dot_cx;
                    let dist_y = py as f32 - screen_dot_cy;
                    let dist_sq = dist_x * dist_x + dist_y * dist_y;

                    // Jitter the radius slightly per dot
                    let dot_radius_sq = max_radius_sq * (0.6 + 0.4 * h3);

                    if dist_sq <= dot_radius_sq {
                        // The Z-order is a combination of the layer (higher layers draw on top)
                        // plus a small random offset so dots within the same layer interleave organically.
                        let h4 = cell_hash(cx, cy, layer, seed.wrapping_add(3));
                        let z = (layer as f32) + h4;

                        if z > best_z {
                            best_z = z;

                            // Sample the underlying image at the dot's center
                            let sample_x =
                                (screen_dot_cx as i32).clamp(0, width as i32 - 1) as usize;
                            let sample_y =
                                (screen_dot_cy as i32).clamp(0, height as i32 - 1) as usize;
                            let sample_idx = sample_y * (width as usize) + sample_x;

                            best_color = src_pixels[sample_idx];
                        }
                    }
                }
            }
        }

        *out_pixel = best_color;
    });

    out_fb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointillism_applies_effect() {
        let mut fb = Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF_FF_00_00); // Red background

        let config = PointillismConfig {
            max_radius: 4.0,
            density: 3,
        };

        let out = apply_pointillism(&fb, config);

        // It shouldn't just be black or the default background color,
        // there should be colors in it that depend on the original image (red dots).
        let mut has_red = false;
        for &p in out.as_slice() {
            if p == 0xFF_FF_00_00 {
                has_red = true;
                break;
            }
        }
        assert!(
            has_red,
            "Pointillism filter did not apply the source color (red) as dots"
        );
    }
}
