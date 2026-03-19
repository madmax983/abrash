//! Fisheye Lens Distortion Filter
//!
//! A retro post-processing effect that applies a radial distortion to the framebuffer,
//! simulating the bulging effect of an ultra-wide fisheye lens.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Fisheye effect.
#[derive(Debug, Clone, Copy)]
pub struct FisheyeConfig {
    /// The strength of the distortion.
    /// Positive values create a barrel (bulging) distortion.
    /// Negative values create a pincushion (pinching) distortion.
    /// Recommended range is typically [-1.0, 1.0].
    pub strength: f32,
    /// A zoom factor applied after distortion to reduce black borders.
    /// Typically >= 1.0. A value of 1.0 means no extra zoom.
    pub zoom: f32,
}

impl Default for FisheyeConfig {
    fn default() -> Self {
        Self {
            strength: 0.5,
            zoom: 1.0,
        }
    }
}

thread_local! {
    // Thread-local buffer to avoid massive allocations per frame while preventing
    // mutable aliasing when reading source pixels and writing destination pixels.
    static FISHEYE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a Fisheye lens distortion effect to the framebuffer.
///
/// Uses reverse mapping: for each destination pixel, it calculates the corresponding
/// source pixel coordinates using a polynomial radial distortion formula.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration containing `strength` and `zoom`.
pub fn apply_fisheye(fb: &mut Framebuffer, config: &FisheyeConfig) {
    if config.strength == 0.0 {
        return; // No distortion requested
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let mut source_pixels = FISHEYE_BUFFER.with(std::cell::RefCell::take);

    let fb_slice = fb.as_slice();
    if source_pixels.len() != fb_slice.len() {
        source_pixels.resize(fb_slice.len(), 0);
    }
    source_pixels.copy_from_slice(fb_slice);

    let dest = fb.as_mut_slice();
    let src: &[u32] = &source_pixels;

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // Normalize spatial coordinates using the shortest screen dimension to prevent
    // elliptical warping. The radius '1.0' will touch the closest edge.
    let shortest_dim = width.min(height) as f32;
    let inv_half_shortest = 2.0 / shortest_dim;
    let half_shortest = shortest_dim / 2.0;

    // Zoom scale applied directly to the lookup coordinates
    let inv_zoom = 1.0 / config.zoom;

    #[cfg(feature = "parallel")]
    let row_iter = dest.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        // Normalize y to roughly [-1.0, 1.0] based on the shortest dimension
        let ny = (y_f32 - cy) * inv_half_shortest;

        for (x, pixel) in row.iter_mut().enumerate() {
            let x_f32 = x as f32;
            // Normalize x to roughly [-1.0, 1.0]
            let nx = (x_f32 - cx) * inv_half_shortest;

            // Calculate distance from center (radius)
            // Use squared distance to avoid sqrt if possible, but true radial distortion needs r
            let r2 = nx * nx + ny * ny;
            let r = r2.sqrt();

            // Only apply distortion inside a reasonable radius to prevent extreme wrapping
            // Typical fisheye functions wrap back or shoot to infinity past a certain point.
            // By doing it based on r, we get a circular lens effect.

            // Reverse mapping: source radius given destination radius
            // r_src = r_dst * (1.0 + strength * r_dst^2)
            // This pulls pixels from further away towards the center (barrel).
            let mut r_src = r * (1.0 + config.strength * r2);

            // Apply zoom
            r_src *= inv_zoom;

            // If r == 0, scale factor is 1.0 to avoid NaN division
            let scale = if r > 0.0001 { r_src / r } else { 1.0 };

            // Map back to unnormalized pixel coordinates
            let src_x_f32 = cx + (nx * scale * half_shortest);
            let src_y_f32 = cy + (ny * scale * half_shortest);

            // Fast float-to-int cast
            let src_x = src_x_f32 as i32;
            let src_y = src_y_f32 as i32;

            // Check bounds. If outside, render black.
            if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                *pixel = src[(src_y as usize) * width + (src_x as usize)];
            } else {
                *pixel = 0xFF000000; // Black border
            }
        }
    });

    FISHEYE_BUFFER.with(|buf| buf.replace(source_pixels));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_fisheye_barrel_distortion() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Clear to white
        fb.clear(0xFFFFFFFF);

        // Draw a black dot slightly off center
        fb.set_pixel(60, 50, 0xFF000000);

        let config = FisheyeConfig {
            strength: 1.0, // Strong barrel distortion
            zoom: 1.0,
        };

        apply_fisheye(&mut fb, &config);

        // Center should remain unchanged (white, unless we placed the dot exactly at 50,50)
        assert_eq!(fb.get_pixel(50, 50), Some(0xFFFFFFFF));

        // The pixel at (60, 50) in destination pulls from a source further away.
        // center is 50. nx = (60-50) / 50 = 0.2
        // r = 0.2
        // r_src = 0.2 * (1.0 + 1.0 * 0.04) = 0.2 * 1.04 = 0.208
        // src_x = 50 + 0.208 * 50 = 50 + 10.4 = 60.4 -> 60
        // So destination (60, 50) pulls from source (60, 50).
        // Let's test a further pixel where the difference is more pronounced.
        // Let destination be (80, 50). nx = 30/50 = 0.6. r = 0.6.
        // r_src = 0.6 * (1.0 + 1.0 * 0.36) = 0.6 * 1.36 = 0.816
        // src_x = 50 + 0.816 * 50 = 50 + 40.8 = 90.8 -> 90.
        // So destination (80, 50) reads from source (90, 50).

        // Let's redraw frame for specific test
        fb.clear(0xFFFFFFFF);
        fb.set_pixel(90, 50, 0xFF000000); // Set source pixel to black

        apply_fisheye(&mut fb, &config);

        // Check if destination (80, 50) got the black pixel
        assert_eq!(fb.get_pixel(80, 50), Some(0xFF000000));

        // Source pixel (90, 50) in destination should map to:
        // nx = 40/50 = 0.8. r = 0.8.
        // r_src = 0.8 * (1.0 + 1.0 * 0.64) = 0.8 * 1.64 = 1.312
        // src_x = 50 + 1.312 * 50 = 50 + 65.6 = 115 -> Out of bounds!
        // Should be black (0xFF000000).
        assert_eq!(fb.get_pixel(90, 50), Some(0xFF000000));
    }
}
