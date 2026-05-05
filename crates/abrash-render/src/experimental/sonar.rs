//! Sonar / Echolocation Filter
//!
//! A post-processing effect that visualizes depth as sweeping sonar waves radiating
//! from the camera over time.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Sonar filter.
#[derive(Debug, Clone, Copy)]
pub struct SonarConfig {
    /// The color of the sonar wave (ARGB format).
    pub wave_color: u32,
    /// The base color for geometry not currently in a wave (ARGB format).
    pub base_color: u32,
    /// Background color for infinite depth or beyond max distance.
    pub background_color: u32,
    /// Distance between waves.
    pub wave_spacing: f32,
    /// Thickness of each wave band (0.0 to 1.0).
    pub wave_thickness: f32,
    /// How fast the waves move outward from the camera.
    pub wave_speed: f32,
    /// Current time, used to animate the waves.
    pub time: f32,
    /// Maximum distance the sonar can reach before fading out.
    pub max_distance: f32,
}

impl Default for SonarConfig {
    fn default() -> Self {
        Self {
            wave_color: 0xFF_00_FF_00,       // Neon Green
            base_color: 0xFF_00_22_00,       // Dark Green
            background_color: 0xFF_00_00_00, // Black
            wave_spacing: 10.0,
            wave_thickness: 0.1,
            wave_speed: 5.0,
            time: 0.0,
            max_distance: 100.0,
        }
    }
}

/// Applies a sonar/echolocation effect based on the z-buffer depths.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `zb` - The z-buffer containing depth values for each pixel.
/// * `config` - Configuration for the sonar effect.
pub fn apply_sonar(fb: &mut Framebuffer, zb: &ZBuffer, config: &SonarConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let fb_slice = fb.as_mut_slice();
    let zb_slice = zb.as_slice();

    // Precalculate wave frequency.
    let frequency = std::f32::consts::TAU / config.wave_spacing;
    let phase_shift = config.time * config.wave_speed * frequency;

    // Precalculate colors outside hot loop
    let c_base = Color::from_argb_u32(config.base_color);
    let c_wave = Color::from_argb_u32(config.wave_color);
    let c_bg = Color::from_argb_u32(config.background_color);

    #[cfg(feature = "parallel")]
    let iter = fb_slice.par_iter_mut().zip(zb_slice.par_iter());

    #[cfg(not(feature = "parallel"))]
    let iter = fb_slice.iter_mut().zip(zb_slice.iter());

    iter.for_each(|(pixel, depth)| {
        if depth.is_infinite() {
            *pixel = config.background_color;
            return;
        }

        if *depth > config.max_distance {
            *pixel = config.background_color;
            return;
        }

        // Calculate a sine wave based on distance
        let wave_val = (*depth * frequency - phase_shift).sin();

        // Map sine output into a narrow band of intensity (0.0 to 1.0)
        let threshold = 1.0 - config.wave_thickness.clamp(0.001, 1.0);

        let intensity = if wave_val > threshold {
            (wave_val - threshold) / (1.0 - threshold)
        } else {
            0.0
        };

        // Fade out intensity as it approaches max_distance
        let distance_fade = 1.0 - (*depth / config.max_distance).clamp(0.0, 1.0);
        let final_intensity = intensity * distance_fade;

        // Darken base color based on distance fade so the scene fades into the background
        let faded_base = c_base.lerp(c_bg, 1.0 - distance_fade);

        *pixel = faded_base.lerp(c_wave, final_intensity).to_argb_u32();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sonar_effect_infinite_depth() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zb = ZBuffer::new(1, 1).unwrap();

        fb.clear(0xFFFF_0000); // Red

        let config = SonarConfig::default();
        apply_sonar(&mut fb, &zb, &config);

        assert_eq!(fb.get_pixel(0, 0), Some(config.background_color));
    }

    #[test]
    fn test_sonar_effect_wave_peak() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zb = ZBuffer::new(1, 1).unwrap();

        let config = SonarConfig {
            wave_color: 0xFF_FF_FF_FF, // White wave
            base_color: 0xFF_00_00_00, // Black base
            background_color: 0xFF_00_00_00,
            wave_spacing: std::f32::consts::TAU, // frequency = 1.0
            wave_thickness: 0.1,
            wave_speed: 0.0,
            time: 0.0,
            max_distance: 100.0,
        };

        // Sine is max at PI/2
        zb.test_and_set(0, 0, std::f32::consts::PI / 2.0);

        apply_sonar(&mut fb, &zb, &config);

        // At peak, it should be close to the wave color (White)
        let color = fb.get_pixel(0, 0).unwrap();
        let r = (color >> 16) & 0xFF;
        assert!(r > 200, "Should be intensely lit by the wave: found {}", r);
    }
}
