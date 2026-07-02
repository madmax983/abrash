//! Crepuscular Rays (God Rays) Post-Processing Effect
//!
//! Simulates volumetric light scattering by performing a radial blur originating
//! from a specific screen-space light position.
//!
//! # Features
//! * Radial sampling accumulation for glowing light shafts.
//! * Customizable density, weight, decay, and exposure.

use crate::framebuffer::Framebuffer;

/// Applies a crepuscular ray (God Rays) effect to the framebuffer.
///
/// This simulates volumetric light scattering from a bright source.
/// It works best when applied after rendering high-contrast elements (e.g., a bright sun)
/// and occluders (e.g., dark trees or buildings in front of the sun).
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `light_x` - Screen-space X coordinate of the light source (can be outside the screen).
/// * `light_y` - Screen-space Y coordinate of the light source.
/// * `density` - Distance between samples. Usually around 1.0.
/// * `weight` - Intensity of each sample. Usually around 0.01 - 0.1.
/// * `decay` - Falloff factor per sample (0.0 to 1.0). High values (>0.9) mean longer rays.
/// * `exposure` - Final brightness multiplier for the accumulated light.
/// * `num_samples` - Number of samples to take along the ray. Higher is smoother but slower (e.g., 32-100).
///
/// Configuration parameters for the God Rays (Crepuscular Rays) effect.
#[derive(Debug, Clone, Copy)]
pub struct GodRaysConfig {
    /// Screen-space X coordinate of the light source (can be outside the screen).
    pub light_x: f32,
    /// Screen-space Y coordinate of the light source.
    pub light_y: f32,
    /// Distance between samples. Usually around 1.0.
    pub density: f32,
    /// Intensity of each sample. Usually around 0.01 - 0.1.
    pub weight: f32,
    /// Falloff factor per sample (0.0 to 1.0). High values (>0.9) mean longer rays.
    pub decay: f32,
    /// Final brightness multiplier for the accumulated light.
    pub exposure: f32,
    /// Number of samples to take along the ray. Higher is smoother but slower (e.g., 32-100).
    pub num_samples: u32,
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_god_rays(fb: &mut Framebuffer, config: &GodRaysConfig) {
    if config.num_samples == 0 || config.weight <= 0.0 || config.exposure <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    SOURCE_PIXELS.with(|buf| {
        let mut original_pixels = buf.borrow_mut();
        original_pixels.clear();
        original_pixels.extend_from_slice(fb.as_slice());

        let original_pixels_slice = original_pixels.as_slice();
        let pixels = fb.as_mut_slice();

        let density_step = config.density / config.num_samples as f32;

        // ⚡ Bolt: Eliminate per-frame heap allocation for sample weights by using a thread-local buffer.
        thread_local! {
            static WEIGHTS_BUFFER: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
        }

        let mut weights_fixed = WEIGHTS_BUFFER.with(std::cell::RefCell::take);
        weights_fixed.clear();
        let mut decay = 1.0;
        for _ in 0..config.num_samples {
            weights_fixed.push((decay * config.weight * 65536.0) as u32);
            decay *= config.decay;
        }
        let weights_slice = weights_fixed.as_slice();

        #[cfg(feature = "parallel")]
        let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = pixels.chunks_exact_mut(width).enumerate();

        let width_i32 = width as i32;
        let height_i32 = height as i32;

        row_iter.for_each(|(y, row)| {
            let base_idx_start = y * width;
            let dy = (y as f32 - config.light_y) * density_step;
            let step_y = (dy * 65536.0) as i32;
            let start_y = ((y as i32) << 16) + 32768; // +0.5 for rounding

            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = (x as f32 - config.light_x) * density_step;
                let step_x = (dx * 65536.0) as i32;

                let mut current_x = ((x as i32) << 16) + 32768;
                let mut current_y = start_y;

                let base_color = original_pixels_slice[base_idx_start + x];

                // 16.16 fixed-point accumulation
                let mut accum_r = ((base_color >> 16) & 0xFF) * 65536;
                let mut accum_g = ((base_color >> 8) & 0xFF) * 65536;
                let mut accum_b = (base_color & 0xFF) * 65536;

                for &weight in weights_slice {
                    current_x -= step_x;
                    current_y -= step_y;

                    let px = current_x >> 16;
                    let py = current_y >> 16;

                    // Casting to u32 handles negative values correctly
                    // as they wrap around to a huge positive number.
                    if (px as u32) < (width_i32 as u32) && (py as u32) < (height_i32 as u32) {
                        let sample_color = original_pixels_slice[py as usize * width + px as usize];

                        let sample_r = (sample_color >> 16) & 0xFF;
                        let sample_g = (sample_color >> 8) & 0xFF;
                        let sample_b = sample_color & 0xFF;

                        accum_r += sample_r * weight;
                        accum_g += sample_g * weight;
                        accum_b += sample_b * weight;
                    }
                }

                // Convert back from fixed point to floating point exposure
                let final_r =
                    (((accum_r as f32) / 65536.0) * config.exposure).clamp(0.0, 255.0) as u32;
                let final_g =
                    (((accum_g as f32) / 65536.0) * config.exposure).clamp(0.0, 255.0) as u32;
                let final_b =
                    (((accum_b as f32) / 65536.0) * config.exposure).clamp(0.0, 255.0) as u32;

                *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
            }
        });

        WEIGHTS_BUFFER.with(|buf| {
            buf.replace(weights_fixed);
        });
    });
}
