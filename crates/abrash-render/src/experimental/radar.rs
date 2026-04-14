//! Radar Sweep Post-Processing Effect
//!
//! Simulates a retro radar screen with a sweeping beam, fading trail,
//! concentric distance rings, and a green tint applied to the scene's luminance.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Radar effect.
#[derive(Debug, Clone, Copy)]
pub struct RadarConfig {
    /// Radar beam sweep speed (radians per second/time unit).
    pub speed: f32,
    /// Color of the radar beam and overlay (default: bright green).
    pub color: u32,
    /// Length of the fading tail behind the sweep line (in radians).
    pub tail_length: f32,
    /// Spacing between concentric distance rings (in pixels).
    pub ring_spacing: f32,
    /// Base intensity of the underlying scene (0.0 to 1.0).
    pub background_intensity: f32,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            speed: 2.0,
            color: 0xFF_00_FF_00,              // Bright green
            tail_length: std::f32::consts::PI, // Half a circle
            ring_spacing: 50.0,
            background_intensity: 0.3,
        }
    }
}

/// Applies a radar sweep overlay and tint to the framebuffer.
pub fn apply_radar(fb: &mut Framebuffer, time: f32, config: &RadarConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // Current angle of the sweeping beam
    let mut current_angle = (time * config.speed) % (2.0 * std::f32::consts::PI);
    if current_angle < 0.0 {
        current_angle += 2.0 * std::f32::consts::PI;
    }

    let r_col = ((config.color >> 16) & 0xFF) as f32;
    let g_col = ((config.color >> 8) & 0xFF) as f32;
    let b_col = (config.color & 0xFF) as f32;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let dy = y as f32 - cy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = x as f32 - cx;

            // Polar coordinates
            let distance = (dx * dx + dy * dy).sqrt();
            let mut angle = dy.atan2(dx);
            if angle < 0.0 {
                angle += 2.0 * std::f32::consts::PI;
            }

            // Calculate angle difference for the sweep tail
            let mut angle_diff = current_angle - angle;
            if angle_diff < 0.0 {
                angle_diff += 2.0 * std::f32::consts::PI;
            }

            // Calculate beam intensity
            let beam_intensity = if angle_diff < config.tail_length {
                1.0 - (angle_diff / config.tail_length)
            } else {
                0.0
            };

            // Rings
            let mut ring_intensity = 0.0;
            if config.ring_spacing > 0.0 {
                let ring_mod = distance % config.ring_spacing;
                if ring_mod < 1.0 {
                    ring_intensity = 0.5; // faint ring
                }
            }

            // Crosshair
            let mut crosshair_intensity = 0.0;
            if dx.abs() < 1.0 || dy.abs() < 1.0 {
                crosshair_intensity = 0.3; // faint crosshair
            }

            // Combine overlay intensities
            let overlay = (beam_intensity + ring_intensity + crosshair_intensity).min(1.0);

            // Original scene intensity
            let luma = pixel_luminance(*pixel) as f32 / 255.0;
            let scene_intensity = luma * config.background_intensity;

            let final_intensity = (scene_intensity + overlay).min(1.0);

            // Apply color
            let r = (r_col * final_intensity) as u32;
            let g = (g_col * final_intensity) as u32;
            let b = (b_col * final_intensity) as u32;

            *pixel = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radar_basic() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_FF_FF_FF); // White background
        let config = RadarConfig::default();
        apply_radar(&mut fb, 0.0, &config);

        // Ensure some pixels are modified and not white
        let center = fb.get_pixel(50, 50).unwrap();
        assert_ne!(center, 0xFF_FF_FF_FF);

        // Since color is green, red and blue should be 0 (or close to 0)
        let r = (center >> 16) & 0xFF;
        let g = (center >> 8) & 0xFF;
        let b = center & 0xFF;
        assert_eq!(r, 0);
        assert!(g > 0);
        assert_eq!(b, 0);
    }
}
