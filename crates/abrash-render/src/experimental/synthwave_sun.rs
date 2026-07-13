//! Synthwave Sun Effect
//!
//! A retro post-processing effect that renders a procedurally generated 80s outrun-style sun
//! with a vertical gradient and horizontal scanline cutouts that increase in size towards the bottom.

#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;

/// Configuration for the Synthwave Sun effect.
#[derive(Debug, Clone, Copy)]
pub struct SynthwaveSunConfig {
    /// Center X coordinate of the sun.
    pub center_x: i32,
    /// Center Y coordinate of the sun.
    pub center_y: i32,
    /// Radius of the sun in pixels.
    pub radius: i32,
    /// Top color of the gradient (ARGB).
    pub top_color: u32,
    /// Bottom color of the gradient (ARGB).
    pub bottom_color: u32,
    /// Base frequency of the cutouts.
    pub cutout_frequency: f32,
}

impl Default for SynthwaveSunConfig {
    fn default() -> Self {
        Self {
            center_x: 400,
            center_y: 300,
            radius: 150,
            top_color: 0xFF_FF_E6_00,    // Bright Yellow
            bottom_color: 0xFF_FF_00_55, // Hot Pink
            cutout_frequency: 0.05,
        }
    }
}

/// Applies a procedural Synthwave Sun to the framebuffer.
///
/// Draws a circle with a vertical color gradient and horizontal cutouts.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the sun.
pub fn apply_synthwave_sun(fb: &mut Framebuffer, config: &SynthwaveSunConfig) {
    if config.radius <= 0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    let r_squared = config.radius * config.radius;
    let start_y = (config.center_y - config.radius).max(0);
    let end_y = (config.center_y + config.radius).min(height - 1);
    let start_x = (config.center_x - config.radius).max(0);
    let end_x = (config.center_x + config.radius).min(width - 1);

    // Unpack colors
    let t_a = (config.top_color >> 24) & 0xFF;
    let t_r = (config.top_color >> 16) & 0xFF;
    let t_g = (config.top_color >> 8) & 0xFF;
    let t_b = config.top_color & 0xFF;

    let b_a = (config.bottom_color >> 24) & 0xFF;
    let b_r = (config.bottom_color >> 16) & 0xFF;
    let b_g = (config.bottom_color >> 8) & 0xFF;
    let b_b = config.bottom_color & 0xFF;

    let d_a = (b_a as f32) - (t_a as f32);
    let d_r = (b_r as f32) - (t_r as f32);
    let d_g = (b_g as f32) - (t_g as f32);
    let d_b = (b_b as f32) - (t_b as f32);

    for y in start_y..=end_y {
        let dy = y - config.center_y;

        // Calculate gradient interpolation factor (0.0 at top, 1.0 at bottom)
        let t = ((dy as f32) / (config.radius as f32 * 2.0)) + 0.5;

        // Determine if this row is cut out
        // Cutouts only happen in the bottom half usually, or get thicker at the bottom
        // We use a sine wave mapped against the y-coordinate.
        // We modify the phase so the cutouts align nicely.
        let phase = dy as f32 * config.cutout_frequency;
        let sine_val = phase.sin();

        // Threshold for cutout, gets higher (more restrictive) towards the top.
        // Towards the bottom, threshold drops, causing thicker lines.
        let cutout_threshold = 1.0 - (t.max(0.0).min(1.0) * 1.5);

        // Skip rendering this row if it falls into a cutout
        if t > 0.3 && sine_val > cutout_threshold {
            continue;
        }

        // Compute row color
        let a = (t_a as f32 + d_a * t).clamp(0.0, 255.0) as u32;
        let r = (t_r as f32 + d_r * t).clamp(0.0, 255.0) as u32;
        let g = (t_g as f32 + d_g * t).clamp(0.0, 255.0) as u32;
        let b = (t_b as f32 + d_b * t).clamp(0.0, 255.0) as u32;
        let color = (a << 24) | (r << 16) | (g << 8) | b;

        // Determine row width
        // x^2 + y^2 = r^2  => x^2 = r^2 - y^2
        let row_w_sq = r_squared - (dy * dy);
        if row_w_sq < 0 {
            continue;
        }

        let row_half_w = (row_w_sq as f32).sqrt() as i32;

        let row_start_x = (config.center_x - row_half_w).max(start_x);
        let row_end_x = (config.center_x + row_half_w).min(end_x);

        for x in row_start_x..=row_end_x {
            fb.set_pixel(x, y, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthwave_sun_renders() {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        fb.clear(0xFF_00_00_00); // Clear to Black

        let config = SynthwaveSunConfig::default();
        apply_synthwave_sun(&mut fb, &config);

        // Center should be colored
        let center_pixel = fb.get_pixel(400, 300).unwrap();
        assert_ne!(center_pixel, 0xFF_00_00_00);

        // Top edge should be closer to top_color
        let top_pixel = fb.get_pixel(400, 151).unwrap();
        assert_ne!(top_pixel, 0xFF_00_00_00);

        // Far outside should be black
        let out_pixel = fb.get_pixel(100, 100).unwrap();
        assert_eq!(out_pixel, 0xFF_00_00_00);
    }
}
