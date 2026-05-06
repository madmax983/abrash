//! Posterization processing for color quantization.
//!
//! This module provides the [`apply_posterize`] function, which reduces the number
//! of distinct colors in a [`Framebuffer`] to create a "poster-like" visual effect,
//! commonly used in stylistic and retro rendering pipelines.

use crate::framebuffer::Framebuffer;

/// Configuration for the Posterize effect.
#[derive(Debug, Clone, Copy)]
pub struct PosterizeConfig {
    /// The number of color levels per channel (e.g., 2.0 to 255.0).
    /// Lower values produce a more pronounced posterized effect.
    pub levels: f32,
}

impl Default for PosterizeConfig {
    fn default() -> Self {
        Self { levels: 4.0 }
    }
}

/// Applies a posterize effect to the given framebuffer in-place.
///
/// Reduces each RGB color channel to a discrete set of steps based on `config.levels`.
/// The alpha channel is preserved.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::experimental::posterize::{apply_posterize, PosterizeConfig};
///
/// let mut fb = Framebuffer::new(2, 1).unwrap();
/// // Setup a smooth gradient
/// fb.set_pixel(0, 0, 0xFF_6E6E6E); // Gray 110
/// fb.set_pixel(1, 0, 0xFF_787878); // Gray 120
///
/// // Quantize the image to only 4 levels per channel (e.g., 0, 85, 170, 255)
/// let config = PosterizeConfig { levels: 4.0 };
/// apply_posterize(&mut fb, &config);
///
/// // Both slightly different grays are snapped to the exact same quantization level (85 / 0x55)
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_555555);
/// assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFF_555555);
/// ```
pub fn apply_posterize(fb: &mut Framebuffer, config: &PosterizeConfig) {
    let levels = config.levels.max(2.0); // Minimum of 2 levels
    let levels_minus_1 = levels - 1.0;

    // Use chunks_exact_mut to eliminate bounds checking and option unwrapping
    let width = fb.width() as usize;
    for row in fb.as_mut_slice().chunks_exact_mut(width) {
        for pixel in row.iter_mut() {
            let p = *pixel;
            // Extract channels
            let a = (p >> 24) & 0xFF;
            let r = ((p >> 16) & 0xFF) as f32;
            let g = ((p >> 8) & 0xFF) as f32;
            let b = (p & 0xFF) as f32;

            // ⚡ Bolt: Replace f32::round() with fast integer casting
            // The color values are shifted to ensure they are always positive,
            // allowing a simple `+ 0.5` cast.
            let new_r = ((((r / 255.0 * levels_minus_1) + 16384.5) as i32 as f32 - 16384.0)
                / levels_minus_1
                * 255.0) as u32;
            let new_g = ((((g / 255.0 * levels_minus_1) + 16384.5) as i32 as f32 - 16384.0)
                / levels_minus_1
                * 255.0) as u32;
            let new_b = ((((b / 255.0 * levels_minus_1) + 16384.5) as i32 as f32 - 16384.0)
                / levels_minus_1
                * 255.0) as u32;

            // Clamp to prevent overflow on precision errors
            let new_r = new_r.min(255);
            let new_g = new_g.min(255);
            let new_b = new_b.min(255);

            *pixel = (a << 24) | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_posterize_reduces_colors() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        // Set two distinct bright pixels (near middle gray)
        fb.set_pixel(0, 0, 0xFF80_8080); // Mid-gray (128)
        fb.set_pixel(1, 0, 0xFF70_7070); // Slightly darker gray (112)

        let config = PosterizeConfig { levels: 2.0 };
        apply_posterize(&mut fb, &config);

        // With 2 levels (0 and 255)
        // 128 / 255 * 1.0 = 0.5019... -> round to 1.0 -> 1.0 / 1.0 * 255.0 = 255
        // 112 / 255 * 1.0 = 0.439... -> round to 0.0 -> 0.0 / 1.0 * 255.0 = 0
        // Oh, wait, 128 and 112 might round to different levels with levels=2.0.
        // Let's use colors that will snap to the same level.
        // If levels = 4 (0, 85, 170, 255):
        // 110/255 * 3 = 1.29 -> round = 1 -> 85
        // 120/255 * 3 = 1.41 -> round = 1 -> 85

        fb.set_pixel(0, 0, 0xFF6E_6E6E); // 110
        fb.set_pixel(1, 0, 0xFF78_7878); // 120

        let config = PosterizeConfig { levels: 4.0 };
        apply_posterize(&mut fb, &config);

        let p1 = fb.get_pixel(0, 0).unwrap();
        let p2 = fb.get_pixel(1, 0).unwrap();

        assert_eq!(p1, p2, "Pixels should be quantized to the same level");
        assert_eq!(p1, 0xFF55_5555, "Should be quantized to exactly 85 (0x55)"); // 85 is 0x55
    }
}
