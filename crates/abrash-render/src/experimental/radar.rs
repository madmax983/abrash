//! Radar post-processing effect.
//!
//! Simulates a sweeping radar scope.

use crate::framebuffer::Framebuffer;

/// Configuration for the radar effect.
#[derive(Debug, Clone, Copy)]
pub struct RadarConfig {
    /// Screen-space X coordinate of the radar center.
    pub center_x: i32,
    /// Screen-space Y coordinate of the radar center.
    pub center_y: i32,
    /// Radius of the radar display.
    pub radius: i32,
    /// Current angle of the sweeping beam (in radians).
    pub angle: f32,
    /// Length of the glowing trail behind the beam (in radians).
    pub trail_length: f32,
    /// Color of the sweeping beam and blips (AARRGGBB).
    pub color: u32,
    /// Color of the grid/background (AARRGGBB).
    pub grid_color: u32,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_y: 0,
            radius: 100,
            angle: 0.0,
            trail_length: std::f32::consts::PI / 2.0,
            color: 0xFF00_FF00,
            grid_color: 0xFF00_4400,
        }
    }
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies the radar effect to the framebuffer.
pub fn apply_radar(fb: &mut Framebuffer, config: &RadarConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    let center_x = config.center_x;
    let center_y = config.center_y;
    let radius = config.radius;
    let radius_sq = radius * radius;

    // Normalize angle to [0, 2pi)
    let mut current_angle = config.angle % (2.0 * std::f32::consts::PI);
    if current_angle < 0.0 {
        current_angle += 2.0 * std::f32::consts::PI;
    }

    let process_row = |(y, row): (usize, &mut [u32])| {
        let dy = (y as i32) - center_y;
        let dy_sq = dy * dy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = (x as i32) - center_x;
            let dx_sq = dx * dx;

            let dist_sq = dx_sq + dy_sq;

            if dist_sq <= radius_sq {
                let dist = (dist_sq as f32).sqrt();

                let mut r = (config.grid_color >> 16) & 0xFF;
                let mut g = (config.grid_color >> 8) & 0xFF;
                let mut b = config.grid_color & 0xFF;

                let cr = (config.color >> 16) & 0xFF;
                let cg = (config.color >> 8) & 0xFF;
                let cb = config.color & 0xFF;

                // Grid lines (concentric circles)
                let grid_spacing = radius as f32 / 4.0;
                let grid_mod = dist % grid_spacing;
                if grid_mod < 1.0 || grid_mod > grid_spacing - 1.0 {
                    r = r.midpoint(cr);
                    g = g.midpoint(cg);
                    b = b.midpoint(cb);
                }

                // Crosshairs
                if dx == 0 || dy == 0 {
                    r = r.midpoint(cr);
                    g = g.midpoint(cg);
                    b = b.midpoint(cb);
                }

                // Sweeping beam
                // ⚡ Bolt: Use fast_atan2 to bypass expensive float trig
                let mut pixel_angle = abrash_core::math::fast_atan2(dy as f32, dx as f32);

                let mut angle_diff = current_angle - pixel_angle;
                if angle_diff < 0.0 {
                    angle_diff += 2.0 * std::f32::consts::PI;
                }

                if angle_diff <= config.trail_length {
                    let intensity = 1.0 - (angle_diff / config.trail_length);
                    // Fade trail based on intensity
                    let alpha = (intensity * 256.0) as u32;
                    let inv_alpha = 256 - alpha;
                    r = (r * inv_alpha + cr * alpha) >> 8;
                    g = (g * inv_alpha + cg * alpha) >> 8;
                    b = (b * inv_alpha + cb * alpha) >> 8;
                }

                *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
    };

    #[cfg(feature = "parallel")]
    fb.as_mut_slice()
        .par_chunks_exact_mut(width)
        .enumerate()
        .for_each(process_row);

    #[cfg(not(feature = "parallel"))]
    fb.as_mut_slice()
        .chunks_exact_mut(width)
        .enumerate()
        .for_each(process_row);
}
