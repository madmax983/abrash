//! Rotozoom effect
//!
//! Applies a rotozoom effect to a texture, writing the result into a framebuffer.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies rotozoom.
#[cfg(feature = "nova")]
pub fn render_rotozoom(
    framebuffer: &mut Framebuffer,
    texture: &Texture,
    angle: f32,
    scale: f32,
    center_x: f32,
    center_y: f32,
) {
    let width = framebuffer.width();
    let height = framebuffer.height();
    let tex_width = texture.width();
    let tex_height = texture.height();

    // Ensure we don't divide by zero
    let scale = if scale.abs() < f32::EPSILON {
        f32::EPSILON
    } else {
        scale
    };

    let sin_a = angle.sin();
    let cos_a = angle.cos();

    let half_w = width as f32 * 0.5;
    let half_h = height as f32 * 0.5;

    let tex_w_mask = tex_width.saturating_sub(1);
    let tex_h_mask = tex_height.saturating_sub(1);

    // Using powers of 2 textures is highly recommended for `& mask` optimization
    let is_pow2 = tex_width.is_power_of_two() && tex_height.is_power_of_two();

    let fb_buffer = framebuffer.as_mut_slice();

    // Constant deltas across a row
    let du_dx = cos_a / scale;
    let dv_dx = sin_a / scale;

    let process_row = |(y, row): (usize, &mut [u32])| {
        let dy = y as f32 - half_h;

        // Start coordinates for the left edge of the row (dx = -half_w)
        let mut u = (-half_w * cos_a - dy * sin_a) / scale + center_x;
        let mut v = (-half_w * sin_a + dy * cos_a) / scale + center_y;

        for pixel in row.iter_mut() {
            let tu = u as i32;
            let tv = v as i32;

            let tex_x = if is_pow2 {
                (tu as u32) & tex_w_mask
            } else {
                tu.rem_euclid(tex_width as i32) as u32
            };

            let tex_y = if is_pow2 {
                (tv as u32) & tex_h_mask
            } else {
                tv.rem_euclid(tex_height as i32) as u32
            };

            let color = texture.get_pixel_texel(tex_x as i32, tex_y as i32);
            *pixel = color;

            u += du_dx;
            v += dv_dx;
        }
    };

    #[cfg(feature = "parallel")]
    fb_buffer
        .chunks_exact_mut(width as usize)
        .enumerate()
        .par_bridge()
        .for_each(process_row);

    #[cfg(not(feature = "parallel"))]
    fb_buffer
        .chunks_exact_mut(width as usize)
        .enumerate()
        .for_each(process_row);
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_rotozoom_basic() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let tex = Texture::new(64, 64).unwrap();
        render_rotozoom(&mut fb, &tex, 0.0, 1.0, 32.0, 32.0);
        // The fact that it doesn't panic means it ran correctly.
    }
}
