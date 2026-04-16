//! Color Splash filter.
//!
//! A post-processing effect that preserves a specific target color (and similar hues)
//! while converting the rest of the image to grayscale, simulating a classic
//! "color splash" or selective color effect.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Color Splash filter.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorSplashConfig {
    /// The target color to preserve (in 0xAARRGGBB format, alpha is ignored).
    pub target_color: u32,
    /// The maximum squared distance in RGB space to still be considered "matching".
    /// Using squared distance avoids `sqrt` calculations in the inner loop.
    pub tolerance_sq: i32,
    /// A feathering distance (squared) to smoothly transition between color and grayscale.
    pub feather_sq: i32,
}

impl Default for ColorSplashConfig {
    fn default() -> Self {
        Self {
            target_color: 0xFF_FF_00_00, // Red
            tolerance_sq: 10_000,
            feather_sq: 5_000,
        }
    }
}

/// Helper to lerp two 8-bit color channels.
#[inline(always)]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a_f = a as f32;
    let b_f = b as f32;
    (a_f + (b_f - a_f) * t).clamp(0.0, 255.0) as u8
}

/// Applies the Color Splash filter to the given framebuffer.
pub fn apply_color_splash(fb: &mut Framebuffer, config: &ColorSplashConfig) {
    let target_r = ((config.target_color >> 16) & 0xFF) as i32;
    let target_g = ((config.target_color >> 8) & 0xFF) as i32;
    let target_b = (config.target_color & 0xFF) as i32;

    let tol_sq = config.tolerance_sq;
    let total_tol_sq = config.tolerance_sq + config.feather_sq;

    // We use a safe denominator for feathering
    let feather_f32 = (config.feather_sq as f32).max(1.0);

    let process_pixel = |pixel: &mut u32| {
        let p = *pixel;
        let r = ((p >> 16) & 0xFF) as i32;
        let g = ((p >> 8) & 0xFF) as i32;
        let b = (p & 0xFF) as i32;

        let dr = r - target_r;
        let dg = g - target_g;
        let db = b - target_b;

        let dist_sq = dr * dr + dg * dg + db * db;

        if dist_sq <= tol_sq {
            // Keep original color
            return;
        }

        let luma_u8 = pixel_luminance(p);
        let grayscale_pixel = (0xFF << 24) | ((luma_u8 as u32) << 16) | ((luma_u8 as u32) << 8) | (luma_u8 as u32);

        if dist_sq >= total_tol_sq {
            // Full grayscale
            *pixel = grayscale_pixel;
        } else {
            // Feathering
            let over_tol = (dist_sq - tol_sq) as f32;
            let t = (over_tol / feather_f32).clamp(0.0, 1.0);

            // smoothstep for a nicer transition
            let t = t * t * (3.0 - 2.0 * t);

            let out_r = lerp_u8(r as u8, luma_u8, t);
            let out_g = lerp_u8(g as u8, luma_u8, t);
            let out_b = lerp_u8(b as u8, luma_u8, t);

            *pixel = (0xFF << 24) | ((out_r as u32) << 16) | ((out_g as u32) << 8) | (out_b as u32);
        }
    };

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        pixels.par_chunks_exact_mut(1024).for_each(|chunk| {
            for p in chunk {
                process_pixel(p);
            }
        });

        let rem = pixels.len() % 1024;
        if rem > 0 {
            let start = pixels.len() - rem;
            for p in &mut pixels[start..] {
                process_pixel(p);
            }
        }
    }

    #[cfg(not(feature = "parallel"))]
    {
        for p in pixels.iter_mut() {
            process_pixel(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_splash() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF_FF_00_00); // Pure red
        fb.set_pixel(1, 0, 0xFF_00_FF_00); // Pure green
        fb.set_pixel(2, 0, 0xFF_00_00_FF); // Pure blue

        let config = ColorSplashConfig {
            target_color: 0xFF_FF_00_00, // Red
            tolerance_sq: 1000,
            feather_sq: 0,
        };

        apply_color_splash(&mut fb, &config);

        let pixels = fb.as_slice();

        // Red should be preserved
        assert_eq!(pixels[0], 0xFF_FF_00_00);

        // Green should be grayscale. Luminance of green (0xFF) is roughly 0.587 * 255 = 150 (0x96)
        let luma_g = pixel_luminance(0xFF_00_FF_00) as u32;
        let expected_g = (0xFF << 24) | (luma_g << 16) | (luma_g << 8) | luma_g;
        assert_eq!(pixels[1], expected_g);

        // Blue should be grayscale. Luminance of blue (0xFF) is roughly 0.114 * 255 = 29 (0x1D)
        let luma_b = pixel_luminance(0xFF_00_00_FF) as u32;
        let expected_b = (0xFF << 24) | (luma_b << 16) | (luma_b << 8) | luma_b;
        assert_eq!(pixels[2], expected_b);
    }
}
