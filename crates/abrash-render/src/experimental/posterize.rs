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
    // ⚡ Bolt: Use a pre-calculated Look-Up Table (LUT) to eliminate integer math per-pixel per-channel
    let levels = config.levels.max(2.0);
    let levels_minus_1 = (levels - 1.0) as u32;
    let safe_divisor = levels_minus_1.max(1);

    let mut lut = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        lut[i as usize] = ((i * levels_minus_1 + 127) / 255 * 255) / safe_divisor;
        i += 1;
    }

    for pixel in fb.as_mut_slice().iter_mut() {
        let p = *pixel;
        let a = p & 0xFF00_0000;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        let new_r = lut[r as usize].min(255);
        let new_g = lut[g as usize].min(255);
        let new_b = lut[b as usize].min(255);

        *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
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

    #[test]
    fn test_apply_posterize_extreme_levels() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF00_0000); // Black
        fb.set_pixel(1, 0, 0xFFFF_FFFF); // White

        // Test with levels = 256.0 (should do nothing practically, max colors)
        let config_max = PosterizeConfig { levels: 256.0 };
        apply_posterize(&mut fb, &config_max);

        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF00_0000);
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFFFF_FFFF);

        // Test with levels = 2.0 (should just be min/max colors)
        let mut fb_min = Framebuffer::new(3, 1).unwrap();
        fb_min.set_pixel(0, 0, 0xFF00_0000); // Black -> 0
        fb_min.set_pixel(1, 0, 0xFF80_8080); // Mid-gray (128) -> 255
        fb_min.set_pixel(2, 0, 0xFFFF_FFFF); // White -> 255

        let config_min = PosterizeConfig { levels: 2.0 };
        apply_posterize(&mut fb_min, &config_min);

        assert_eq!(fb_min.get_pixel(0, 0).unwrap(), 0xFF00_0000);
        assert_eq!(fb_min.get_pixel(1, 0).unwrap(), 0xFFFF_FFFF);
        assert_eq!(fb_min.get_pixel(2, 0).unwrap(), 0xFFFF_FFFF);
    }
}
