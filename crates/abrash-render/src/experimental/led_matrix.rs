//! LED Matrix Display Post-Processing Effect
//!
//! A retro post-processing effect that simulates an LED dot-matrix display
//! by splitting the framebuffer into discrete circular diodes with a grid of black space between them.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static LED_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the LED Matrix effect.
#[derive(Debug, Clone, Copy)]
pub struct LedMatrixConfig {
    /// The size of each LED cell in pixels (e.g., 8).
    pub cell_size: usize,
    /// The radius of the LED diode inside the cell in pixels (e.g., 3.0 for an 8x8 cell).
    pub dot_radius: f32,
    /// Whether to enhance the brightness of the "lit" LED to simulate bloom.
    pub bloom_intensity: f32,
}

impl Default for LedMatrixConfig {
    fn default() -> Self {
        Self {
            cell_size: 8,
            dot_radius: 3.5,
            bloom_intensity: 1.2,
        }
    }
}

/// Applies an LED matrix display effect to the framebuffer.
///
/// Divides the framebuffer into cells. For each cell, it calculates the average
/// color and then renders a filled circle (the "LED") using that color in the center.
/// The remainder of the cell is left black to simulate the gaps between LEDs.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the LED matrix effect.
pub fn apply_led_matrix(fb: &mut Framebuffer, config: &LedMatrixConfig) {
    if config.cell_size <= 1 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cell_size = config.cell_size;
    let dot_radius_sq = config.dot_radius * config.dot_radius;

    // We must read from the original pixels and write to a new buffer
    // because we map cells to average colors and redraw them entirely.
    LED_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels_mut = &mut src_fb_vec[..size];
        src_pixels_mut.copy_from_slice(fb.as_slice());

        // Re-borrow immutably for thread safety in rayon closures
        let src_pixels: &[u32] = src_pixels_mut;

        let dest_pixels = fb.as_mut_slice();

        // Safe parallelization by row-chunks
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            let row_chunk_size = cell_size * width;
            let num_chunks = height / cell_size;
            let (main_chunks, remainder) = dest_pixels.split_at_mut(num_chunks * row_chunk_size);

            main_chunks
                .par_chunks_exact_mut(row_chunk_size)
                .enumerate()
                .for_each(|(chunk_idx, cell_row_pixels)| {
                    let start_y = chunk_idx * cell_size;

                for x in (0..width).step_by(cell_size) {
                    let end_x = std::cmp::min(x + cell_size, width);
                    let end_y = start_y + cell_size; // Safe because of chunks_exact_mut

                    // 1. Calculate average color of the cell
                    let mut sum_r = 0;
                    let mut sum_g = 0;
                    let mut sum_b = 0;
                    let mut count = 0;

                    for cy in start_y..end_y {
                        let src_row_start = cy * width;
                        for cx in x..end_x {
                            let pixel = src_pixels[src_row_start + cx];
                            sum_r += (pixel >> 16) & 0xFF;
                            sum_g += (pixel >> 8) & 0xFF;
                            sum_b += pixel & 0xFF;
                            count += 1;
                        }
                    }

                    let avg_r = if count > 0 { sum_r / count } else { 0 };
                    let avg_g = if count > 0 { sum_g / count } else { 0 };
                    let avg_b = if count > 0 { sum_b / count } else { 0 };

                    // Apply bloom
                    let mut r = (avg_r as f32 * config.bloom_intensity) as u32;
                    let mut g = (avg_g as f32 * config.bloom_intensity) as u32;
                    let mut b = (avg_b as f32 * config.bloom_intensity) as u32;

                    r = r.min(255);
                    g = g.min(255);
                    b = b.min(255);

                    let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                    // Center of the cell
                    let center_x = x as f32 + (cell_size as f32 / 2.0);
                    let center_y = start_y as f32 + (cell_size as f32 / 2.0);

                    // 2. Draw the LED diode
                    for cy in 0..cell_size {
                        let world_y = start_y + cy;
                        let y_dest_offset = cy * width;
                        let dy = world_y as f32 + 0.5 - center_y;

                        for cx in 0..(end_x - x) {
                            let world_x = x + cx;
                            let dx = world_x as f32 + 0.5 - center_x;

                            let dist_sq = dx * dx + dy * dy;

                            if dist_sq <= dot_radius_sq {
                                // Inside LED
                                cell_row_pixels[y_dest_offset + world_x] = color;
                            } else {
                                // Outside LED (Gap)
                                cell_row_pixels[y_dest_offset + world_x] = 0xFF00_0000;
                            }
                        }
                    }
                }
            });

            // Handle the remainder (the last partial row of cells)
            if !remainder.is_empty() {
                let start_y = num_chunks * cell_size;
                let end_y = height;
                let actual_cell_height = end_y - start_y;

                for x in (0..width).step_by(cell_size) {
                    let end_x = std::cmp::min(x + cell_size, width);

                    // 1. Calculate average color
                    let mut sum_r = 0;
                    let mut sum_g = 0;
                    let mut sum_b = 0;
                    let mut count = 0;

                    for cy in start_y..end_y {
                        let src_row_start = cy * width;
                        for cx in x..end_x {
                            let pixel = src_pixels[src_row_start + cx];
                            sum_r += (pixel >> 16) & 0xFF;
                            sum_g += (pixel >> 8) & 0xFF;
                            sum_b += pixel & 0xFF;
                            count += 1;
                        }
                    }

                    let avg_r = if count > 0 { sum_r / count } else { 0 };
                    let avg_g = if count > 0 { sum_g / count } else { 0 };
                    let avg_b = if count > 0 { sum_b / count } else { 0 };

                    let mut r = (avg_r as f32 * config.bloom_intensity) as u32;
                    let mut g = (avg_g as f32 * config.bloom_intensity) as u32;
                    let mut b = (avg_b as f32 * config.bloom_intensity) as u32;

                    r = r.min(255);
                    g = g.min(255);
                    b = b.min(255);

                    let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                    let center_x = x as f32 + (cell_size as f32 / 2.0);
                    let center_y = start_y as f32 + (cell_size as f32 / 2.0);

                    // 2. Draw the LED diode (clamped to actual_cell_height)
                    for cy in 0..actual_cell_height {
                        let world_y = start_y + cy;
                        let y_dest_offset = cy * width;
                        let dy = world_y as f32 + 0.5 - center_y;

                        for cx in 0..(end_x - x) {
                            let world_x = x + cx;
                            let dx = world_x as f32 + 0.5 - center_x;

                            let dist_sq = dx * dx + dy * dy;

                            if dist_sq <= dot_radius_sq {
                                remainder[y_dest_offset + world_x] = color;
                            } else {
                                remainder[y_dest_offset + world_x] = 0xFF00_0000;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            // Sequential fallback
            for start_y in (0..height).step_by(cell_size) {
                let end_y = std::cmp::min(start_y + cell_size, height);

                for x in (0..width).step_by(cell_size) {
                    let end_x = std::cmp::min(x + cell_size, width);

                    // 1. Calculate average color
                    let mut sum_r = 0;
                    let mut sum_g = 0;
                    let mut sum_b = 0;
                    let mut count = 0;

                    for cy in start_y..end_y {
                        let src_row_start = cy * width;
                        for cx in x..end_x {
                            let pixel = src_pixels[src_row_start + cx];
                            sum_r += (pixel >> 16) & 0xFF;
                            sum_g += (pixel >> 8) & 0xFF;
                            sum_b += pixel & 0xFF;
                            count += 1;
                        }
                    }

                    let avg_r = if count > 0 { sum_r / count } else { 0 };
                    let avg_g = if count > 0 { sum_g / count } else { 0 };
                    let avg_b = if count > 0 { sum_b / count } else { 0 };

                    let mut r = (avg_r as f32 * config.bloom_intensity) as u32;
                    let mut g = (avg_g as f32 * config.bloom_intensity) as u32;
                    let mut b = (avg_b as f32 * config.bloom_intensity) as u32;

                    r = r.min(255);
                    g = g.min(255);
                    b = b.min(255);

                    let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                    let center_x = x as f32 + (cell_size as f32 / 2.0);
                    let center_y = start_y as f32 + (cell_size as f32 / 2.0);

                    // 2. Draw the LED diode
                    for cy in start_y..end_y {
                        let dest_row_start = cy * width;
                        let dy = cy as f32 + 0.5 - center_y;

                        for cx in x..end_x {
                            let dx = cx as f32 + 0.5 - center_x;
                            let dist_sq = dx * dx + dy * dy;

                            if dist_sq <= dot_radius_sq {
                                dest_pixels[dest_row_start + cx] = color;
                            } else {
                                dest_pixels[dest_row_start + cx] = 0xFF00_0000;
                            }
                        }
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
    fn test_apply_led_matrix_black_image() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF00_0000);

        let config = LedMatrixConfig {
            cell_size: 5,
            dot_radius: 2.0,
            bloom_intensity: 1.0,
        };

        apply_led_matrix(&mut fb, &config);

        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF00_0000, "Entire image should remain black");
        }
    }

    #[test]
    fn test_apply_led_matrix_gaps() {
        let mut fb = Framebuffer::new(8, 8).unwrap();
        // Solid white
        fb.clear(0xFFFF_FFFF);

        let config = LedMatrixConfig {
            cell_size: 8,
            dot_radius: 2.0, // Small radius, lots of gap
            bloom_intensity: 1.0,
        };

        apply_led_matrix(&mut fb, &config);

        // Center pixel should be white (LED)
        assert_eq!(fb.get_pixel(4, 4).unwrap(), 0xFFFF_FFFF);

        // Corner pixel should be black (Gap)
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF00_0000);
    }
}
