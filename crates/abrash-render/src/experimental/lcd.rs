//! LCD / Gameboy Screen Post-Processing Filter
//!
//! A retro filter that simulates the look of an old liquid-crystal display.
//! It maps the framebuffer's colors to a 4-color green palette (like the original Gameboy)
//! and adds a subtle grid to simulate the physical gaps between LCD pixels.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the LCD Filter.
#[derive(Debug, Clone, Copy)]
pub struct LcdConfig {
    /// The size of the simulated LCD pixels in screen pixels (e.g., 4).
    pub pixel_size: usize,
    /// The color of the grid lines between pixels.
    pub grid_color: u32,
    /// The 4 colors used for the LCD display, ordered from darkest to lightest.
    pub palette: [u32; 4],
}

impl Default for LcdConfig {
    fn default() -> Self {
        Self {
            pixel_size: 4,
            grid_color: 0xFF_8BAC0F, // A greenish-yellow color for the grid lines
            palette: [
                0xFF_0F380F, // Darkest Green (Black)
                0xFF_306230, // Dark Green (Dark Gray)
                0xFF_8BAC0F, // Light Green (Light Gray)
                0xFF_9BBC0F, // Lightest Green (White)
            ],
        }
    }
}

/// Applies an LCD screen effect to the framebuffer.
///
/// Divides the screen into a grid of `pixel_size` x `pixel_size` cells.
/// For each cell, it calculates the average luminance, maps it to one of the 4 colors
/// in the configured palette, and fills the cell with that color. It leaves a 1-pixel
/// gap on the right and bottom of each cell to simulate the LCD grid.
///
/// # Arguments
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the LCD effect.
pub fn apply_lcd(fb: &mut Framebuffer, config: &LcdConfig) {
    if config.pixel_size == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let p_size = config.pixel_size;
    let pixels = fb.as_mut_slice();

    // To prevent read-after-write aliasing when calculating averages and then writing,
    // we process block by block. Since we only read from the current block to calculate
    // its average and then overwrite it, we are safe to do this in-place.

    #[cfg(feature = "parallel")]
    let block_row_iter = pixels.par_chunks_exact_mut(width * p_size).enumerate();
    #[cfg(not(feature = "parallel"))]
    let block_row_iter = pixels.chunks_exact_mut(width * p_size).enumerate();

    block_row_iter.for_each(|(_, block_rows)| {
        let block_height = block_rows.len() / width;
        if block_height == 0 {
            return;
        }

        for bx in (0..width).step_by(p_size) {
            let block_width = std::cmp::min(p_size, width - bx);

            // 1. Calculate average luminance for the block
            let mut sum_lum = 0u32;
            for y in 0..block_height {
                let row_start = y * width;
                for x in 0..block_width {
                    let color = block_rows[row_start + bx + x];
                    sum_lum += u32::from(pixel_luminance(color));
                }
            }

            let cell_pixels = (block_width * block_height) as u32;
            let avg_lum = sum_lum / cell_pixels; // 0 to 255

            // 2. Map luminance to palette index
            // We divide 256 by 4 = 64.
            // 0-63 -> 0 (Darkest)
            // 64-127 -> 1
            // 128-191 -> 2
            // 192-255 -> 3 (Lightest)
            let palette_idx = (avg_lum / 64).min(3) as usize;
            let cell_color = config.palette[palette_idx];

            // 3. Fill the block
            for y in 0..block_height {
                let row_start = y * width;
                for x in 0..block_width {
                    // Draw grid line if on the right or bottom edge of the cell,
                    // unless we are at the very edge of the screen where a cell might be truncated.
                    // To keep it simple and consistent with typical LCD filters, we draw the grid
                    // on the last pixel of the block_size.
                    let is_grid_x = x == p_size - 1;
                    let is_grid_y = y == p_size - 1;

                    let final_color = if is_grid_x || is_grid_y {
                        config.grid_color
                    } else {
                        cell_color
                    };

                    block_rows[row_start + bx + x] = final_color;
                }
            }
        }
    });

    // Handle any remaining rows at the bottom that didn't fit into a full chunk
    let remainder = height % p_size;
    if remainder > 0 {
        let remainder_start = height - remainder;
        let remainder_slice = &mut pixels[(remainder_start * width)..];

        for bx in (0..width).step_by(p_size) {
            let block_width = std::cmp::min(p_size, width - bx);

            // 1. Calculate average luminance
            let mut sum_lum = 0u32;
            for y in 0..remainder {
                let row_start = y * width;
                for x in 0..block_width {
                    let color = remainder_slice[row_start + bx + x];
                    sum_lum += u32::from(pixel_luminance(color));
                }
            }

            let cell_pixels = (block_width * remainder) as u32;
            let avg_lum = sum_lum / cell_pixels;

            // 2. Map to palette
            let palette_idx = (avg_lum / 64).min(3) as usize;
            let cell_color = config.palette[palette_idx];

            // 3. Fill the block
            for y in 0..remainder {
                let row_start = y * width;
                for x in 0..block_width {
                    let is_grid_x = x == p_size - 1;
                    let is_grid_y = y == p_size - 1;

                    let final_color = if is_grid_x || is_grid_y {
                        config.grid_color
                    } else {
                        cell_color
                    };

                    remainder_slice[row_start + bx + x] = final_color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcd_palette_mapping() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Set to fully white (luminance 255)
        fb.clear(0xFF_FFFFFF);

        let config = LcdConfig::default();
        apply_lcd(&mut fb, &config);

        // Should map to lightest green (palette[3]) except for the grid edges
        let cell_color = config.palette[3];
        assert_eq!(fb.get_pixel(0, 0).unwrap(), cell_color);

        // Check grid edges (x=3 or y=3)
        assert_eq!(fb.get_pixel(3, 0).unwrap(), config.grid_color);
        assert_eq!(fb.get_pixel(0, 3).unwrap(), config.grid_color);
        assert_eq!(fb.get_pixel(3, 3).unwrap(), config.grid_color);

        // Now test black
        fb.clear(0xFF_000000);
        apply_lcd(&mut fb, &config);

        // Should map to darkest green (palette[0])
        let dark_color = config.palette[0];
        assert_eq!(fb.get_pixel(0, 0).unwrap(), dark_color);
        assert_eq!(fb.get_pixel(3, 3).unwrap(), config.grid_color);
    }
}
