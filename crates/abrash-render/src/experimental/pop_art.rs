//! Pop Art Post-Processing Filter
//!
//! A retro-style filter that scales the image down into a 2x2 grid (4 quadrants)
//! and applies a different color tint to each quadrant, simulating classic Pop Art.

use abrash_core::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static POP_ART_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Pop Art effect.
#[derive(Debug, Clone, Copy)]
pub struct PopArtConfig {
    pub color_top_left: u32,
    pub color_top_right: u32,
    pub color_bottom_left: u32,
    pub color_bottom_right: u32,
}

impl Default for PopArtConfig {
    fn default() -> Self {
        Self {
            color_top_left: 0xFFFF0000,     // Red
            color_top_right: 0xFF00FF00,    // Green
            color_bottom_left: 0xFF0000FF,  // Blue
            color_bottom_right: 0xFFFFFF00, // Yellow
        }
    }
}

/// Applies a Pop Art filter to the framebuffer.
///
/// Divides the screen into 4 quadrants. Each quadrant shows a scaled-down
/// version of the entire original framebuffer, tinted with a specific color.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The colors for the four quadrants.
pub fn apply_pop_art(fb: &mut Framebuffer, config: &PopArtConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    POP_ART_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        src_fb_vec.clear();
        src_fb_vec.extend_from_slice(fb.as_slice());

        let src_pixels = &src_fb_vec[..];
        let dest_pixels = fb.as_mut_slice();

        let half_w = width / 2;
        let half_h = height / 2;

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            for (x, pixel) in row.iter_mut().enumerate() {
                // Determine which quadrant we are in
                let is_right = x >= half_w;
                let is_bottom = y >= half_h;

                let tint_color = match (is_right, is_bottom) {
                    (false, false) => config.color_top_left,
                    (true, false) => config.color_top_right,
                    (false, true) => config.color_bottom_left,
                    (true, true) => config.color_bottom_right,
                };

                // Map (x, y) to source coordinates (scale down by 2)
                let local_x = if is_right { x - half_w } else { x };
                let local_y = if is_bottom { y - half_h } else { y };

                // Handle edge cases where width/height are odd
                let src_x = (local_x * 2).min(width.saturating_sub(1));
                let src_y = (local_y * 2).min(height.saturating_sub(1));

                let src_pixel = src_pixels[src_y * width + src_x];

                // Extract source color
                let r = (src_pixel >> 16) & 0xFF;
                let g = (src_pixel >> 8) & 0xFF;
                let b = src_pixel & 0xFF;

                // Fast luminance calculation for the source pixel
                let lum = (r * 299 + g * 587 + b * 114) / 1000;

                // Extract tint color
                let t_r = (tint_color >> 16) & 0xFF;
                let t_g = (tint_color >> 8) & 0xFF;
                let t_b = tint_color & 0xFF;

                // Multiply luminance by the tint color
                let out_r = (lum * t_r) / 255;
                let out_g = (lum * t_g) / 255;
                let out_b = (lum * t_b) / 255;

                *pixel = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pop_art() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Fill the source with white, so luminance is 255
        fb.clear(0xFFFFFFFF);

        let config = PopArtConfig {
            color_top_left: 0xFFFF0000,     // Red
            color_top_right: 0xFF00FF00,    // Green
            color_bottom_left: 0xFF0000FF,  // Blue
            color_bottom_right: 0xFFFFFF00, // Yellow
        };

        apply_pop_art(&mut fb, &config);

        // Top left quadrant (x < 2, y < 2) should be Red
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF0000);
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFFFF0000);

        // Top right quadrant (x >= 2, y < 2) should be Green
        assert_eq!(fb.get_pixel(2, 0).unwrap(), 0xFF00FF00);
        assert_eq!(fb.get_pixel(3, 1).unwrap(), 0xFF00FF00);

        // Bottom left quadrant (x < 2, y >= 2) should be Blue
        assert_eq!(fb.get_pixel(0, 2).unwrap(), 0xFF0000FF);
        assert_eq!(fb.get_pixel(1, 3).unwrap(), 0xFF0000FF);

        // Bottom right quadrant (x >= 2, y >= 2) should be Yellow
        assert_eq!(fb.get_pixel(2, 2).unwrap(), 0xFFFFFF00);
        assert_eq!(fb.get_pixel(3, 3).unwrap(), 0xFFFFFF00);
    }
}
