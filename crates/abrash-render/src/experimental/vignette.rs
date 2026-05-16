//! Vignette Filter
//!
//! A post-processing effect that darkens the corners of the image to draw
//! focus to the center.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Vignette filter.
#[derive(Debug, Clone, Copy)]
pub struct VignetteConfig {
    /// The intensity of the darkening at the edges (0.0 to 1.0)
    pub intensity: f32,
    /// The size of the un-darkened center radius (0.0 to 1.0)
    pub radius: f32,
}

impl Default for VignetteConfig {
    fn default() -> Self {
        Self {
            intensity: 0.8,
            radius: 0.5,
        }
    }
}

/// Applies a vignette effect to the framebuffer.
///
/// Pixels near the edges will be darkened based on the `intensity` and `radius`.
pub fn apply_vignette(fb: &mut Framebuffer, config: &VignetteConfig) {
    if config.intensity <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    let half_w = width as f32 * 0.5;
    let half_h = height as f32 * 0.5;

    // Calculate the maximum possible distance from the center (to the corner)
    let max_dist_sq = half_w * half_w + half_h * half_h;
    let max_dist = max_dist_sq.sqrt();

    let radius_dist = max_dist * config.radius;
    let falloff_range = max_dist - radius_dist;

    if falloff_range <= 0.0 {
        return; // Radius is so large it covers everything
    }

    let inv_falloff = 1.0 / falloff_range;

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let dy = y as f32 - half_h;
        let dy_sq = dy * dy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = x as f32 - half_w;

            // Note: f32::hypot is slow, using sqrt(dx*dx + dy*dy) per .jules/bolt.md
            #[allow(clippy::imprecise_flops)]
            let dist = (dx * dx + dy_sq).sqrt();

            if dist > radius_dist {
                // Calculate darkening factor from 0.0 (edge of radius) to 1.0 (screen corner)
                let mut factor = (dist - radius_dist) * inv_falloff;

                // Square it for a smoother curve
                factor *= factor;

                // Apply intensity scale
                let darken = 1.0 - (factor * config.intensity).min(1.0);

                let p = *pixel;
                let a = (p >> 24) & 0xFF;
                let r = (p >> 16) & 0xFF;
                let g = (p >> 8) & 0xFF;
                let b = p & 0xFF;

                let r_new = ((r as f32) * darken) as u32;
                let g_new = ((g as f32) * darken) as u32;
                let b_new = ((b as f32) * darken) as u32;

                *pixel = (a << 24) | (r_new << 16) | (g_new << 8) | b_new;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vignette_darkens_edges() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFFFF_FFFF); // Solid white

        let config = VignetteConfig {
            intensity: 1.0,
            radius: 0.1, // Very small un-darkened center
        };

        apply_vignette(&mut fb, &config);

        // Center should remain pure white
        assert_eq!(fb.get_pixel(50, 50).unwrap(), 0xFFFF_FFFF);

        // Corners should be completely black (or very close to it)
        let top_left = fb.get_pixel(0, 0).unwrap() & 0x00FF_FFFF; // ignore alpha
        assert!(top_left < 0x0010_1010, "Corner pixel should be dark, was 0x{:08X}", top_left);

        let bottom_right = fb.get_pixel(99, 99).unwrap() & 0x00FF_FFFF;
        assert!(bottom_right < 0x0010_1010, "Corner pixel should be dark, was 0x{:08X}", bottom_right);
    }
}
