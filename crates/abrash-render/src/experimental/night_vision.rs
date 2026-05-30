//! Night Vision Filter
//!
//! A retro post-processing effect simulating analog night vision goggles.
//! It amplifies luminance non-linearly, applies a green phosphor tint,
//! adds high-frequency noise, scanlines, and a vignette.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Night Vision effect.
#[derive(Debug, Clone, Copy)]
pub struct NightVisionConfig {
    /// Amplification factor for low light (e.g., 1.5).
    pub amplification: f32,
    /// Overall green tint intensity (0.0 to 1.0).
    pub green_tint: f32,
    /// Intensity of the procedural noise/snow (0.0 to 1.0).
    pub noise_intensity: f32,
    /// Time variable used for animating noise.
    pub time: f32,
}

impl Default for NightVisionConfig {
    fn default() -> Self {
        Self {
            amplification: 1.5,
            green_tint: 1.0,
            noise_intensity: 0.2,
            time: 0.0,
        }
    }
}

/// Applies a night vision post-processing effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_night_vision(fb: &mut Framebuffer, config: &NightVisionConfig) {
    if config.green_tint <= 0.0 && config.noise_intensity <= 0.0 {
        return; // Nothing to do
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let half_w = width as f32 * 0.5;
    let half_h = height as f32 * 0.5;
    let max_dist_sq = half_w * half_w + half_h * half_h;

    // Scale max distance down slightly to make vignette stronger at edges
    let max_dist_sq = max_dist_sq * 0.9;

    let noise_seed = (config.time * 1000.0) as u32 ^ 0x5555_5555;

    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width.max(1)).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width.max(1)).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f = y as f32;
        let dy = y_f - half_h;
        let dy_sq = dy * dy;

        let row_noise_base = noise_seed.wrapping_add((y as u32).wrapping_mul(7919));

        for (x, pixel) in row.iter_mut().enumerate() {
            let p = *pixel;
            let r = ((p >> 16) & 0xFF) as f32 / 255.0;
            let g = ((p >> 8) & 0xFF) as f32 / 255.0;
            let b = (p & 0xFF) as f32 / 255.0;

            // 1. Calculate luminance (Standard Rec. 601)
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;

            // 2. Light Amplification (boost darks using a fast sqrt curve approximation ~ x^0.75)
            // A curve < 1.0 boosts lower values more than higher values.
            // Note: x^0.75 is a close and fast approximation to x^0.65
            let sqrt_lum = lum.sqrt();
            let amplified = (sqrt_lum * sqrt_lum.sqrt() * config.amplification).clamp(0.0, 1.0);

            // 3. Green Phosphor Tint (P43 Phosphor roughly)
            // Mostly green, some blue, tiny bit of red
            let mut out_r = amplified * 0.1 * config.green_tint;
            let mut out_g = amplified * 0.95 * config.green_tint;
            let mut out_b = amplified * 0.2 * config.green_tint;

            // Blend with original if green_tint < 1.0
            if config.green_tint < 1.0 {
                out_r = r * (1.0 - config.green_tint) + out_r;
                out_g = g * (1.0 - config.green_tint) + out_g;
                out_b = b * (1.0 - config.green_tint) + out_b;
            }

            // 4. Procedural High-Frequency Noise
            if config.noise_intensity > 0.0 {
                // Simple fast hash
                let n = (x as u32).wrapping_mul(1973).wrapping_add(row_noise_base);
                // Hash to float 0.0-1.0
                let noise_val = (n.wrapping_mul(2_654_435_761) >> 16) as f32 / 65535.0;
                // Shift to -0.5 to +0.5 range and scale
                let noise_offset = (noise_val - 0.5) * config.noise_intensity;

                out_r += noise_offset;
                out_g += noise_offset;
                out_b += noise_offset;
            }

            // 5. Faint Scanlines (darken every other row)
            if y % 2 == 0 {
                out_r *= 0.9;
                out_g *= 0.9;
                out_b *= 0.9;
            }

            // 6. Vignette
            let dx = x as f32 - half_w;
            let dist_sq = dx * dx + dy_sq;
            let dist_norm_sq = dist_sq / max_dist_sq;
            let vignette = (1.0 - dist_norm_sq).max(0.0); // 1.0 at center, 0.0 at corners
            let vignette = (vignette * vignette * (3.0 - 2.0 * vignette)).clamp(0.0, 1.0); // Smooth falloff approx without sqrt

            out_r *= vignette;
            out_g *= vignette;
            out_b *= vignette;

            // Write back to row
            let final_r = (out_r.clamp(0.0, 1.0) * 255.0) as u32;
            let final_g = (out_g.clamp(0.0, 1.0) * 255.0) as u32;
            let final_b = (out_b.clamp(0.0, 1.0) * 255.0) as u32;

            *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_night_vision_boosts_luminance_and_tints_green() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Very dark gray pixel
        fb.clear(0xFF20_2020);

        let config = NightVisionConfig {
            amplification: 2.0,   // High amplification
            green_tint: 1.0,      // Full green
            noise_intensity: 0.0, // No noise for deterministic test
            time: 0.0,
        };

        apply_night_vision(&mut fb, &config);

        // Center pixel (least affected by vignette)
        let pixel = fb.get_pixel(5, 5).unwrap();

        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        // Ensure it's significantly brighter than 0x20 (32)
        // And ensure it's overwhelmingly green
        assert!(g > 100, "Green channel was not amplified enough: {g}");
        assert!(g > r * 5, "Green channel is not dominant over red");
        assert!(g > b * 2, "Green channel is not dominant over blue");
    }

    #[test]
    fn test_night_vision_vignette_darkens_corners() {
        let width = 20;
        let height = 20;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Pure white
        fb.clear(0xFFFF_FFFF);

        let config = NightVisionConfig {
            amplification: 1.0,
            green_tint: 1.0,
            noise_intensity: 0.0,
            time: 0.0,
        };

        apply_night_vision(&mut fb, &config);

        // Center pixel
        let center_pixel = fb.get_pixel(10, 10).unwrap();
        let center_g = (center_pixel >> 8) & 0xFF;

        // Corner pixel
        let corner_pixel = fb.get_pixel(0, 0).unwrap();
        let corner_g = (corner_pixel >> 8) & 0xFF;

        assert!(
            corner_g < center_g,
            "Vignette did not darken corners: center={center_g}, corner={corner_g}"
        );
    }
}
