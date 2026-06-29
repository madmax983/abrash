//! Infinite Tunnel Post-Processing Filter
//!
//! Applies a perspective 3D tunnel effect by mapping screen coordinates
//! to polar coordinates to fetch texture data.

#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

/// Applies an infinite 3D tunnel effect.
///
/// # Arguments
///
/// * `framebuffer` - The framebuffer to modify in-place.
/// * `time` - The current animation time.
/// * `texture` - The texture mapped to the walls of the tunnel.
///
/// Maps Cartesian screen coordinates to polar coordinates (angle and distance),
/// transforming them into texture coordinates to create a perspective tunnel.
///
/// ⚡ Bolt Performance Optimization:
/// Directly mutating the framebuffer slice instead of collecting to an intermediate
/// `Vec` array entirely elides dynamic heap allocations (saving W*H bytes per frame)
/// and removes the O(N) secondary pass previously required to copy pixels back to the screen.
pub fn apply_tunnel(framebuffer: &mut Framebuffer, time: f32, texture: &Texture) {
    let width = framebuffer.width() as usize;
    let height = framebuffer.height() as usize;

    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;

    framebuffer
        .as_mut_slice()
        .chunks_exact_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            let dy = y as f32 - center_y;

            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - center_x;

                // Distance from center squared
                let dist_sq = dx.mul_add(dx, dy * dy).max(1.0);

                // Fast Inverse Square Root for 1.0 / distance
                let mut inv_dist = f32::from_bits(0x5f37_59df - (dist_sq.to_bits() >> 1));
                inv_dist = inv_dist * (1.5 - (0.5 * dist_sq * inv_dist * inv_dist)); // 1st iteration

                // Angle
                let angle = f32::atan2(dy, dx);

                // Map to U, V
                // U maps around the cylinder (angle)
                let u = (angle / std::f32::consts::PI + 1.0) * 0.5;

                // V maps down the depth of the tunnel
                // We scale by some factor to make the texture repeat nicely
                let depth_scale = (width.min(height) as f32) * 0.5;
                let v = depth_scale * inv_dist + time;

                // Calculate a simple shading based on distance (darker further away)
                let shade = (1.0 / (inv_dist * depth_scale)).clamp(0.0, 1.0);

                let u_norm = u.fract();
                let v_norm = v.fract();

                let u_val = if u_norm < 0.0 { u_norm + 1.0 } else { u_norm };
                let v_val = if v_norm < 0.0 { v_norm + 1.0 } else { v_norm };

                // texture.get_pixel takes normalized floats in [0.0, 1.0] and returns a single u32 color
                let texel = texture.get_pixel(u_val, v_val);

                // Apply shading using SWAR multiplication for speed
                let shade_fix = (shade * 256.0) as u64;

                // Pack R and B into one u64, G into another
                let rb = ((u64::from(texel)) & 0x00FF_00FF) * shade_fix;
                let g  = (((u64::from(texel)) >> 8) & 0x0000_00FF) * shade_fix;

                // Shift down and mask
                let rb_out = (rb >> 8) & 0x00FF_00FF;
                let g_out  = (g >> 8) & 0x0000_00FF;

                let a = texel & 0xFF00_0000;
                *pixel = a | (rb_out as u32) | ((g_out as u32) << 8);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_tunnel() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let tex = Texture::new(64, 64).unwrap();

        apply_tunnel(&mut fb, 0.0, &tex);
    }
}
