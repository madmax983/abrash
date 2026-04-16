//! Selective Color filter.
//!
//! A post-processing effect that converts the image to grayscale, except for pixels
//! that match a specific target hue. Useful for emphasizing certain objects or colors
//! (e.g., Sin City style).

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Selective Color effect.
#[derive(Debug, Clone, Copy)]
pub struct SelectiveColorConfig {
    /// The target hue (0.0 to 360.0) to retain.
    pub target_hue: f32,
    /// The tolerance or distance threshold for the hue.
    pub tolerance: f32,
    /// How much to desaturate non-matching colors (0.0 = original, 1.0 = full grayscale).
    pub desaturation: f32,
}

impl Default for SelectiveColorConfig {
    fn default() -> Self {
        Self {
            target_hue: 0.0,   // Red
            tolerance: 30.0,   // Allow slight variations
            desaturation: 1.0, // Full grayscale
        }
    }
}

/// Applies the Selective Color filter to the framebuffer.
pub fn apply_selective_color(fb: &mut Framebuffer, config: &SelectiveColorConfig) {
    let process_pixel = |p: u32| -> u32 {
        let color = Color::from_argb_u32(p);
        let (h, _, _) = color.to_hsv();

        // Calculate the shortest angular distance between the target hue and pixel hue
        let diff = (h - config.target_hue).abs();
        let dist = diff.min(360.0 - diff);

        if dist <= config.tolerance {
            p // Keep original color
        } else {
            // Apply desaturation based on the configuration
            let luma = pixel_luminance(p) as f32 / 255.0;
            let gray_color = Color::new(luma, luma, luma, color.a);

            // Lerp between the original color and grayscale based on desaturation
            let final_color = Color::new(
                color.r + (gray_color.r - color.r) * config.desaturation,
                color.g + (gray_color.g - color.g) * config.desaturation,
                color.b + (gray_color.b - color.b) * config.desaturation,
                color.a,
            );

            final_color.to_argb_u32()
        }
    };

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        pixels.par_iter_mut().for_each(|pixel| {
            *pixel = process_pixel(*pixel);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for pixel in pixels.iter_mut() {
            *pixel = process_pixel(*pixel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_selective_color_retains_target() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Red color
        fb.clear(0xFFFF_0000);

        let config = SelectiveColorConfig {
            target_hue: 0.0, // Red
            tolerance: 30.0,
            desaturation: 1.0,
        };

        apply_selective_color(&mut fb, &config);

        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF_0000));
    }

    #[test]
    fn test_apply_selective_color_grayscales_non_target() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Green color
        fb.clear(0xFF00_FF00);

        let config = SelectiveColorConfig {
            target_hue: 0.0, // Red
            tolerance: 30.0,
            desaturation: 1.0,
        };

        apply_selective_color(&mut fb, &config);

        // Green luma is roughly ~150 in our pixel_luminance logic
        let pixel = fb.get_pixel(0, 0).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        // Ensure it's grayscaled (R == G == B)
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!(r > 0 && r < 255);
    }
}
