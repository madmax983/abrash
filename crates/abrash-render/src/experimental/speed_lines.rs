//! Speed Lines Post-Processing Effect.
//!
//! A procedural effect that simulates anime/comic style radial "speed lines"
//! radiating outwards from a central point. The lines are generated using a
//! pseudo-random hash function based on the angle and distance from the center.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Speed Lines effect.
#[derive(Debug, Clone, Copy)]
pub struct SpeedLinesConfig {
    /// The X coordinate of the center (0.0 to 1.0).
    pub center_x: f32,
    /// The Y coordinate of the center (0.0 to 1.0).
    pub center_y: f32,
    /// Color of the speed lines.
    pub line_color: u32,
    /// Seed used for procedural generation, or time value for animation.
    pub time: f32,
    /// The radius (in pixels) of the central "safe zone" where no lines appear.
    pub inner_radius: f32,
    /// The density of the lines (number of angular sectors).
    pub density: u32,
    /// Minimum length of the lines.
    pub min_length: f32,
    /// Maximum length variance of the lines.
    pub max_length: f32,
}

impl Default for SpeedLinesConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            line_color: 0xFFFF_FFFF,
            time: 0.0,
            inner_radius: 100.0,
            density: 360,
            min_length: 50.0,
            max_length: 300.0,
        }
    }
}

/// Computes a pseudo-random float between 0.0 and 1.0 based on two integer seeds.
#[inline(always)]
fn hash_2d(x: u32, y: u32) -> f32 {
    let mut state = x
        .wrapping_mul(747_796_405)
        .wrapping_add(y.wrapping_mul(283_927_953));
    let mut rng = XorShift32::new(state);
    let r = rng.next_u32();
    // Normalize to 0.0 .. 1.0
    (r as f32) / (u32::MAX as f32)
}

/// Applies a radial speed lines effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the speed lines.
pub fn apply_speed_lines(fb: &mut Framebuffer, config: &SpeedLinesConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let w_f32 = width as f32;
    let h_f32 = height as f32;
    let cx = config.center_x * w_f32;
    let cy = config.center_y * h_f32;

    let time_seed = (config.time * 10.0) as u32;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let dy = (y as f32) - cy;

        let inner_radius_sq = config.inner_radius * config.inner_radius;
        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let dx = (x as f32) - cx;

            // Calculate squared distance from center
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < inner_radius_sq {
                continue; // Inside the safe zone
            }

            // Calculate angle (-PI to PI)
            let mut angle = abrash_core::math::fast_atan2(dy, dx);
            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }

            // Normalize angle to [0, density]
            let angle_normalized = angle / std::f32::consts::TAU;
            let sector = (angle_normalized * config.density as f32).floor() as u32;

            // Generate a random length variation for this sector
            let noise = hash_2d(sector, time_seed);

            // Random thickness to make some lines thicker
            let noise2 = hash_2d(sector.wrapping_add(1337), time_seed);

            // Determine line start distance
            let start_dist = config.inner_radius
                + config.min_length
                + noise * (config.max_length - config.min_length);

            if dist_sq > start_dist * start_dist {
                // If it passes the start distance, check if we're inside the line thickness
                // We want lines to occupy only a part of the sector
                let sector_fraction = (angle_normalized * config.density as f32).fract();
                let thickness = 0.1 + noise2 * 0.4; // 10% to 50% of the sector

                if sector_fraction < thickness {
                    // Blend alpha (simple replace for now if alpha is 255)
                    let a = (config.line_color >> 24) & 0xFF;
                    if a == 255 {
                        *pixel = config.line_color;
                    } else if a > 0 {
                        // Blend
                        let bg = *pixel;
                        let r1 = (config.line_color >> 16) & 0xFF;
                        let g1 = (config.line_color >> 8) & 0xFF;
                        let b1 = config.line_color & 0xFF;

                        let r2 = (bg >> 16) & 0xFF;
                        let g2 = (bg >> 8) & 0xFF;
                        let b2 = bg & 0xFF;

                        let inv_a = 255 - a;
                        let r = (r1 * a + r2 * inv_a) / 255;
                        let g = (g1 * a + g2 * inv_a) / 255;
                        let b = (b1 * a + b2 * inv_a) / 255;

                        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_lines_basic() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000); // Black background

        let mut config = SpeedLinesConfig::default();
        config.inner_radius = 10.0;
        config.line_color = 0xFFFF_FFFF; // White lines

        apply_speed_lines(&mut fb, &config);

        // Center should be untouched (black)
        assert_eq!(fb.get_pixel(50, 50).unwrap(), 0xFF00_0000);

        // Some pixels further out should be white (lines)
        let mut has_white = false;
        for &p in fb.as_slice() {
            if p == 0xFFFF_FFFF {
                has_white = true;
                break;
            }
        }
        assert!(
            has_white,
            "Speed lines should draw white pixels outside the inner radius"
        );
    }

    #[test]
    fn test_speed_lines_alpha_blend() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000); // Black

        let mut config = SpeedLinesConfig::default();
        config.inner_radius = 5.0;
        config.min_length = 5.0;
        config.max_length = 5.0;
        // Semi-transparent red: A=128 (approx 50%), R=255
        config.line_color = 0x80FF_0000;

        apply_speed_lines(&mut fb, &config);

        let mut has_blend = false;
        for &p in fb.as_slice() {
            if p != 0xFF00_0000 {
                // It should be a blended dark red, e.g., 0xFF800000
                let r = (p >> 16) & 0xFF;
                assert!(r > 0 && r < 255);
                has_blend = true;
                break;
            }
        }
        assert!(has_blend);
    }
}
