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
pub fn apply_tunnel(framebuffer: &mut Framebuffer, time: f32, texture: &Texture) {
    let width = framebuffer.width() as usize;
    let height = framebuffer.height() as usize;

    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;

    let mut new_pixels = vec![0; width * height];

    new_pixels
        .chunks_exact_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            let dy = y as f32 - center_y;

            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - center_x;

                // Distance from center
                #[allow(clippy::imprecise_flops)]
                let distance = (dx * dx + dy * dy).sqrt().max(1.0);

                // Angle
                let angle = f32::atan2(dy, dx);

                // Map to U, V
                // U maps around the cylinder (angle)
                let u = (angle / std::f32::consts::PI + 1.0) * 0.5;

                // V maps down the depth of the tunnel
                // We scale by some factor to make the texture repeat nicely
                let depth_scale = (width.min(height) as f32) * 0.5;
                let v = depth_scale / distance + time;

                // Calculate a simple shading based on distance (darker further away)
                let shade = (distance / depth_scale).clamp(0.0, 1.0);

                let u_norm = u.fract();
                let v_norm = v.fract();

                let u_val = if u_norm < 0.0 { u_norm + 1.0 } else { u_norm };
                let v_val = if v_norm < 0.0 { v_norm + 1.0 } else { v_norm };

                // texture.get_pixel takes normalized floats in [0.0, 1.0] and returns a single u32 color
                let texel = texture.get_pixel(u_val, v_val);

                // Apply shading
                let a = (texel >> 24) & 0xFF;
                let r = ((texel >> 16) & 0xFF) as f32 * shade;
                let g = ((texel >> 8) & 0xFF) as f32 * shade;
                let b = (texel & 0xFF) as f32 * shade;

                *pixel = (a << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            }
        });

    for y in 0..framebuffer.height() {
        for x in 0..framebuffer.width() {
            let idx = (y as usize) * width + (x as usize);
            framebuffer.set_pixel(x as i32, y as i32, new_pixels[idx]);
        }
    }
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
