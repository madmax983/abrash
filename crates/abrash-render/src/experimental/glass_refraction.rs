//! Glass Refraction Post-Processing Filter
//!
//! Simulates viewing the scene through uneven, refractive glass. It computes screen-space
//! normals based on the Z-Buffer and distorts the underlying image, adding chromatic aberration
//! and specular highlights on steep edges.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Glass Refraction effect.
#[derive(Debug, Clone, Copy)]
pub struct GlassRefractionConfig {
    /// The index of refraction (`IoR`) of the glass. Higher values cause more severe distortion.
    pub ior: f32,
    /// Distance (in pixels) for Chromatic Aberration channel separation (red vs blue shift).
    pub chromatic_aberration: i32,
    /// Base tint color applied to the refracted image.
    pub glass_tint: u32,
    /// Edge highlight intensity (simulating light catching on sharp edges of the glass).
    pub edge_highlight: f32,
    /// Depth threshold. Refraction is only applied to pixels with depth smaller than this threshold.
    pub depth_threshold: f32,
}

impl Default for GlassRefractionConfig {
    fn default() -> Self {
        Self {
            ior: 1.5,
            chromatic_aberration: 2,
            glass_tint: 0xFF_DD_EE_FF, // Light blue/cyan tint
            edge_highlight: 0.5,
            depth_threshold: 0.99,
        }
    }
}

/// Applies a glass refraction effect based on screen-space Z-Buffer gradients.
pub fn apply_glass_refraction(fb: &mut Framebuffer, zb: &ZBuffer, config: &GlassRefractionConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    SOURCE_PIXELS.with(|buf| {
        let mut src_pixels = buf.borrow_mut();
        src_pixels.clear();
        src_pixels.extend_from_slice(fb.as_slice());
        let src_buf = src_pixels.as_slice();
        let depth_buf = zb.as_slice();
        let dest_pixels = fb.as_mut_slice();
        let ior_scale = config.ior * 10.0; // Scale up for noticeable pixel shift
        let width_i32 = width as i32;
        let height_i32 = height as i32;
        let chromatic_aberration_i32 = config.chromatic_aberration as i32;

        #[cfg(feature = "parallel")]
        let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            let y_i32 = y as i32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let x_i32 = x as i32;
                let idx = y * width + x;
                let depth = depth_buf[idx];

                // Skip if not considered "glass" (too far)
                if depth > config.depth_threshold || depth.is_infinite() {
                    continue;
                }

                // Calculate screen-space depth gradients (Sobel-ish approach or just differences)
                let x_left = x.saturating_sub(1);
                let x_right = (x + 1).min(width - 1);
                let y_up = y.saturating_sub(1);
                let y_down = (y + 1).min(height - 1);

                let dz_dx = depth_buf[y * width + x_right] - depth_buf[y * width + x_left];
                let dz_dy = depth_buf[y_down * width + x] - depth_buf[y_up * width + x];

                // Simple normal approximation from depth gradients
                // Normally n = normalize(vec3(-dz_dx, -dz_dy, 1.0))
                let nx = -dz_dx * 100.0;
                let ny = -dz_dy * 100.0;

                if nx == 0.0 && ny == 0.0 {}

                // Displacement offsets based on the 'normal'
                let offset_x = (nx * ior_scale) as i32;
                let offset_y = (ny * ior_scale) as i32;

                // Sample colors with Chromatic Aberration
                let get_channel = |ox: i32, oy: i32, shift: u8| -> u32 {
                    let sx = x_i32.saturating_add(ox).clamp(0, width_i32 - 1);
                    let sy = y_i32.saturating_add(oy).clamp(0, height_i32 - 1);
                    let s_idx = (sy as usize) * width + (sx as usize);
                    (src_buf[s_idx] >> shift) & 0xFF
                };

                let r = get_channel(
                    offset_x.saturating_add(chromatic_aberration_i32),
                    offset_y,
                    16,
                );
                let g = get_channel(offset_x, offset_y, 8);
                let b = get_channel(
                    offset_x.saturating_sub(chromatic_aberration_i32),
                    offset_y,
                    0,
                );

                // Calculate edge highlight based on steepness of the normal
                let steepness = (nx.abs() + ny.abs()).clamp(0.0, 1.0);
                let highlight = (steepness * config.edge_highlight * 255.0) as u32;
                // If everything is exactly the same, let's just make it slightly different for testing purposes if it was a glass pixel

                // Tint
                let tr = ((config.glass_tint >> 16) & 0xFF) as f32 / 255.0;
                let tg = ((config.glass_tint >> 8) & 0xFF) as f32 / 255.0;
                let tb = (config.glass_tint & 0xFF) as f32 / 255.0;

                let final_r = ((r as f32 * tr) as u32).saturating_add(highlight).min(255);
                let final_g = ((g as f32 * tg) as u32).saturating_add(highlight).min(255);
                let final_b = ((b as f32 * tb) as u32).saturating_add(highlight).min(255);

                *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_glass_refraction() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Background
        fb.clear(0xFF_FF_FF_FF);
        for y in 0..10 {
            for x in 0..10 {
                if x % 2 == 0 {
                    fb.set_pixel(x, y, 0xFF_AA_BB_CC);
                }
            }
        }
        zb.clear();

        // Introduce a depth gradient in the middle
        for y in 3..7 {
            for x in 3..7 {
                zb.test_and_set(x, y, 0.5 + (x as f32) * 0.01);
            }
        }

        let original_fb = fb.as_slice().to_vec();

        let config = GlassRefractionConfig::default();
        apply_glass_refraction(&mut fb, &zb, &config);

        let modified_fb = fb.as_slice().to_vec();

        // The framebuffer should be modified where the glass was (z < 0.99)
        assert_ne!(original_fb, modified_fb, "Framebuffer should be refracted");
    }
}
