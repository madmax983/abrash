//! Volumetric Light Shafts (God Rays).
//!
//! Simulates light scattering by performing a radial blur from a light source position.
//!
//! # Algorithm
//! 1. Create a downsampled mask of bright pixels (luminance threshold).
//! 2. For each pixel on screen, cast a ray towards the light source.
//! 3. Sample the mask along the ray and accumulate light.
//! 4. Additively blend the result with the original image.

use crate::framebuffer::Framebuffer;
use crate::math::Vec2;
use crate::utils::pixel_luminance;
use std::cell::RefCell;

thread_local! {
    static GOD_RAY_BUFFERS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies volumetric light shafts (God Rays) to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `light_screen_pos` - Screen space position of the light source (in pixels).
/// * `density` - Controls the spread of the rays (e.g., 1.0). Higher = longer rays.
/// * `weight` - Intensity of each sample (e.g., 0.01).
/// * `decay` - Light decay factor per sample (e.g., 1.0). Controls falloff.
/// * `exposure` - Final intensity multiplier (e.g., 1.0).
/// * `samples` - Number of samples along the ray (e.g., 100).
pub fn apply_god_rays(
    fb: &mut Framebuffer,
    light_screen_pos: Vec2,
    density: f32,
    weight: f32,
    decay: f32,
    exposure: f32,
    samples: usize,
) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Use a downsampled buffer (half width/height) for performance and soft look.
    let mask_width = width / 2;
    let mask_height = height / 2;

    if mask_width == 0 || mask_height == 0 {
        return;
    }

    GOD_RAY_BUFFERS.with(|buffer_ref| {
        let mut buffer = buffer_ref.borrow_mut();
        let needed_size = mask_width * mask_height;

        if buffer.len() < needed_size {
            buffer.resize(needed_size, 0);
        }

        let mask = &mut buffer[..needed_size];

        // 1. Generation Pass: Downsample & Threshold
        // We only keep bright pixels to cast rays.
        let threshold = 200;

        for y in 0..mask_height {
            for x in 0..mask_width {
                // Sample from original image (nearest neighbor: 2x)
                let src_x = (x * 2).min(width - 1);
                let src_y = (y * 2).min(height - 1);
                let src_idx = src_y * width + src_x;
                let pixel = pixels[src_idx];

                let lum = pixel_luminance(pixel);
                if lum > threshold {
                    mask[y * mask_width + x] = pixel;
                } else {
                    mask[y * mask_width + x] = 0xFF000000; // Occluded
                }
            }
        }

        // 2. Radial Blur Pass
        // Iterate over full resolution pixels.
        // We accumulate in-place? No, we need read access to mask and write to pixels.
        // Since mask is separate, we can write directly to pixels.

        // Precompute density / samples
        let delta_scale = density / samples as f32;

        for y in 0..height {
            for x in 0..width {
                let tex_coord = Vec2::new(x as f32, y as f32);

                // Vector pointing FROM pixel TO light (scaled)
                // We subtract this vector iteratively to move towards light.
                // Wait: P' = P - (P - L) * scale
                // If scale is positive, we move towards L.
                // delta = (tex_coord - light_screen_pos) * delta_scale
                // new_pos = current_pos - delta
                // = current_pos - (tex_coord - light_screen_pos) * scale
                // = current_pos + (light_screen_pos - tex_coord) * scale
                // Yes, this moves towards light.
                let delta = (tex_coord - light_screen_pos) * delta_scale;

                let mut current_pos = tex_coord;
                let mut illumination_decay = 1.0;

                let mut r_acc = 0.0;
                let mut g_acc = 0.0;
                let mut b_acc = 0.0;

                for _ in 0..samples {
                    current_pos = current_pos - delta;

                    // Sample from mask (nearest neighbor)
                    // Map current_pos (full res) to mask coords (half res)
                    let mask_x = (current_pos.x * 0.5) as i32;
                    let mask_y = (current_pos.y * 0.5) as i32;

                    if mask_x >= 0
                        && mask_x < mask_width as i32
                        && mask_y >= 0
                        && mask_y < mask_height as i32
                    {
                        let idx = mask_y as usize * mask_width + mask_x as usize;
                        // SAFETY: Bounds checked above
                        let sample = unsafe { *mask.get_unchecked(idx) };

                        // If black (or transparent), it adds nothing.
                        // Check for non-black color.
                        if (sample & 0x00FFFFFF) != 0 {
                            let r = ((sample >> 16) & 0xFF) as f32;
                            let g = ((sample >> 8) & 0xFF) as f32;
                            let b = (sample & 0xFF) as f32;

                            let factor = illumination_decay * weight;
                            r_acc += r * factor;
                            g_acc += g * factor;
                            b_acc += b * factor;
                        }
                    }

                    illumination_decay *= decay;
                }

                // Composite
                if r_acc > 0.0 || g_acc > 0.0 || b_acc > 0.0 {
                    let pixel_idx = y * width + x;
                    // SAFETY: y < height, x < width
                    let original = unsafe { *pixels.get_unchecked(pixel_idx) };
                    let r_orig = ((original >> 16) & 0xFF) as f32;
                    let g_orig = ((original >> 8) & 0xFF) as f32;
                    let b_orig = (original & 0xFF) as f32;

                    // Additive blend with exposure
                    let r_final = (r_orig + r_acc * exposure).min(255.0) as u32;
                    let g_final = (g_orig + g_acc * exposure).min(255.0) as u32;
                    let b_final = (b_orig + b_acc * exposure).min(255.0) as u32;

                    unsafe {
                        *pixels.get_unchecked_mut(pixel_idx) =
                            (original & 0xFF000000) | (r_final << 16) | (g_final << 8) | b_final;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_god_rays_basic() {
        let width = 20;
        let height = 20;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // 1. Draw a bright light source in center
        fb.set_pixel(10, 10, 0xFFFFFFFF); // White

        // 2. Apply God Rays
        // Light pos at (10, 10)
        apply_god_rays(
            &mut fb,
            Vec2::new(10.0, 10.0),
            1.0, // Density
            0.5, // Weight
            0.9, // Decay
            1.0, // Exposure
            5,   // Samples
        );

        // 3. Check a pixel near the light source (should be brightened)
        // (11, 11) should have sampled (10, 10)
        let p = fb.get_pixel(11, 11).unwrap();
        // Luminance should be > 0 (it started black)
        assert!(
            pixel_luminance(p) > 0,
            "Pixel (11, 11) should be lit by god rays"
        );
    }
}
