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

pub fn apply_god_rays(fb: &mut Framebuffer, config: &GodRaysConfig) {
    if config.num_samples == 0 || config.weight <= 0.0 || config.exposure <= 0.0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // We need a copy of the original pixels to sample from
    // while we write accumulated values to the framebuffer.
    let original_pixels = fb.as_slice().to_vec();
    let pixels = fb.as_mut_slice();

    let inv_width = 1.0 / width as f32;
    let inv_height = 1.0 / height as f32;

    // Normalized light position (0.0 to 1.0)
    let light_u = config.light_x * inv_width;
    let light_v = config.light_y * inv_height;

    for y in 0..height {
        let v = y as f32 * inv_height;
        for x in 0..width {
            let u = x as f32 * inv_width;

            // Calculate vector from pixel to light source
            let mut delta_u = u - light_u;
            let mut delta_v = v - light_v;

            // Scale delta by density
            delta_u *= config.density / config.num_samples as f32;
            delta_v *= config.density / config.num_samples as f32;

            let mut current_u = u;
            let mut current_v = v;

            let mut illumination_decay = 1.0;

            // Get base color of current pixel
            let base_idx = (y * width + x) as usize;
            let base_color = original_pixels[base_idx];
            let base_r = ((base_color >> 16) & 0xFF) as f32;
            let base_g = ((base_color >> 8) & 0xFF) as f32;
            let base_b = (base_color & 0xFF) as f32;

            let mut accum_r = base_r;
            let mut accum_g = base_g;
            let mut accum_b = base_b;

            // Sample along the ray towards the light
            for _ in 0..config.num_samples {
                current_u -= delta_u;
                current_v -= delta_v;

                // Clamp UVs to screen bounds
                if !(0.0..1.0).contains(&current_u) || !(0.0..1.0).contains(&current_v) {
                    continue; // Skip out-of-bounds samples
                }

                let sx = (current_u * width as f32) as i32;
                let sy = (current_v * height as f32) as i32;

                let sample_idx = (sy * width + sx) as usize;
                let sample_color = original_pixels[sample_idx];

                let sample_r = ((sample_color >> 16) & 0xFF) as f32;
                let sample_g = ((sample_color >> 8) & 0xFF) as f32;
                let sample_b = (sample_color & 0xFF) as f32;

                // Accumulate scaled by weight and decay
                accum_r += sample_r * illumination_decay * config.weight;
                accum_g += sample_g * illumination_decay * config.weight;
                accum_b += sample_b * illumination_decay * config.weight;

                illumination_decay *= config.decay;
            }

            // Apply exposure and clamp to 0-255
            let final_r = (accum_r * config.exposure).clamp(0.0, 255.0) as u32;
            let final_g = (accum_g * config.exposure).clamp(0.0, 255.0) as u32;
            let final_b = (accum_b * config.exposure).clamp(0.0, 255.0) as u32;

            pixels[base_idx] = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
        }
    }
}
