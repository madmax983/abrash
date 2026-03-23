//! ASCII Display Post-Processing Effect
//!
//! A retro post-processing effect that converts the framebuffer into an ASCII
//! art display, rendering characters back onto the screen using a built-in font.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the ASCII Display effect.
#[derive(Debug, Clone, Copy)]
pub struct AsciiDisplayConfig {
    /// Width of the ASCII character cell in pixels (e.g., 6).
    pub cell_width: usize,
    /// Height of the ASCII character cell in pixels (e.g., 10).
    pub cell_height: usize,
    /// Use colored characters (true) or monochrome green/white (false).
    pub colorize: bool,
    /// If not colorized, the default monochrome foreground color.
    pub monochrome_color: u32,
    /// Background color.
    pub background_color: u32,
}

impl Default for AsciiDisplayConfig {
    fn default() -> Self {
        Self {
            cell_width: 6,
            cell_height: 10,
            colorize: true,
            monochrome_color: 0xFF_00FF00, // Retro green
            background_color: 0xFF_000000, // Black
        }
    }
}

/// A minimal 6x10 font for 10 ASCII characters ordered by brightness.
/// Characters: ` .:-=+*#%@`
/// Each character is represented by 10 bytes, where the lower 6 bits of each byte
/// represent the pixels of a row.
const FONT: [[u8; 10]; 10] = [
    // ` ` (Space)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // `.`
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    // `:`
    [0x00, 0x00, 0x0C, 0x0C, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    // `-`
    [0x00, 0x00, 0x00, 0x00, 0x1E, 0x1E, 0x00, 0x00, 0x00, 0x00],
    // `=`
    [0x00, 0x00, 0x00, 0x1E, 0x00, 0x1E, 0x00, 0x00, 0x00, 0x00],
    // `+`
    [0x00, 0x00, 0x0C, 0x0C, 0x3F, 0x3F, 0x0C, 0x0C, 0x00, 0x00],
    // `*`
    [0x00, 0x00, 0x0C, 0x2D, 0x1E, 0x1E, 0x2D, 0x0C, 0x00, 0x00],
    // `#`
    [0x00, 0x12, 0x12, 0x3F, 0x12, 0x12, 0x3F, 0x12, 0x12, 0x00],
    // `%`
    [0x00, 0x23, 0x13, 0x08, 0x04, 0x02, 0x32, 0x31, 0x00, 0x00],
    // `@`
    [0x1E, 0x21, 0x2D, 0x35, 0x35, 0x2D, 0x20, 0x1E, 0x00, 0x00],
];

/// Applies an ASCII display effect to the framebuffer.
///
/// Divides the framebuffer into cells. For each cell, calculates the average
/// color and luminance, maps the luminance to an ASCII character, and renders
/// the character back into the cell using a built-in font.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the ASCII display.
pub fn apply_ascii_display(fb: &mut Framebuffer, config: &AsciiDisplayConfig) {
    if config.cell_width == 0 || config.cell_height == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cw = config.cell_width;
    let ch = config.cell_height;

    // We need to read original pixels, so we clone the buffer
    let original_pixels = fb.as_slice().to_vec();
    let pixels = fb.as_mut_slice();

    let cols = width / cw;
    let rows = height / ch;

    // Safely parallelize over chunked rows representing cell rows
    #[cfg(feature = "parallel")]
    let cell_row_iter = pixels.par_chunks_exact_mut(width * ch).enumerate();
    #[cfg(not(feature = "parallel"))]
    let cell_row_iter = pixels.chunks_exact_mut(width * ch).enumerate();

    cell_row_iter.for_each(|(cy, row_pixels)| {
        for cx in 0..cols {
            let start_x = cx * cw;

            // 1. Calculate average color and luminance for the cell
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut sum_lum = 0u32;

            for dy in 0..ch {
                for dx in 0..cw {
                    let px = start_x + dx;
                    let py = cy * ch + dy;
                    let idx = py * width + px;
                    let color = original_pixels[idx];

                    sum_r += (color >> 16) & 0xFF;
                    sum_g += (color >> 8) & 0xFF;
                    sum_b += color & 0xFF;
                    sum_lum += u32::from(pixel_luminance(color));
                }
            }

            let cell_pixels = (cw * ch) as u32;
            let avg_r = sum_r / cell_pixels;
            let avg_g = sum_g / cell_pixels;
            let avg_b = sum_b / cell_pixels;
            let avg_lum = sum_lum / cell_pixels;

            // Map luminance (0-255) to character index (0-9)
            let char_idx = ((avg_lum * 9) / 255).min(9) as usize;

            let fg_color = if config.colorize {
                0xFF_000000 | (avg_r << 16) | (avg_g << 8) | avg_b
            } else {
                config.monochrome_color
            };

            let bg_color = config.background_color;

            // 2. Render the character into the cell
            let char_bitmap = &FONT[char_idx];

            for dy in 0..ch {
                let bitmap_y = (dy * 10) / ch; // Scale Y to 0-9
                let row_bits = char_bitmap[bitmap_y];

                let dest_row_start = dy * width;

                for dx in 0..cw {
                    let px = start_x + dx;
                    let bitmap_x = (dx * 6) / cw; // Scale X to 0-5

                    // The font is 6 bits wide.
                    // Let's assume the highest bit (bit 5) is the leftmost pixel.
                    let mask = 1 << (5 - bitmap_x);

                    let dest_idx = dest_row_start + px;

                    if (row_bits & mask) != 0 {
                        row_pixels[dest_idx] = fg_color;
                    } else {
                        row_pixels[dest_idx] = bg_color;
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
    fn test_ascii_display_white_to_brightest() {
        let mut fb = Framebuffer::new(6, 10).unwrap();
        // Fully white
        fb.clear(0xFF_FFFFFF);

        let config = AsciiDisplayConfig {
            cell_width: 6,
            cell_height: 10,
            colorize: false,
            monochrome_color: 0xFF_FFFFFF, // White text
            background_color: 0xFF_000000, // Black bg
        };

        apply_ascii_display(&mut fb, &config);

        // Brightest character `@` is index 9.
        // Row 0 of `@` is 0x1E (011110)
        let row0 = &fb.as_slice()[0..6];

        // Let's manually verify the first row matches `011110`
        assert_eq!(row0[0], 0xFF_000000); // 0
        assert_eq!(row0[1], 0xFF_FFFFFF); // 1
        assert_eq!(row0[2], 0xFF_FFFFFF); // 1
        assert_eq!(row0[3], 0xFF_FFFFFF); // 1
        assert_eq!(row0[4], 0xFF_FFFFFF); // 1
        assert_eq!(row0[5], 0xFF_000000); // 0
    }

    #[test]
    fn test_ascii_display_black_to_darkest() {
        let mut fb = Framebuffer::new(6, 10).unwrap();
        // Fully black
        fb.clear(0xFF_000000);

        let config = AsciiDisplayConfig::default();

        apply_ascii_display(&mut fb, &config);

        // Darkest character ` ` is index 0. All pixels should be background_color.
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, config.background_color);
        }
    }
}
