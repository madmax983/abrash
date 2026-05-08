#![cfg(feature = "nova")]
//! Synthwave / Vaporwave 3D Grid Filter
//!
//! A retro-futuristic post-processing effect that renders a classic
//! synthwave 3D wireframe floor grid with a horizon and a glowing "sun".

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Synthwave filter.
#[derive(Debug, Clone, Copy)]
pub struct SynthwaveConfig {
    /// The RGB color of the grid lines (e.g., Neon Pink or Cyan).
    pub grid_color: u32,
    /// The RGB color of the sun (e.g., Hot Pink or Orange).
    pub sun_color: u32,
    /// Background color of the sky (e.g., Dark Purple).
    pub sky_color: u32,
    /// Background color of the ground (e.g., Black).
    pub ground_color: u32,
    /// The Y coordinate of the horizon (0.0 to 1.0, where 0.5 is the middle).
    pub horizon: f32,
    /// The thickness of the grid lines.
    pub line_thickness: f32,
    /// Current time, used to scroll the grid forward.
    pub time: f32,
    /// The speed at which the grid scrolls.
    pub scroll_speed: f32,
}

impl Default for SynthwaveConfig {
    fn default() -> Self {
        Self {
            grid_color: 0xFF_F20089,   // Hot Pink
            sun_color: 0xFF_FFD166,    // Cyber Yellow
            sky_color: 0xFF_01012B,    // Deep Space Blue
            ground_color: 0xFF_000000, // Black
            horizon: 0.5,
            line_thickness: 0.03, // In mapped coordinate space
            time: 0.0,
            scroll_speed: 5.0,
        }
    }
}

/// Applies a synthwave retro-grid effect to the framebuffer.
///
/// This draws an infinite perspective grid below the horizon, and a glowing
/// sun and sky gradient above the horizon. It completely overwrites the
/// existing framebuffer content.
pub fn apply_synthwave(fb: &mut Framebuffer, config: &SynthwaveConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let horizon_y = (config.horizon * height as f32) as usize;
    let time_offset = config.time * config.scroll_speed;

    // Center coordinates
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;

    // Sun parameters
    let sun_radius = height as f32 * 0.25;
    let sun_radius_sq = sun_radius * sun_radius;
    let sun_y = horizon_y as f32 - height as f32 * 0.05;

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let py = y as f32;

        if y < horizon_y {
            // SKY
            let sky_blend = 1.0 - (py / horizon_y as f32);
            let sky_c = blend_colors(config.sky_color, 0xFF_000000, sky_blend);

            for (x, pixel) in row.iter_mut().enumerate() {
                let px = x as f32;
                let dx = px - cx;
                let dy = py - sun_y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < sun_radius_sq {
                    // Sun
                    let mut is_sun = true;

                    // Classic retro sun horizontal cutouts
                    if dy > 0.0 {
                        let strip = (dy / (sun_radius * 0.1)).floor() as i32;
                        if strip % 2 == 1 {
                            is_sun = false;
                        }
                    }

                    if is_sun {
                        let sun_blend = dist_sq / sun_radius_sq; // Glow toward edges
                        *pixel = blend_colors(config.sun_color, 0xFF_FFFFFF, sun_blend * 0.5);
                    } else {
                        *pixel = sky_c;
                    }
                } else {
                    // Glow around sun
                    if dist_sq < sun_radius_sq * 1.5 {
                        let glow_factor = 1.0 - ((dist_sq - sun_radius_sq) / (sun_radius_sq * 0.5));
                        *pixel = blend_colors(config.sun_color, sky_c, 1.0 - (glow_factor * 0.3));
                    } else {
                        *pixel = sky_c;
                    }
                }
            }
        } else {
            // GROUND
            let dy = py - horizon_y as f32;
            let perspective_z = 1.0 / (dy.max(1.0) / height as f32);

            let v = perspective_z + time_offset;
            let grid_v = v.fract();

            // Far away fades to black
            let depth_fade = 1.0 - (dy / (height as f32 - horizon_y as f32)).clamp(0.0, 1.0);

            for (x, pixel) in row.iter_mut().enumerate() {
                let px = x as f32;
                let dx = px - cx;

                let u = dx * perspective_z * 0.05; // 0.05 controls grid horizontal spread
                let grid_u = u.fract().abs();

                // Check if we are on a grid line
                // The thickness should diminish slightly in perspective
                let adjusted_thickness = config.line_thickness * perspective_z.clamp(0.5, 2.0);

                let is_line = grid_u < adjusted_thickness
                    || grid_u > 1.0 - adjusted_thickness
                    || grid_v < adjusted_thickness
                    || grid_v > 1.0 - adjusted_thickness;

                if is_line {
                    *pixel = blend_colors(0xFF_000000, config.grid_color, depth_fade);
                } else {
                    *pixel = config.ground_color;
                }
            }
        }
    });
}

#[inline(always)]
fn blend_colors(c1: u32, c2: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let inv_t = 1.0 - t;

    let a1 = ((c1 >> 24) & 0xFF) as f32;
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = (c1 & 0xFF) as f32;

    let a2 = ((c2 >> 24) & 0xFF) as f32;
    let r2 = ((c2 >> 16) & 0xFF) as f32;
    let g2 = ((c2 >> 8) & 0xFF) as f32;
    let b2 = (c2 & 0xFF) as f32;

    let a = (a1 * inv_t + a2 * t) as u32;
    let r = (r1 * inv_t + r2 * t) as u32;
    let g = (g1 * inv_t + g2 * t) as u32;
    let b = (b1 * inv_t + b2 * t) as u32;

    (a << 24) | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthwave_config_default() {
        let config = SynthwaveConfig::default();
        assert_eq!(config.grid_color, 0xFF_F20089);
    }

    #[test]
    fn test_apply_synthwave() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_000000);

        let config = SynthwaveConfig::default();
        apply_synthwave(&mut fb, &config);

        // It should have drawn the sun and grid, so not all pixels are black anymore
        let mut has_non_black = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF_000000 {
                has_non_black = true;
                break;
            }
        }
        assert!(has_non_black, "Framebuffer should have been modified");
    }
}
