//! Snow Overlay Filter
//!
//! A retro post-processing effect that overlays falling snow on the framebuffer.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Snow effect.
#[derive(Debug, Clone, Copy)]
pub struct SnowConfig {
    /// Density of the snow (0.0 to 1.0).
    pub density: f32,
    /// Speed of the falling snow.
    pub speed: f32,
    /// Time variable to animate the falling snow.
    pub time: f32,
    /// General intensity of the snow overlay.
    pub intensity: f32,
    /// Horizontal wind displacement.
    pub wind: f32,
}

impl Default for SnowConfig {
    fn default() -> Self {
        Self {
            density: 0.1,
            speed: 1.0,
            time: 0.0,
            intensity: 1.0,
            wind: 0.2,
        }
    }
}

/// Applies a falling snow effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_snow(fb: &mut Framebuffer, config: &SnowConfig) {
    if config.density <= 0.0 || config.intensity <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let time = config.time;
    let speed = config.speed;
    let density = config.density.clamp(0.0, 1.0);
    let wind = config.wind;

    // Density controls the probability of a pixel being snow.
    // 0.0 = no snow, 1.0 = solid white. A nice blizzard is around 0.01 - 0.05.
    // However, the test uses 0.5. To make it look natural we scale it.
    let probability_threshold = (density * 255.0) as u32; // Simplified probability
    let base_seed = (time * 1000.0) as u32 ^ 0x51E1_0101;

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        for (x, pixel) in row.iter_mut().enumerate() {
            // We want the snow to fall and blow with the wind.
            // So we sample noise based on offset coordinates.
            let x_offset = (time * speed * wind * 100.0) as i32;
            let y_offset = (time * speed * 100.0) as i32;

            // Simple noise function based on coordinates
            let sample_x = (x as i32 - x_offset).rem_euclid(width as i32) as u32;
            let sample_y = (y as i32 - y_offset).rem_euclid(height as i32) as u32;

            // Generate noise based on the sample coordinates to make flakes "stick" and move
            let mut noise_state = sample_x
                .wrapping_mul(1_973)
                .wrapping_add(sample_y.wrapping_mul(9_277))
                .wrapping_add(1_337);

            if noise_state == 0 {
                noise_state = 1;
            }

            // Mix in the global PRNG just for variety if needed, but a coordinate-based hash
            // is better for moving particles so they don't flicker.
            let mut coord_prng = XorShift32::new(noise_state);
            let val = coord_prng.next_u32() % 25_500; // Map to 0-25500 for finer probability

            // Scale threshold for visual density. A max density of 1.0 should mean probability 25500.
            let scaled_threshold = probability_threshold * 100;

            if val < scaled_threshold {
                // Vary opacity based on the noise value itself for depth
                let flake_alpha = (val % 255) as f32 / 255.0;
                let final_alpha = (flake_alpha * config.intensity).clamp(0.0, 1.0);

                // Simple alpha blending with white
                let p = *pixel;
                let r = ((p >> 16) & 0xFF) as f32;
                let g = ((p >> 8) & 0xFF) as f32;
                let b = (p & 0xFF) as f32;

                let out_r = r + (255.0 - r) * final_alpha;
                let out_g = g + (255.0 - g) * final_alpha;
                let out_b = b + (255.0 - b) * final_alpha;

                *pixel = (p & 0xFF00_0000)
                    | ((out_r as u32) << 16)
                    | ((out_g as u32) << 8)
                    | (out_b as u32);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snow_changes_pixels() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF00_0000); // Black background

        let config = SnowConfig {
            density: 0.5,
            speed: 1.0,
            time: 1.0,
            intensity: 1.0,
            wind: 0.0,
        };

        apply_snow(&mut fb, &config);

        let mut has_snow = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF00_0000 {
                has_snow = true;
                break;
            }
        }
        assert!(has_snow, "Snow filter did not change any pixels");
    }

    #[test]
    fn test_snow_zero_density_no_change() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF12_3456);

        let config = SnowConfig {
            density: 0.0,
            ..Default::default()
        };

        apply_snow(&mut fb, &config);

        for &pixel in fb.as_slice() {
            assert_eq!(
                pixel, 0xFF12_3456,
                "Snow filter changed pixels when density was zero"
            );
        }
    }
}
