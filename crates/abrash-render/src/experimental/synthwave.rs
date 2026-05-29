//! Retro Synthwave Backdrop Generator
//!
//! Generates a stylized "Outrun" synthwave background consisting of a gradient sky,
//! a glowing sun with horizontal cutouts, and a perspective wireframe grid.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Synthwave Backdrop.
#[derive(Debug, Clone, Copy)]
pub struct SynthwaveConfig {
    /// Time for animating the grid.
    pub time: f32,
    /// Horizon line Y position (0.0 to 1.0, e.g. 0.5 is middle).
    pub horizon: f32,
    /// Sky gradient top color (ARGB).
    pub sky_top: u32,
    /// Sky gradient bottom color (ARGB).
    pub sky_bottom: u32,
    /// Sun color (ARGB).
    pub sun_color: u32,
    /// Sun radius (0.0 to 1.0 relative to screen height).
    pub sun_radius: f32,
    /// Grid line color (ARGB).
    pub grid_color: u32,
    /// Draw over existing background only (where depth == `f32::INFINITY`).
    pub background_only: bool,
}

impl Default for SynthwaveConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            horizon: 0.5,
            sky_top: 0xFF_0B0033,    // Dark purple/blue
            sky_bottom: 0xFF_CC0066, // Neon pink
            sun_color: 0xFF_FFCC00,  // Cyber yellow
            sun_radius: 0.25,
            grid_color: 0xFF_00FFFF, // Cyan
            background_only: false,
        }
    }
}

fn lerp_color(c1: u32, c2: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = (c1 & 0xFF) as f32;

    let r2 = ((c2 >> 16) & 0xFF) as f32;
    let g2 = ((c2 >> 8) & 0xFF) as f32;
    let b2 = (c2 & 0xFF) as f32;

    let r = (r1 + (r2 - r1) * t) as u32;
    let g = (g1 + (g2 - g1) * t) as u32;
    let b = (b1 + (b2 - b1) * t) as u32;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Renders a Synthwave backdrop onto the framebuffer.
/// If `config.background_only` is true, only pixels where `zb` depth is ``f32::INFINITY`` are modified.
pub fn apply_synthwave(fb: &mut Framebuffer, zb: Option<&ZBuffer>, config: &SynthwaveConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let horizon_y = (config.horizon * height as f32) as usize;
    let center_x = width as f32 * 0.5;
    let sun_center_y = horizon_y as f32;
    let sun_radius_px = config.sun_radius * height as f32;
    let sun_radius_sq = sun_radius_px * sun_radius_px;

    let pixels = fb.as_mut_slice();
    let depths = zb.map(abrash_core::zbuffer::ZBuffer::as_slice);

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let is_sky = y < horizon_y;

        let row_depths = depths.map(|d| &d[y * width..(y + 1) * width]);

        for x in 0..width {
            if config.background_only {
                if let Some(r_depths) = row_depths {
                    if r_depths[x] < f32::INFINITY {
                        continue;
                    }
                }
            }

            let mut final_color = 0xFF_000000;

            if is_sky {
                let sky_t = y as f32 / horizon_y as f32;
                final_color = lerp_color(config.sky_top, config.sky_bottom, sky_t);

                let dx = x as f32 - center_x;
                let dy = y as f32 - sun_center_y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq <= sun_radius_sq {
                    let rel_y = dy / sun_radius_px;
                    let bottom_dist = 1.0 + rel_y;

                    let freq = 10.0;
                    let phase = bottom_dist * freq;
                    let sin_val = phase.sin();

                    let threshold = -0.5 + rel_y * 1.5;

                    if sin_val > threshold {
                        final_color = config.sun_color;
                    }
                }
            } else {
                let y_from_horizon = (y - horizon_y) as f32;
                if y_from_horizon > 0.0 {
                    let pz = 100.0 / y_from_horizon;
                    let px = (x as f32 - center_x) * pz / 100.0;

                    let grid_scale = 50.0;
                    let speed = 20.0;
                    let scroll = config.time * speed;

                    let u_grid = (px * grid_scale).fract();
                    let v_grid = ((pz * grid_scale) - scroll).fract();

                    let line_thickness = 0.05 + 0.05 * pz;

                    if u_grid.abs() < line_thickness
                        || (1.0 - u_grid.abs()) < line_thickness
                        || v_grid.abs() < line_thickness
                        || (1.0 - v_grid.abs()) < line_thickness
                    {
                        let fade =
                            (1.0 - (y_from_horizon / (height - horizon_y) as f32)).clamp(0.0, 1.0);
                        final_color = lerp_color(0xFF_000000, config.grid_color, fade);
                    }
                }
            }
            row[x] = final_color;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn test_apply_synthwave_basic() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = SynthwaveConfig::default();
        apply_synthwave(&mut fb, None, &config);

        // Sky pixel
        let top_pixel = fb.get_pixel(50, 10).unwrap();
        assert_ne!(top_pixel, 0xFF_000000);

        // Sun pixel
        let sun_pixel = fb.get_pixel(50, 48).unwrap();
        assert_ne!(sun_pixel, 0xFF_000000);
    }

    #[test]
    fn test_apply_synthwave_background_only() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();

        fb.clear(0xFF_FFFFFF);
        zb.clear();

        // Put a foreground object at (5, 5)
        zb.as_mut_slice()[55] = 10.0;

        let config = SynthwaveConfig {
            background_only: true,
            ..Default::default()
        };
        apply_synthwave(&mut fb, Some(&zb), &config);

        // Background pixel should change
        assert_ne!(fb.get_pixel(1, 1).unwrap(), 0xFF_FFFFFF);

        // Foreground pixel should not change
        assert_eq!(fb.get_pixel(5, 5).unwrap(), 0xFF_FFFFFF);
    }
}
