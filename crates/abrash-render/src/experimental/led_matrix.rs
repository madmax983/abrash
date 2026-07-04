//! LED Matrix Filter
//!
//! A post-processing effect that converts the image into an LED matrix display,
//! grouping pixels into cells and drawing a glowing circular LED for each cell
//! to simulate a jumbotron or pixel display.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use std::cell::RefCell;
thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the LED Matrix post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct LedMatrixConfig {
    /// The size of each LED cell in pixels.
    pub cell_size: usize,
    /// The radius of the glowing LED within the cell. Should be <= `cell_size` / 2.
    pub led_radius: f32,
    /// How much to darken the background (0.0 = black, 1.0 = original color).
    pub background_darken: f32,
    /// How much to darken the LED edge for an antialiased look (0.0 = sharp, 1.0 = soft).
    pub edge_softness: f32,
}

impl Default for LedMatrixConfig {
    fn default() -> Self {
        Self {
            cell_size: 10,
            led_radius: 4.0,
            background_darken: 0.1,
            edge_softness: 1.0,
        }
    }
}

/// Applies an LED Matrix effect to the framebuffer.
///
/// Converts the image into a pixelated LED style by sampling the center of each cell
/// and drawing a circular dot with the sampled color, leaving the rest dark.
///
/// # Arguments
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the LED matrix effect.
pub fn apply_led_matrix(fb: &mut Framebuffer, config: &LedMatrixConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.cell_size <= 1 {
        return;
    }

    let cell_size = config.cell_size;
    let half_cell = cell_size / 2;
    let led_radius_sq = config.led_radius * config.led_radius;

    // Clone the source framebuffer because we are reading non-linearly
    // ⚡ Bolt: Eliminate per-frame heap allocation by using a thread-local static buffer.
    let mut source_pixels = SOURCE_PIXELS.with(RefCell::take);
    let size = width * height;
    source_pixels.clear();
    source_pixels.extend_from_slice(fb.as_slice());

    let src_pixels = &source_pixels[..size]; // reference to local
    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = dest_pixels.chunks_exact_mut(width).enumerate();

    let bg_mult = (config.background_darken * 256.0).clamp(0.0, 256.0) as u32;

    iter.for_each(|(y, row)| {
        for (x, pixel) in row.iter_mut().enumerate() {
            let cell_x = x / cell_size;
            let cell_y = y / cell_size;

            let center_x = (cell_x * cell_size + half_cell).min(width - 1);
            let center_y = (cell_y * cell_size + half_cell).min(height - 1);

            let src_pixel = src_pixels[center_y * width + center_x];

            let dx = x as f32 - center_x as f32;
            let dy = y as f32 - center_y as f32;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= led_radius_sq {
                // Inside the LED: smooth edge if needed
                if config.edge_softness > 0.0 {
                    let dist = dist_sq.sqrt();
                    let edge_dist = config.led_radius - dist;
                    if edge_dist < config.edge_softness {
                        let factor = edge_dist / config.edge_softness;
                        *pixel = darken_pixel(src_pixel, (factor * 256.0) as u32);
                    } else {
                        *pixel = src_pixel;
                    }
                } else {
                    *pixel = src_pixel;
                }
            } else {
                // Outside the LED: background
                *pixel = darken_pixel(src_pixel, bg_mult);
            }
        }
    });

    SOURCE_PIXELS.with(|buf| buf.replace(source_pixels));
}

#[inline(always)]
const fn darken_pixel(pixel: u32, darken_fixed: u32) -> u32 {
    let alpha = pixel & 0xFF00_0000;
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    let new_r = (r * darken_fixed) >> 8;
    let new_g = (g * darken_fixed) >> 8;
    let new_b = (b * darken_fixed) >> 8;

    alpha | (new_r << 16) | (new_g << 8) | new_b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_led_matrix_config_default() {
        let config = LedMatrixConfig::default();
        assert_eq!(config.cell_size, 10);
        assert!((config.led_radius - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_led_matrix() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        // Set all to white
        fb.clear(0xFF_FF_FF_FF);

        let config = LedMatrixConfig {
            cell_size: 10,
            led_radius: 4.0,
            background_darken: 0.0, // Pure black background
            edge_softness: 0.0,
        };
        apply_led_matrix(&mut fb, &config);

        // Center of the first cell (5,5) should be white
        let center_pixel = fb.get_pixel(5, 5).unwrap();
        assert_eq!(center_pixel, 0xFF_FF_FF_FF);

        // Corner of the first cell (0,0) should be black
        let corner_pixel = fb.get_pixel(0, 0).unwrap();
        assert_eq!(corner_pixel, 0xFF_00_00_00);
    }
}
