//! Lens Distortion Filter
//!
//! A post-processing effect that applies radial distortion to the framebuffer,
//! simulating barrel or pincushion distortion found in camera lenses.

#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Lens Distortion effect.
#[derive(Debug, Clone, Copy)]
pub struct LensDistortionConfig {
    /// Distortion coefficient.
    /// Positive values create pincushion distortion, negative values create barrel distortion.
    /// Zero means no distortion.
    pub distortion: f32,
    /// The scale factor applied to the coordinates to avoid zooming in too much.
    /// A value of 1.0 means no additional scaling.
    pub scale: f32,
}

impl Default for LensDistortionConfig {
    fn default() -> Self {
        Self {
            distortion: 0.5,
            scale: 1.0,
        }
    }
}

/// Applies a Lens Distortion filter to the given framebuffer.
pub fn apply_lens_distortion(fb: &mut Framebuffer, config: LensDistortionConfig) {
    if config.distortion == 0.0 {
        return;
    }

    let width = fb.width();
    let height = fb.height();

    if width == 0 || height == 0 {
        return;
    }

    let src_buffer = fb.as_slice().to_vec();
    let dest_buffer = fb.as_mut_slice();

    let half_w = width as f32 * 0.5;
    let half_h = height as f32 * 0.5;

    // Use the maximum dimension to normalize coordinates to roughly [-1.0, 1.0]
    // so distortion behaves consistently across aspect ratios.
    let max_dim = half_w.max(half_h);
    let inv_max_dim = 1.0 / max_dim;

    #[cfg(feature = "parallel")]
    let iter = dest_buffer.par_chunks_exact_mut(width as usize).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = dest_buffer.chunks_exact_mut(width as usize).enumerate();

    iter.for_each(|(y_usize, row)| {
        let y_f32 = y_usize as f32;
        // Normalized y coordinate [-1.0, 1.0]
        let dy = (y_f32 - half_h) * inv_max_dim;
        let dy_sq = dy * dy;

        for (x_usize, pixel_out) in row.iter_mut().enumerate() {
            let x_f32 = x_usize as f32;
            let dx = (x_f32 - half_w) * inv_max_dim;
            let r_sq = dx * dx + dy_sq;

            // Simple radial distortion model: r' = r * (1 + k * r^2)
            // Or using scaling: p' = p * scale * (1 + k * r^2)
            let f = config.scale * (1.0 + config.distortion * r_sq);

            let new_dx = dx * f;
            let new_dy = dy * f;

            // Convert back to pixel coordinates
            let src_x = (new_dx * max_dim + half_w) as i32;
            let src_y = (new_dy * max_dim + half_h) as i32;

            if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                let src_idx = (src_y as u32 * width + src_x as u32) as usize;
                *pixel_out = src_buffer[src_idx];
            } else {
                // Out of bounds becomes black
                *pixel_out = 0xFF00_0000;
            }
        }
    });
}
