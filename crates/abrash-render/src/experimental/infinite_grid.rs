//! Infinite Grid Filter
//!
//! A procedural post-processing effect that draws an infinite perspective 3D grid
//! in a 2D pass, mapping screen (x, y) to world (x, z) using ray-plane intersection.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Infinite Grid filter.
#[derive(Debug, Clone, Copy)]
pub struct InfiniteGridConfig {
    /// Color of the grid lines.
    pub line_color: u32,
    /// Background color.
    pub background_color: u32,
    /// Height of the camera above the grid.
    pub camera_height: f32,
    /// Spacing between grid lines.
    pub grid_spacing: f32,
    /// Base thickness of the grid lines.
    pub line_thickness: f32,
    /// How fast the grid scrolls.
    pub speed: f32,
    /// Current time, used to animate the grid.
    pub time: f32,
    /// Maximum depth before the grid fades to the background color.
    pub horizon_distance: f32,
}

impl Default for InfiniteGridConfig {
    fn default() -> Self {
        Self {
            line_color: 0xFF_FF_00_FF,       // Neon Magenta
            background_color: 0xFF_00_00_00, // Black
            camera_height: 1.0,
            grid_spacing: 1.0,
            line_thickness: 0.05,
            speed: 2.0,
            time: 0.0,
            horizon_distance: 20.0,
        }
    }
}

/// Applies an infinite perspective 3D grid effect.
pub fn apply_infinite_grid(fb: &mut Framebuffer, config: &InfiniteGridConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;

    let fov_factor = 1.0; // Assume 90 degree FOV

    let c_line = Color::from_argb_u32(config.line_color);
    let c_bg = Color::from_argb_u32(config.background_color);

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    let inv_half_width = 1.0 / half_width;
    let inv_grid_spacing = 1.0 / config.grid_spacing;

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        // Map y to range [1.0, -1.0]
        let ndc_y = (half_height - y_f32) / half_height;

        // The grid is on the XZ plane at y = -camera_height.
        // We only draw the bottom half of the screen (ndc_y < 0.0)
        if ndc_y >= 0.0 {
            row.fill(config.background_color);
            return;
        }

        // Depth (Z) calculation based on ray-plane intersection
        // ray_dir.y = ndc_y. ray_dir.z = -fov_factor.
        // plane_y = -camera_height
        // t = plane_y / ray_dir.y = -camera_height / ndc_y
        let t = -config.camera_height / ndc_y;
        let z = t * fov_factor;

        if z > config.horizon_distance || z <= 0.0 {
            row.fill(config.background_color);
            return;
        }

        // Distance fade (1.0 at camera, 0.0 at horizon)
        let fade = 1.0 - (z / config.horizon_distance).clamp(0.0, 1.0);

        // Anti-aliasing/fading thickness: lines appear thicker further away in texture space
        // to maintain pixel width, or we can just use a fixed derivative
        let thickness = config.line_thickness * z;

        // Offset Z for animation
        let world_z = z - config.time * config.speed;
        let z_grid = (world_z / config.grid_spacing).fract().abs();

        // Z-line distance
        let dist_z = z_grid.min(1.0 - z_grid) * config.grid_spacing;

        let world_x_step = t * inv_half_width;
        let mut world_x = -half_width * world_x_step;

        for pixel in row.iter_mut() {
            let x_grid = (world_x * inv_grid_spacing).fract().abs();
            let dist_x = x_grid.min(1.0 - x_grid) * config.grid_spacing;

            // Minimum distance to any grid line
            let dist = dist_x.min(dist_z);

            if dist < thickness {
                // Anti-alias edge
                let alpha = 1.0 - (dist / thickness);
                let final_alpha = alpha * fade;

                *pixel = c_bg.lerp(c_line, final_alpha).to_argb_u32();
            } else {
                *pixel = config.background_color;
            }

            world_x += world_x_step;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infinite_grid_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = InfiniteGridConfig::default();
        apply_infinite_grid(&mut fb, &config);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_infinite_grid_horizon() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let config = InfiniteGridConfig::default();
        apply_infinite_grid(&mut fb, &config);

        // Top half should be background color
        assert_eq!(fb.get_pixel(5, 2), Some(config.background_color));
    }
}
