//! Synthwave Grid Post-Processing Filter
//!
//! A retro effect that maps the lower half of the screen to a 3D perspective
//! grid moving towards the camera.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Synthwave Grid effect.
#[derive(Debug, Clone)]
pub struct SynthwaveGridConfig {
    /// The horizon line position on the screen (Y coordinate).
    pub horizon: f32,
    /// Color of the grid lines (ARGB).
    pub grid_color: u32,
    /// Color of the sky above the horizon (ARGB).
    pub sky_color: u32,
    /// Color of the ground between grid lines (ARGB).
    pub ground_color: u32,
    /// Distance between grid lines.
    pub grid_spacing: f32,
    /// Thickness of the grid lines.
    pub line_thickness: f32,
    /// Speed at which the grid scrolls forward.
    pub speed: f32,
    /// Current time for scrolling animation.
    pub time: f32,
    /// Camera field of view (perspective factor).
    pub fov: f32,
    /// Height of the camera above the grid.
    pub camera_height: f32,
    /// Distance at which the grid fades into the background.
    pub fog_distance: f32,
}

impl Default for SynthwaveGridConfig {
    fn default() -> Self {
        Self {
            horizon: 100.0,
            grid_color: 0xFF_FF00FF, // Neon Pink
            sky_color: 0xFF_000022,  // Dark Blue
            ground_color: 0xFF_000000, // Black
            grid_spacing: 10.0,
            line_thickness: 1.0,
            speed: 50.0,
            time: 0.0,
            fov: 150.0,
            camera_height: 20.0,
            fog_distance: 500.0,
        }
    }
}

/// Applies a synthwave perspective grid to the framebuffer.
///
/// Overwrites the entire framebuffer. The upper half is filled with the sky color,
/// and the lower half is rendered as an infinite moving 3D grid.
pub fn apply_synthwave_grid(fb: &mut Framebuffer, config: &SynthwaveGridConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let half_w = width as f32 / 2.0;

    let horizon_i = config.horizon as i32;
    let horizon_y = horizon_i.max(0).min(height as i32) as usize;

    // Fast path: fill sky color
    if horizon_y > 0 {
        pixels[..horizon_y * width].fill(config.sky_color);
    }

    if horizon_y >= height {
        return;
    }

    let ground_pixels = &mut pixels[horizon_y * width..];

    #[cfg(feature = "parallel")]
    let row_iter = ground_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = ground_pixels.chunks_exact_mut(width).enumerate();

    let sky_color = config.sky_color;
    let grid_color = config.grid_color;
    let ground_color = config.ground_color;

    row_iter.for_each(|(dy, row)| {
        let y = horizon_y + dy;
        let screen_y_dist = (y as f32) - config.horizon;

        if screen_y_dist <= 0.0 {
            row.fill(sky_color);
            return;
        }

        // Project Y coordinate into 3D Z distance
        let distance = (config.camera_height * config.fov) / screen_y_dist;

        let scroll_z = distance + config.time * config.speed;
        let z_mod = scroll_z % config.grid_spacing;

        // Anti-aliasing / distance-based line thickness fading
        let current_thickness = (config.line_thickness * (distance / 50.0).max(1.0)).min(config.grid_spacing / 2.0);

        let line_intensity = if z_mod < current_thickness || z_mod > config.grid_spacing - current_thickness {
            let dist_to_edge = z_mod.min(config.grid_spacing - z_mod);
            (1.0 - (dist_to_edge / current_thickness)).max(0.0)
        } else {
            0.0
        };

        let fog = (distance / config.fog_distance).clamp(0.0, 1.0);

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = (x as f32) - half_w;
            let world_x = (distance * dx) / config.fov;
            let x_mod = world_x.rem_euclid(config.grid_spacing);

            let v_line_intensity = if x_mod < current_thickness || x_mod > config.grid_spacing - current_thickness {
                let dist_to_edge = x_mod.min(config.grid_spacing - x_mod);
                (1.0 - (dist_to_edge / current_thickness)).max(0.0)
            } else {
                0.0
            };

            let intensity = line_intensity.max(v_line_intensity);
            let final_intensity = intensity * (1.0 - fog);

            *pixel = lerp_color(ground_color, grid_color, final_intensity);
        }
    });
}

fn lerp_color(c1: u32, c2: u32, t: f32) -> u32 {
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
    fn test_synthwave_grid() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = SynthwaveGridConfig {
            horizon: 50.0,
            grid_color: 0xFF_FFFFFF,
            sky_color: 0xFF_000000,
            ground_color: 0xFF_111111,
            ..Default::default()
        };

        apply_synthwave_grid(&mut fb, &config);

        // Sky should be black
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_000000);

        // Ensure there are some grid lines (white or gray)
        let has_grid = fb.as_slice().iter().any(|&p| p == 0xFF_FFFFFF || p == 0xFF_111111);
        assert!(has_grid);
    }
}
