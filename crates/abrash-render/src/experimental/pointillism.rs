//! Pointillism Filter Module
//!
//! A procedural post-processing effect that repaints the image using randomized overlapping dots
//! or brush strokes. It mimics the Pointillism art style (e.g., Georges Seurat).

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

/// Configuration for the Pointillism filter.
#[derive(Debug, Clone)]
pub struct PointillismConfig {
    /// The size of the cell grid. Larger cells mean larger dots.
    pub cell_size: i32,
    /// The radius of the dots relative to the cell size (e.g., 0.8 means 80% of cell).
    pub dot_scale: f32,
    /// How much randomness to apply to dot positions within their cell.
    pub jitter: f32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            cell_size: 10,
            dot_scale: 1.2,
            jitter: 0.5,
        }
    }
}

// Procedural hash function
#[inline(always)]
const fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(127_412_617);
    h ^ (h >> 16)
}

/// Applies a Pointillism filter to the framebuffer.
///
/// Uses a highly parallelizable procedural painter's algorithm. Instead of spawning particles
/// and drawing them, it iterates over every screen pixel and checks the surrounding grid cells
/// to see if a dot covers the current pixel.
pub fn apply_pointillism(fb: &mut Framebuffer, config: &PointillismConfig) {
    if config.cell_size <= 1 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    thread_local! {
        static SRC_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    SRC_BUFFER.with(|buf| {
        let mut src_vec = buf.borrow_mut();
        let size = width * height;
        if src_vec.len() < size {
            src_vec.resize(size, 0);
        }
        let src_slice = &mut src_vec[..size];
        src_slice.copy_from_slice(fb.as_slice());

        let pixels = fb.as_mut_slice();

        let cell_size = config.cell_size;
        let cell_size_f = cell_size as f32;
        let max_radius = cell_size_f * config.dot_scale;
        let max_radius_sq = max_radius * max_radius;

        // The maximum number of cells we need to search around a pixel
        let search_radius = (config.dot_scale + config.jitter).ceil() as i32;

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let mut min_dist_sq = f32::MAX;
                        let mut best_color = 0xFF00_0000;
                        let mut hit = false;

                        // Current pixel cell
                        let cx = (x as i32) / cell_size;
                        let cy = (y as i32) / cell_size;

                        // Check neighboring cells for overlapping dots
                        for dy in -search_radius..=search_radius {
                            for dx in -search_radius..=search_radius {
                                let cell_x = cx + dx;
                                let cell_y = cy + dy;

                                let h = hash(cell_x, cell_y);

                                // Random offsets [-1.0, 1.0]
                                let rx = ((h & 0xFF) as f32 / 127.5) - 1.0;
                                let ry = (((h >> 8) & 0xFF) as f32 / 127.5) - 1.0;

                                let center_x =
                                    (cell_x as f32 + 0.5 + rx * config.jitter) * cell_size_f;
                                let center_y =
                                    (cell_y as f32 + 0.5 + ry * config.jitter) * cell_size_f;

                                let dist_x = x as f32 - center_x;
                                let dist_y = y as f32 - center_y;
                                let dist_sq = dist_x * dist_x + dist_y * dist_y;

                                if dist_sq <= max_radius_sq && dist_sq < min_dist_sq {
                                    min_dist_sq = dist_sq;

                                    // Sample color from the source image at the dot's center
                                    let sample_x =
                                        (center_x as i32).clamp(0, (width - 1) as i32) as usize;
                                    let sample_y =
                                        (center_y as i32).clamp(0, (height - 1) as i32) as usize;
                                    best_color = src_slice[sample_y * width + sample_x];
                                    hit = true;
                                }
                            }
                        }

                        if hit {
                            *pixel = best_color;
                        } else {
                            // If no dot covers this pixel, you could leave the original color or use a background color.
                            // Pointillism usually paints a white canvas.
                            *pixel = 0xFF_FFFFFF;
                        }
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            // Same logic, scalar
            for y in 0..height {
                for x in 0..width {
                    let mut min_dist_sq = f32::MAX;
                    let mut best_color = 0xFF00_0000;
                    let mut hit = false;

                    let cx = (x as i32) / cell_size;
                    let cy = (y as i32) / cell_size;

                    for dy in -search_radius..=search_radius {
                        for dx in -search_radius..=search_radius {
                            let cell_x = cx + dx;
                            let cell_y = cy + dy;

                            let h = hash(cell_x, cell_y);

                            let rx = ((h & 0xFF) as f32 / 127.5) - 1.0;
                            let ry = (((h >> 8) & 0xFF) as f32 / 127.5) - 1.0;

                            let center_x = (cell_x as f32 + 0.5 + rx * config.jitter) * cell_size_f;
                            let center_y = (cell_y as f32 + 0.5 + ry * config.jitter) * cell_size_f;

                            let dist_x = x as f32 - center_x;
                            let dist_y = y as f32 - center_y;
                            let dist_sq = dist_x * dist_x + dist_y * dist_y;

                            if dist_sq <= max_radius_sq && dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;

                                let sample_x =
                                    (center_x as i32).clamp(0, (width - 1) as i32) as usize;
                                let sample_y =
                                    (center_y as i32).clamp(0, (height - 1) as i32) as usize;
                                best_color = src_slice[sample_y * width + sample_x];
                                hit = true;
                            }
                        }
                    }

                    if hit {
                        pixels[y * width + x] = best_color;
                    } else {
                        pixels[y * width + x] = 0xFF_FFFFFF;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pointillism() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFFFF_0000); // Red background

        let config = PointillismConfig {
            cell_size: 5,
            dot_scale: 1.0,
            jitter: 0.0,
        };

        apply_pointillism(&mut fb, &config);

        // Ensure it doesn't crash and modifies the buffer
        // (Due to the white background for unhit pixels, some pixels might be white, others red)
        let has_red = fb.as_slice().iter().any(|&p| p == 0xFFFF_0000);
        assert!(
            has_red,
            "Pointillism should sample and retain some original colors"
        );
    }
}
