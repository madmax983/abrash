//! Vignette post-processing effect.
//!
//! Applies a darkening effect towards the edges of the image, drawing focus to the center.

use crate::framebuffer::Framebuffer;

/// Configuration for the Vignette post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct VignetteConfig {
    /// The X coordinate of the vignette center (0.0 to 1.0, where 0.5 is the middle).
    pub center_x: f32,
    /// The Y coordinate of the vignette center (0.0 to 1.0, where 0.5 is the middle).
    pub center_y: f32,
    /// The radius at which the darkening begins (as a fraction of the maximum distance to a corner, 0.0 to 1.0).
    pub inner_radius: f32,
    /// The radius at which the darkening reaches maximum intensity (as a fraction of the max distance, 0.0 to 1.0).
    pub outer_radius: f32,
    /// The maximum intensity of the darkness (0.0 = no effect, 1.0 = pitch black).
    pub intensity: f32,
}

impl Default for VignetteConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            inner_radius: 0.2,
            outer_radius: 1.0,
            intensity: 0.8,
        }
    }
}

/// Applies a Vignette filter to the framebuffer in-place.
///
/// Darkens pixels based on their distance from the center point.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `config` - The configuration parameters for the vignette effect.
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// # Panics
///
/// Panics if the framebuffer slice length does not exactly match `width * height`.
pub fn apply_vignette(fb: &mut Framebuffer, config: &VignetteConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // To satisfy Havoc/Forge panics and par_chunks_exact_mut bounds rules:
    assert_eq!(fb.as_slice().len(), width * height);

    let dest_pixels = fb.as_mut_slice();

    // The maximum possible distance from the center is to one of the corners.
    // We treat distance in UV coordinate space (0.0 to 1.0) for consistency,
    // handling aspect ratio scaling so the vignette is perfectly elliptical.
    let aspect_ratio = width as f32 / height as f32;
    let cx_uv = config.center_x;
    let cy_uv = config.center_y;

    // Maximum distance from center to corner in UV space (adjusted by aspect ratio).
    let max_dx_uv = cx_uv.max(1.0 - cx_uv) * aspect_ratio;
    let max_dy_uv = cy_uv.max(1.0 - cy_uv);
    let max_dist_sq = max_dx_uv * max_dx_uv + max_dy_uv * max_dy_uv;

    let inner_sq = config.inner_radius * config.inner_radius * max_dist_sq;
    let outer_sq = config.outer_radius * config.outer_radius * max_dist_sq;

    // Avoid division by zero if inner and outer radius are the same
    let range_sq = (outer_sq - inner_sq).max(0.000_001);

    // Precalculate inverse width/height to avoid division in the loop
    let inv_width = 1.0 / (width as f32);
    let inv_height = 1.0 / (height as f32);

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let v = (y as f32 + 0.5) * inv_height; // Center of pixel
        let dy = v - cy_uv;
        let dy_sq = dy * dy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let u = (x as f32 + 0.5) * inv_width; // Center of pixel
            let dx = (u - cx_uv) * aspect_ratio;
            let dist_sq = dx * dx + dy_sq;

            if dist_sq > inner_sq {
                // Calculate how far we are between inner and outer radius (0.0 to 1.0)
                let t = ((dist_sq - inner_sq) / range_sq).clamp(0.0, 1.0);

                // Smoothstep-like interpolation for smoother falloff: 3t^2 - 2t^3
                let smooth_t = t * t * (3.0 - 2.0 * t);

                // Calculate darkening factor (1.0 = unchanged, 0.0 = completely dark)
                let darkening = 1.0 - (smooth_t * config.intensity);

                // Apply darkening to RGB channels
                let p = *pixel;
                let a = p & 0xFF_00_00_00;
                let r = (((p >> 16) & 0xFF) as f32 * darkening) as u32;
                let g = (((p >> 8) & 0xFF) as f32 * darkening) as u32;
                let b = ((p & 0xFF) as f32 * darkening) as u32;

                *pixel = a | (r << 16) | (g << 8) | b;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_vignette() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // Fill with white
        fb.clear(0xFF_FF_FF_FF);

        let config = VignetteConfig {
            center_x: 0.5,
            center_y: 0.5,
            inner_radius: 0.0,
            outer_radius: 1.0,
            intensity: 1.0,
        };

        apply_vignette(&mut fb, &config);

        let center = fb.get_pixel(2, 2).unwrap();
        let corner = fb.get_pixel(0, 0).unwrap();

        // In a 5x5 grid with center 0.5/0.5, pixel (2,2) is the exact center.
        // It should remain unchanged (white) because its distance is 0,
        // which is not greater than inner_sq (0.0).
        assert_eq!(center, 0xFF_FF_FF_FF);

        // Corner should be darker than center
        let center_r = (center >> 16) & 0xFF;
        let corner_r = (corner >> 16) & 0xFF;

        assert!(
            corner_r < center_r,
            "Corner {corner_r} should be darker than center {center_r}"
        );
    }
}
