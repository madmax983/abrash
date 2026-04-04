//! Duotone Post-Processing Effect
//!
//! Maps pixel luminance to a gradient between two colors.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Duotone effect.
#[derive(Debug, Clone, Copy)]
pub struct DuotoneConfig {
    /// Color to map to dark (low luminance) pixels.
    pub dark_color: u32,
    /// Color to map to light (high luminance) pixels.
    pub light_color: u32,
}

impl Default for DuotoneConfig {
    fn default() -> Self {
        Self {
            dark_color: 0xFF_191414,  // Spotify dark
            light_color: 0xFF_1DB954, // Spotify green
        }
    }
}

/// Applies a Duotone filter to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the Duotone effect.
pub fn apply_duotone(fb: &mut Framebuffer, config: &DuotoneConfig) {
    let width = fb.width() as usize;
    if width == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    let dark_a = (config.dark_color >> 24) & 0xFF;
    let dark_r = (config.dark_color >> 16) & 0xFF;
    let dark_g = (config.dark_color >> 8) & 0xFF;
    let dark_b = config.dark_color & 0xFF;

    let light_a = (config.light_color >> 24) & 0xFF;
    let light_r = (config.light_color >> 16) & 0xFF;
    let light_g = (config.light_color >> 8) & 0xFF;
    let light_b = config.light_color & 0xFF;

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width);
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width);

    iter.for_each(|row| {
        for pixel in row.iter_mut() {
            let luma = u32::from(pixel_luminance(*pixel));
            // Ensure alpha is fully preserved from blending if required,
            // but usually we keep original alpha or blend it.
            // A simple lerp: V = dark + (light - dark) * luma / 255

            let inv_luma = 255 - luma;

            let new_a = (dark_a * inv_luma + light_a * luma) / 255;
            let new_r = (dark_r * inv_luma + light_r * luma) / 255;
            let new_g = (dark_g * inv_luma + light_g * luma) / 255;
            let new_b = (dark_b * inv_luma + light_b * luma) / 255;

            *pixel = (new_a << 24) | (new_r << 16) | (new_g << 8) | new_b;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_duotone() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        // One black pixel, one white pixel
        fb.set_pixel(0, 0, 0xFF_000000);
        fb.set_pixel(1, 0, 0xFF_FFFFFF);

        let config = DuotoneConfig {
            dark_color: 0xFF_FF0000,  // Red
            light_color: 0xFF_0000FF, // Blue
        };

        apply_duotone(&mut fb, &config);

        // Expect the black pixel to become red, and white pixel to become blue
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_FF0000);
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFF_0000FF);
    }
}
