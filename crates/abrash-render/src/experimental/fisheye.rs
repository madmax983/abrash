//! Fisheye post-processing effect.
//!
//! Simulates a wide-angle lens by warping the image spherically outward
//! from a center point.

use crate::framebuffer::Framebuffer;

/// Configuration for the Fisheye post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct FisheyeConfig {
    /// The X coordinate of the effect center (0.0 to 1.0, where 0.5 is the middle).
    pub center_x: f32,
    /// The Y coordinate of the effect center (0.0 to 1.0, where 0.5 is the middle).
    pub center_y: f32,
    /// The strength of the fisheye distortion (higher is more distorted).
    pub strength: f32,
}

impl Default for FisheyeConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            strength: 2.0,
        }
    }
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Fisheye filter to the framebuffer in-place.
///
/// Warps the image pixels spherically based on their distance from the center,
/// simulating a wide-angle lens effect.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `config` - The configuration parameters for the fisheye effect.
///
/// # Panics
///
/// Panics if the framebuffer slice length does not exactly match `width * height`.
pub fn apply_fisheye(fb: &mut Framebuffer, config: &FisheyeConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Clone the source framebuffer to safely sample non-linear pixel displacements
    // without aliasing issues (as learned from the Water Ripple and Kaleidoscope filters).
    let source_pixels = fb.as_slice().to_vec();

    // To satisfy Havoc/Forge panics and par_chunks_exact_mut bounds rules:
    assert_eq!(fb.as_slice().len(), width * height);

    let dest_pixels = fb.as_mut_slice();

    let cx = config.center_x * width as f32;
    let cy = config.center_y * height as f32;

    // Normalize coordinates based on the shortest dimension to keep the fisheye perfectly circular
    let min_dim = (width as f32).min(height as f32);
    let inv_half_min_dim = 2.0 / min_dim;

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let dy = (y as f32 - cy) * inv_half_min_dim;
        let dy2 = dy * dy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = (x as f32 - cx) * inv_half_min_dim;
            let distance2 = dx * dx + dy2;

            // Only apply within a unit circle
            if distance2 <= 1.0 {
                #[allow(clippy::imprecise_flops)]
                let distance = distance2.sqrt();
                // Apply fisheye distortion formula.
                // If strength = 1.0, r = distance (no distortion).
                // If strength > 1.0, r < distance for r < 1 (bulges outward).
                // If strength < 1.0, r > distance for r < 1 (pinches inward).
                let r = distance.powf(config.strength);

                // Optimized math: instead of `atan2` then `cos`/`sin`, scale directly by r / distance
                let scale = if distance > 0.0 { r / distance } else { 0.0 };
                let warped_x = dx * scale;
                let warped_y = dy * scale;

                // Convert back to screen space (fast float-to-int casts instead of round())
                let src_x = (warped_x / inv_half_min_dim + cx) as i32;
                let src_y = (warped_y / inv_half_min_dim + cy) as i32;

                if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                    *pixel = source_pixels[src_y as usize * width + src_x as usize];
                } else {
                    *pixel = 0xFF_00_00_00;
                }
            } else {
                // Keep original pixel outside the fisheye effect
                *pixel = source_pixels[y * width + x];
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_fisheye() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // A red dot in the center, black elsewhere
        fb.clear(0xFF_00_00_00);
        fb.set_pixel(2, 2, 0xFF_FF_00_00);

        let config = FisheyeConfig {
            center_x: 0.5,
            center_y: 0.5,
            strength: 1.5,
        };

        apply_fisheye(&mut fb, &config);

        // We expect the center pixel to remain red (or sample a neighbor).
        let p = fb.get_pixel(2, 2).unwrap();
        let neighbor = fb.get_pixel(1, 1).unwrap();
        assert!(p == 0xFF_FF_00_00 || neighbor == 0xFF_FF_00_00);
    }
}
