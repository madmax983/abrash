//! Mode 7 Pseudo-3D Floor Rendering
//!
//! Simulates the classic SNES Mode 7 affine transformation technique
//! to render a 2D texture as a 3D perspective floor.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Mode 7 projection.
#[derive(Debug, Clone)]
pub struct Mode7Config {
    /// Camera X position
    pub cx: f32,
    /// Camera Y position (height above the floor)
    pub cy: f32,
    /// Camera Z position
    pub cz: f32,
    /// Camera rotation angle (yaw) in radians
    pub angle: f32,
    /// Field of view (affects perspective scaling)
    pub fov: f32,
    /// Horizon line position on the screen (Y coordinate)
    pub horizon: f32,
    /// Scaling factor for the texture mapping
    pub scale: f32,
    /// Background color for distant fog
    pub fog_color: u32,
    /// Distance where fog starts
    pub fog_start: f32,
    /// Distance where fog ends (fully opaque)
    pub fog_end: f32,
}

impl Default for Mode7Config {
    fn default() -> Self {
        Self {
            cx: 0.0,
            cy: 50.0,
            cz: 0.0,
            angle: 0.0,
            fov: 256.0,
            horizon: 100.0,
            scale: 1.0,
            fog_color: 0xFF00_0000,
            fog_start: 200.0,
            fog_end: 800.0,
        }
    }
}

/// Helper function to interpolate between two ARGB colors.
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

/// Renders a Mode 7 pseudo-3D floor to the framebuffer.
///
/// This technique projects a flat 2D texture into 3D space by calculating
/// the intersection of a ray from the camera through each screen pixel
/// onto a mathematical ground plane.
///
/// * `fb`: The destination framebuffer.
/// * `texture`: The source texture to project.
/// * `config`: The camera and projection parameters.
pub fn render_mode7(fb: &mut Framebuffer, texture: &Texture, config: &Mode7Config) {
    let w = fb.width() as usize;
    let h = fb.height() as usize;

    let cos_angle = config.angle.cos();
    let sin_angle = config.angle.sin();

    let horizon_i = config.horizon as i32;
    let start_y = horizon_i.max(0) as usize;

    // We only draw below the horizon
    if start_y >= h {
        return;
    }

    let tex_w = texture.width() as f32;
    let tex_h = texture.height() as f32;
    let tex_w_i = texture.width() as usize;
    let tex_h_i = texture.height() as usize;

    let tex_w_mask = if tex_w_i.is_power_of_two() {
        tex_w_i - 1
    } else {
        0
    };
    let tex_h_mask = if tex_h_i.is_power_of_two() {
        tex_h_i - 1
    } else {
        0
    };

    // Check if texture is empty
    if tex_w_i == 0 || tex_h_i == 0 {
        return;
    }

    let tex_data = texture.pixels();
    let half_w = w as f32 / 2.0;

    let row_count = h - start_y;

    // To allow parallel processing row-by-row
    let buffer = fb.as_mut_slice();
    let target_slice = &mut buffer[start_y * w..start_y * w + row_count * w];

    #[cfg(feature = "parallel")]
    let row_iter = target_slice.par_chunks_exact_mut(w).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = target_slice.chunks_exact_mut(w).enumerate();

    row_iter.for_each(|(dy, row)| {
        let y = start_y + dy;
        let screen_y_dist = (y as f32) - config.horizon;

        // Prevent division by zero at the exact horizon
        if screen_y_dist <= 0.0 {
            return;
        }

        // Depth from camera based on field of view and height
        let distance = (config.cy * config.fov) / screen_y_dist;

        // Optional: crude distance fog to hide aliasing at the horizon
        let fog_factor =
            ((distance - config.fog_start) / (config.fog_end - config.fog_start)).clamp(0.0, 1.0);

        // Precalculate horizontal step size in space coordinates
        let step_space_x = distance / config.fov;

        // Rotate step sizes
        let step_rot_x = step_space_x * cos_angle;
        let step_rot_z = step_space_x * sin_angle;

        // Apply scale to step sizes
        let d_map_x = step_rot_x * config.scale;
        let d_map_z = step_rot_z * config.scale;

        // Initial space coordinates for x = 0
        let initial_screen_x_dist = 0.0 - half_w;
        let initial_space_x = (distance * initial_screen_x_dist) / config.fov;
        let space_z = distance;

        // Initial rotated coordinates
        let initial_rot_x = initial_space_x * cos_angle - space_z * sin_angle;
        let initial_rot_z = initial_space_x * sin_angle + space_z * cos_angle;

        // Initial map coordinates
        let mut map_x = (initial_rot_x + config.cx) * config.scale;
        let mut map_z = (initial_rot_z + config.cz) * config.scale;

        if tex_w_mask != 0 && tex_h_mask != 0 {
            if fog_factor > 0.0 {
                for pixel in row.iter_mut() {
                    let u = map_x.floor() as i32;
                    let v = map_z.floor() as i32;

                    let tx = (u as usize) & tex_w_mask;
                    let ty = (v as usize) & tex_h_mask;

                    let color = tex_data[ty * tex_w_i + tx];

                    *pixel = lerp_color(color, config.fog_color, fog_factor);

                    // Step mapping coordinates for the next pixel
                    map_x += d_map_x;
                    map_z += d_map_z;
                }
            } else {
                for pixel in row.iter_mut() {
                    let u = map_x.floor() as i32;
                    let v = map_z.floor() as i32;

                    let tx = (u as usize) & tex_w_mask;
                    let ty = (v as usize) & tex_h_mask;

                    *pixel = tex_data[ty * tex_w_i + tx];

                    // Step mapping coordinates for the next pixel
                    map_x += d_map_x;
                    map_z += d_map_z;
                }
            }
        } else {
            for pixel in row.iter_mut() {
                // Wrap texture coordinates using euclidean remainder
                let u = map_x.floor() as i32;
                let u = u.rem_euclid(tex_w_i as i32) as usize;
                let v = map_z.floor() as i32;
                let v = v.rem_euclid(tex_h_i as i32) as usize;

                let tx = u % tex_w_i;
                let ty = v % tex_h_i;

                let color = tex_data[ty * tex_w_i + tx];

                *pixel = if fog_factor > 0.0 {
                    lerp_color(color, config.fog_color, fog_factor) // Fade to fog color
                } else {
                    color
                };

                // Step mapping coordinates for the next pixel
                map_x += d_map_x;
                map_z += d_map_z;
            }
        }
    });
}
