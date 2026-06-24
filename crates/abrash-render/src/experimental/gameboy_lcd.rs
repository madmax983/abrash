//! Gameboy LCD Simulator Filter
//!
//! A mashup filter that combines Palette Quantization (Gameboy 4-colors),
//! Macro-pixelation, LCD grid lines, and phosphor decay (ghosting).

use crate::experimental::palette::{Palette, apply_palette};
use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static PREVIOUS_FRAME: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Gameboy LCD Simulator.
#[derive(Debug, Clone, Copy)]
pub struct GameboyLcdConfig {
    /// The size of each LCD "macro pixel".
    pub pixel_size: usize,
    /// How much to darken the grid lines (0.0 = black, 1.0 = original color).
    pub grid_darken: f32,
    /// Phosphor decay/ghosting blend factor (0.0 = no ghosting, 1.0 = full ghosting).
    pub ghosting_decay: f32,
}

impl Default for GameboyLcdConfig {
    fn default() -> Self {
        Self {
            pixel_size: 4,
            grid_darken: 0.8,
            ghosting_decay: 0.5,
        }
    }
}

/// Applies the Gameboy LCD simulation to the framebuffer.
///
/// # Arguments
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the Gameboy LCD effect.
pub fn apply_gameboy_lcd(fb: &mut Framebuffer, config: &GameboyLcdConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.pixel_size == 0 {
        return;
    }

    // 1. Quantize colors to the Gameboy palette
    let palette = Palette::gameboy();
    apply_palette(fb, &palette);

    let b_size = config.pixel_size;
    let grid_darken = (config.grid_darken * 256.0).clamp(0.0, 256.0) as u32;
    let ghosting = config.ghosting_decay.clamp(0.0, 1.0);

    let pixels = fb.as_mut_slice();
    let size = width * height;

    // 2 & 3. Process Macro-pixelation and LCD grid lines
    #[cfg(feature = "parallel")]
    {
        let chunk_size = width * b_size;
        pixels
            .par_chunks_exact_mut(chunk_size)
            .for_each(|block_rows| {
                let block_height = block_rows.len() / width;
                if block_height == 0 {
                    return;
                }

                // Process first row of the block
                for x in (0..width).step_by(b_size) {
                    let block_width = std::cmp::min(b_size, width - x);
                    let color = block_rows[x]; // sample upper-left

                    for dx in 0..block_width {
                        let is_right_edge = dx == block_width - 1;
                        // Grid line top row right edge
                        block_rows[x + dx] = if is_right_edge {
                            darken_pixel(color, grid_darken)
                        } else {
                            color
                        };
                    }
                }

                // Copy and process remaining rows
                if block_height > 1 {
                    let (first_row_region, rest) = block_rows.split_at_mut(width);
                    for by in 1..block_height {
                        let is_bottom_edge = by == block_height - 1;
                        let dest_start = (by - 1) * width;

                        for x in 0..width {
                            let mut color = first_row_region[x];
                            // Apply grid darken to bottom edge or right edge (already darkened in first row)
                            let is_right_edge = (x % b_size) == b_size - 1;

                            if is_bottom_edge {
                                color = darken_pixel(color, grid_darken);
                            }
                            rest[dest_start + x] = color;
                        }
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in (0..height).step_by(b_size) {
            let block_height = std::cmp::min(b_size, height - y);
            let row_start = y * width;

            for x in (0..width).step_by(b_size) {
                let block_width = std::cmp::min(b_size, width - x);
                let color = pixels[row_start + x]; // sample upper-left

                for by in 0..block_height {
                    for bx in 0..block_width {
                        let is_right_edge = bx == block_width - 1;
                        let is_bottom_edge = by == block_height - 1;

                        let final_color = if is_right_edge || is_bottom_edge {
                            darken_pixel(color, grid_darken)
                        } else {
                            color
                        };

                        pixels[row_start + (by * width) + x + bx] = final_color;
                    }
                }
            }
        }
    }

    // 4. Phosphor decay (ghosting)
    if ghosting > 0.0 {
        PREVIOUS_FRAME.with(|buf| {
            let mut prev_frame = buf.borrow_mut();
            if prev_frame.len() < size {
                prev_frame.resize(size, 0xFF00_0000);
            }

            let prev_slice = &mut prev_frame[..size];

            #[cfg(feature = "parallel")]
            {
                pixels
                    .par_iter_mut()
                    .zip(prev_slice.par_iter_mut())
                    .for_each(|(curr, prev)| {
                        *curr = blend_colors(*prev, *curr, ghosting);
                        *prev = *curr;
                    });
            }

            #[cfg(not(feature = "parallel"))]
            {
                for i in 0..size {
                    pixels[i] = blend_colors(prev_slice[i], pixels[i], ghosting);
                    prev_slice[i] = pixels[i];
                }
            }
        });
    }
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

#[inline(always)]
fn blend_colors(c1: u32, c2: u32, t: f32) -> u32 {
    let inv_t = 1.0 - t;

    let a1 = ((c1 >> 24) & 0xFF) as f32;
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = (c1 & 0xFF) as f32;

    let a2 = ((c2 >> 24) & 0xFF) as f32;
    let r2 = ((c2 >> 16) & 0xFF) as f32;
    let g2 = ((c2 >> 8) & 0xFF) as f32;
    let b2 = (c2 & 0xFF) as f32;

    let a = (a1 * t + a2 * inv_t) as u32;
    let r = (r1 * t + r2 * inv_t) as u32;
    let g = (g1 * t + g2 * inv_t) as u32;
    let b = (b1 * t + b2 * inv_t) as u32;

    (a << 24) | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gameboy_lcd_config_default() {
        let config = GameboyLcdConfig::default();
        assert_eq!(config.pixel_size, 4);
        assert!((config.grid_darken - 0.8).abs() < 1e-6);
        assert!((config.ghosting_decay - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_apply_gameboy_lcd_modifies_buffer() {
        let mut fb = Framebuffer::new(8, 8).unwrap();
        fb.clear(0xFF_FF_FF_FF);

        let mut fb_clone = Framebuffer::new(8, 8).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        let config = GameboyLcdConfig::default();
        apply_gameboy_lcd(&mut fb, &config);

        let mut different = false;
        for i in 0..(8 * 8) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(
            different,
            "Gameboy LCD filter did not modify the framebuffer"
        );
    }
}
