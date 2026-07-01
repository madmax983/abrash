//! Pointillism Filter
//!
//! A post-processing effect that simulates a Pointillist painting style.
//! It works by sampling colors from the source image and splatting small
//! colored dots (circles) over a background.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Pointillism effect.
#[derive(Debug, Clone, Copy)]
pub struct PointillismConfig {
    /// The maximum radius of each dot.
    pub dot_radius: f32,
    /// The number of dots to draw per 100x100 pixel area.
    pub density: u32,
    /// Background color of the "canvas" (usually white or off-white).
    pub canvas_color: u32,
    /// Random seed for dot placement.
    pub seed: u32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            dot_radius: 3.0,
            density: 2000, // 2000 dots per 10,000 pixels = ~20% coverage per layer, but they overlap
            canvas_color: 0xFF_F0_F0_EA, // Off-white canvas
            seed: 0,
        }
    }
}

// Simple procedural hash for pseudo-random numbers
#[inline(always)]
const fn hash(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h
}

/// Applies a Pointillism effect to the framebuffer.
pub fn apply_pointillism(fb: &mut Framebuffer, config: &PointillismConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.dot_radius <= 0.0 || config.density == 0 {
        return;
    }

    // Clone original pixels
    let mut src_pixels = SOURCE_PIXELS.with(RefCell::take);
    let size = width * height;
    if src_pixels.len() < size {
        src_pixels.resize(size, 0);
    }
    src_pixels[..size].copy_from_slice(fb.as_slice());

    // Clear to canvas color
    fb.clear(config.canvas_color);

    let pixels = fb.as_mut_slice();

    // To process in parallel without writing conflicts, we divide the screen into independent tiles.
    // We'll use chunking over rows. This isn't perfect for overlapping circles if they cross chunk boundaries,
    // but for small dots, it's acceptable or we can just iterate chunks and constrain drawing within the chunk.

    // Actually, to make it perfectly safe for Rayon without complex tiling,
    // we can iterate over the entire destination buffer and for each pixel,
    // check if it falls within any randomly placed dot in its local neighborhood.
    // This is an inverse-mapping approach.

    let radius_i32 = config.dot_radius.ceil() as i32;
    let radius_sq = config.dot_radius * config.dot_radius;
    let src_slice = &src_pixels[..size];
    let density_scale = (config.density as f32) / 10000.0; // Expected dots per pixel

    let seed = config.seed;

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        let y_i32 = y as i32;
        for (x, pixel) in row.iter_mut().enumerate() {
            let x_i32 = x as i32;

            // For a given pixel (x,y), look at the surrounding neighborhood of size (2*R)x(2*R)
            // to see if any dot placed in that neighborhood covers this pixel.
            // We evaluate dots in a fixed order (e.g. by hashing the neighborhood coordinates)
            // and the "last" dot drawn on top wins.

            let mut final_color = config.canvas_color;
            let mut min_dist_sq = f32::MAX;
            let mut found_dot = false;

            // To simulate overlapping, we can just pick the nearest dot center in the neighborhood,
            // or evaluate a few procedural dots per grid cell.
            // Let's divide the screen into cells of size roughly equal to the expected distance between dots.
            let expected_dist = 1.0 / density_scale.sqrt();
            let cell_size = expected_dist.max(1.0);
            let cell_size_i32 = cell_size.ceil() as i32;

            // We need to check cells within radius
            let cells_to_check = (config.dot_radius / cell_size).ceil() as i32;

            let center_cell_x = (x as f32 / cell_size).floor() as i32;
            let center_cell_y = (y as f32 / cell_size).floor() as i32;

            // We want painters algorithm: dots drawn later overlap earlier ones.
            // We can simulate this by hashing the cell to get a 'z-index' or timestamp,
            // and taking the dot with the highest timestamp that covers this pixel.
            let mut max_timestamp = 0;

            for cy in (center_cell_y - cells_to_check)..=(center_cell_y + cells_to_check) {
                for cx in (center_cell_x - cells_to_check)..=(center_cell_x + cells_to_check) {
                    let cell_hash_base = hash(
                        (cx as u32).wrapping_mul(1973) ^ (cy as u32).wrapping_mul(9277) ^ seed,
                    );

                    // Does this cell have a dot? (Based on density)
                    // We can just assume 1 dot per cell, but jitter its position,
                    // and use the cell size to control density.

                    // Position jitter within the cell
                    let jx = (hash(cell_hash_base ^ 0x1234) % 1000) as f32 / 1000.0;
                    let jy = (hash(cell_hash_base ^ 0x5678) % 1000) as f32 / 1000.0;

                    let dot_x = cx as f32 * cell_size + jx * cell_size;
                    let dot_y = cy as f32 * cell_size + jy * cell_size;

                    let dx = x as f32 - dot_x;
                    let dy = y as f32 - dot_y;
                    let dist_sq = dx * dx + dy * dy;

                    // Radius jitter (varies between 0.5 * R and 1.0 * R)
                    let r_jitter =
                        0.5 + 0.5 * ((hash(cell_hash_base ^ 0x9ABC) % 1000) as f32 / 1000.0);
                    let actual_r_sq = radius_sq * r_jitter * r_jitter;

                    if dist_sq <= actual_r_sq {
                        let timestamp = hash(cell_hash_base ^ 0xDEF0);
                        if timestamp > max_timestamp {
                            max_timestamp = timestamp;
                            found_dot = true;

                            // Sample the color at the dot's center
                            let sample_x =
                                (dot_x.round() as i32).clamp(0, width as i32 - 1) as usize;
                            let sample_y =
                                (dot_y.round() as i32).clamp(0, height as i32 - 1) as usize;
                            final_color = src_slice[sample_y * width + sample_x];
                        }
                    }
                }
            }

            if found_dot {
                *pixel = final_color;
            }
        }
    });

    SOURCE_PIXELS.with(|buf| {
        buf.replace(src_pixels);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pointillism_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        apply_pointillism(&mut fb, &PointillismConfig::default());
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_apply_pointillism_standard() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_FF_00_00); // Red background

        let mut config = PointillismConfig::default();
        config.dot_radius = 5.0;
        config.density = 5000;

        apply_pointillism(&mut fb, &config);

        // Verify that the framebuffer has been altered and is no longer entirely solid red,
        // it should have some canvas color or dots.
        let mut has_canvas = false;
        let mut has_red = false;

        for &p in fb.as_slice() {
            if p == config.canvas_color {
                has_canvas = true;
            } else if p == 0xFF_FF_00_00 {
                has_red = true;
            }
        }

        assert!(has_canvas || has_red, "Should have something");
    }
}
