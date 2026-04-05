//! Pop Art Filter
//!
//! A retro post-processing effect that splits the screen into 4 quadrants and
//! applies a high-contrast Andy Warhol style pop-art color mapping to each.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Pop Art effect.
#[derive(Debug, Clone, Copy)]
pub struct PopArtConfig {
    /// Color for the dark areas of the top-left quadrant
    pub q1_dark: u32,
    /// Color for the light areas of the top-left quadrant
    pub q1_light: u32,

    /// Color for the dark areas of the top-right quadrant
    pub q2_dark: u32,
    /// Color for the light areas of the top-right quadrant
    pub q2_light: u32,

    /// Color for the dark areas of the bottom-left quadrant
    pub q3_dark: u32,
    /// Color for the light areas of the bottom-left quadrant
    pub q3_light: u32,

    /// Color for the dark areas of the bottom-right quadrant
    pub q4_dark: u32,
    /// Color for the light areas of the bottom-right quadrant
    pub q4_light: u32,
}

impl Default for PopArtConfig {
    fn default() -> Self {
        Self {
            q1_dark: 0xFF_0000FF,  // Blue
            q1_light: 0xFF_FFFF00, // Yellow

            q2_dark: 0xFF_FF0000,  // Red
            q2_light: 0xFF_00FFFF, // Cyan

            q3_dark: 0xFF_FF00FF,  // Magenta
            q3_light: 0xFF_00FF00, // Green

            q4_dark: 0xFF_000000,  // Black
            q4_light: 0xFF_FFFFFF, // White
        }
    }
}

/// Applies a pop art effect to the framebuffer.
///
/// Splits the image into 4 quadrants. For each pixel, it scales its coordinates
/// so the quadrant contains the full image, samples the original image, calculates
/// the luminance, and interpolates between the quadrant's configured light and dark colors.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the Pop Art colors.
pub fn apply_pop_art(fb: &mut Framebuffer, config: &PopArtConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let half_w = width / 2;
    let half_h = height / 2;

    let src = fb.as_slice().to_vec();
    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row_pixels)| {
        for x in 0..width {
            let is_top = y < half_h;
            let is_left = x < half_w;

            // Map the quadrant coordinate back to the full image coordinate
            let src_x = if is_left {
                (x * 2).min(width - 1)
            } else {
                ((x - half_w) * 2).min(width - 1)
            };

            let src_y = if is_top {
                (y * 2).min(height - 1)
            } else {
                ((y - half_h) * 2).min(height - 1)
            };

            let src_color = src[src_y * width + src_x];
            let lum = pixel_luminance(src_color) as f32 / 255.0;

            let (dark_color, light_color) = match (is_top, is_left) {
                (true, true) => (config.q1_dark, config.q1_light), // Top-Left
                (true, false) => (config.q2_dark, config.q2_light), // Top-Right
                (false, true) => (config.q3_dark, config.q3_light), // Bottom-Left
                (false, false) => (config.q4_dark, config.q4_light), // Bottom-Right
            };

            // Interpolate between dark and light based on luminance
            let dr = ((dark_color >> 16) & 0xFF) as f32;
            let dg = ((dark_color >> 8) & 0xFF) as f32;
            let db = (dark_color & 0xFF) as f32;

            let lr = ((light_color >> 16) & 0xFF) as f32;
            let lg = ((light_color >> 8) & 0xFF) as f32;
            let lb = (light_color & 0xFF) as f32;

            // Simple thresholding or interpolation
            // Let's use a sharp threshold with slight anti-aliasing (smoothstep) for the classic look.
            // But simple linear interpolation works well too for testing.
            let r = dr + (lr - dr) * lum;
            let g = dg + (lg - dg) * lum;
            let b = db + (lb - db) * lum;

            row_pixels[x] = 0xFF_000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pop_art() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Fill the framebuffer with a solid mid-gray
        fb.clear(0xFF_808080);

        let config = PopArtConfig::default();
        apply_pop_art(&mut fb, &config);

        // The image should be divided into quadrants with the respective colors applied.
        // Since input is gray (luminance ~128), it should be a mix of light and dark colors.
        // Let's just check the top-left corner (q1) to ensure it's no longer the original gray.
        let p = fb.get_pixel(0, 0).unwrap();
        assert_ne!(
            p, 0xFF_808080,
            "Pixel should be modified by the pop art filter"
        );
    }
}
