//! Arbitrary Color Palette Quantization
//!
//! Maps every pixel in the framebuffer to the nearest color in a given predefined palette.
//! This allows for retro 8-bit, CGA, Gameboy, or custom aesthetic color restrictions.

use crate::framebuffer::Framebuffer;

/// A struct holding an arbitrary set of ARGB colors.
#[derive(Debug, Clone)]
pub struct Palette {
    pub colors: Vec<u32>,
}

impl Palette {
    /// Creates a new custom palette from a list of ARGB colors.
    #[must_use]
    pub const fn new(colors: Vec<u32>) -> Self {
        Self { colors }
    }

    /// The classic 4-color Nintendo Gameboy palette.
    #[must_use]
    pub fn gameboy() -> Self {
        Self {
            colors: vec![
                0xFF_0F380F, // Darkest Green
                0xFF_306230, // Dark Green
                0xFF_8BAC0F, // Light Green
                0xFF_9BBC0F, // Lightest Green
            ],
        }
    }

    /// The classic 16-color IBM CGA palette.
    #[must_use]
    pub fn cga() -> Self {
        Self {
            colors: vec![
                0xFF_000000,
                0xFF_0000AA,
                0xFF_00AA00,
                0xFF_00AAAA,
                0xFF_AA0000,
                0xFF_AA00AA,
                0xFF_AA5500,
                0xFF_AAAAAA,
                0xFF_555555,
                0xFF_5555FF,
                0xFF_55FF55,
                0xFF_55FFFF,
                0xFF_FF5555,
                0xFF_FF55FF,
                0xFF_FFFF55,
                0xFF_FFFFFF,
            ],
        }
    }

    /// A stylized synthwave/vaporwave aesthetic palette.
    #[must_use]
    pub fn vaporwave() -> Self {
        Self {
            colors: vec![
                0xFF_01012B, // Deep Space Blue
                0xFF_2D00F7, // Neon Blue
                0xFF_F20089, // Hot Pink
                0xFF_E500A4, // Magenta
                0xFF_DB00B6, // Purple
                0xFF_8900F2, // Deep Violet
                0xFF_FFD166, // Cyber Yellow
                0xFF_06D6A0, // Seafoam Green
            ],
        }
    }

    /// A stylized 1-bit monochrome (Black and White) palette.
    #[must_use]
    pub fn monochrome() -> Self {
        Self {
            colors: vec![0xFF_000000, 0xFF_FFFFFF],
        }
    }

    /// Returns the closest ARGB color in the palette to the given ARGB color.
    /// Finds the nearest color using squared Euclidean distance in RGB space.
    #[must_use]
    pub fn closest_color(&self, target_color: u32) -> u32 {
        if self.colors.is_empty() {
            return target_color;
        }

        let target_r = ((target_color >> 16) & 0xFF) as i32;
        let target_g = ((target_color >> 8) & 0xFF) as i32;
        let target_b = (target_color & 0xFF) as i32;
        let target_a = target_color & 0xFF00_0000;

        let mut min_dist = std::i32::MAX;
        let mut best_color = target_color;

        for &palette_color in &self.colors {
            let pr = ((palette_color >> 16) & 0xFF) as i32;
            let pg = ((palette_color >> 8) & 0xFF) as i32;
            let pb = (palette_color & 0xFF) as i32;

            let dr = target_r - pr;
            let dg = target_g - pg;
            let db = target_b - pb;

            // Squared Euclidean distance
            let dist_sq = dr * dr + dg * dg + db * db;

            if dist_sq < min_dist {
                min_dist = dist_sq;
                best_color = palette_color;
            }
        }

        target_a | (best_color & 0x00FF_FFFF)
    }
}

/// Applies an arbitrary color palette quantization to the framebuffer in-place.
///
/// Maps every pixel in the framebuffer to the visually closest color in the `palette`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `palette` - The `Palette` to map colors against.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::experimental::palette::{apply_palette, Palette};
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// // ... render scene ...
/// apply_palette(&mut fb, &Palette::gameboy());
/// ```
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_palette(fb: &mut Framebuffer, palette: &Palette) {
    if palette.colors.is_empty() {
        return;
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pixels.par_iter_mut().for_each(|pixel| {
            *pixel = palette.closest_color(*pixel);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for pixel in pixels.iter_mut() {
            *pixel = palette.closest_color(*pixel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closest_color_monochrome() {
        let palette = Palette::monochrome();

        // Very dark grey should map to Black
        let dark_grey = 0xFF_222222;
        assert_eq!(palette.closest_color(dark_grey), 0xFF_000000);

        // Very light grey should map to White
        let light_grey = 0xFF_DDDDDD;
        assert_eq!(palette.closest_color(light_grey), 0xFF_FFFFFF);

        // Alpha channel should be preserved
        let semi_transparent_dark = 0x88_111111;
        assert_eq!(palette.closest_color(semi_transparent_dark), 0x88_000000);
    }

    #[test]
    fn test_closest_color_gameboy() {
        let palette = Palette::gameboy();

        // Pure green should map to the light green (since 0xFF_8BAC0F is closer in Euclidean RGB space than 0xFF_9BBC0F)
        let pure_green = 0xFF_00FF00;
        assert_eq!(palette.closest_color(pure_green), 0xFF_8BAC0F);

        // Pure black should map to the darkest green
        let pure_black = 0xFF_000000;
        assert_eq!(palette.closest_color(pure_black), 0xFF_0F380F);
    }

    #[test]
    fn test_apply_palette() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFF_111111); // Dark
        fb.set_pixel(1, 0, 0xFF_EEEEEE); // Light
        fb.set_pixel(0, 1, 0x88_333333); // Dark with alpha
        fb.set_pixel(1, 1, 0x12_CCCCCC); // Light with alpha

        let palette = Palette::monochrome();
        apply_palette(&mut fb, &palette);

        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_000000);
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFF_FFFFFF);
        assert_eq!(fb.get_pixel(0, 1).unwrap(), 0x88_000000);
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0x12_FFFFFF);
    }
}
