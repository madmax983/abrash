//! Fisheye Lens Filter
//!
//! Simulates an ultra-wide-angle lens distortion (barrel distortion) that warps the image,
//! creating a bulbous, spherical effect where the center is magnified and the edges are compressed.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Fisheye lens distortion filter to the framebuffer.
///
/// This filter maps screen coordinates to normalized space `(-1.0..1.0)` and applies a
/// non-linear radial mapping: `r_new = r * (1.0 + strength * r^2)`.
///
/// * `fb`: The Framebuffer to modify.
/// * `strength`: The strength of the barrel distortion. `0.0` is no distortion.
///               Positive values bubble the image outward (fisheye).
pub fn apply_fisheye(fb: &mut Framebuffer, strength: f32) {
    if strength == 0.0 {
        return; // Identity, no distortion needed
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;

    // Use the shortest dimension to normalize space so the distortion remains perfectly circular
    let min_dim = half_w.min(half_h);

    // Bolt Performance Optimization:
    // We must clone the source framebuffer because this is a spatial effect where a
    // destination pixel might need to sample from an arbitrary source location.
    // By hoisting this buffer into a `thread_local!` we eliminate a `Vec` heap allocation
    // (via `.to_vec()`) per frame, reducing memory fragmentation and allocation overhead.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    SOURCE_PIXELS.with(|buf| {
        let mut src_pixels = buf.borrow_mut();
        src_pixels.clear();
        src_pixels.extend_from_slice(fb.as_slice());

        // Extract a primitive slice to prevent capturing the `!Send` `RefMut` in the Rayon closure
        let src_pixels_slice = src_pixels.as_slice();
        let dst_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let row_iter = dst_pixels.par_chunks_exact_mut(width.max(1)).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dst_pixels.chunks_exact_mut(width.max(1)).enumerate();

        row_iter.for_each(|(y, row)| {
            let dy = (y as f32) - half_h;

            for (x, pixel) in row.iter_mut().enumerate().take(width) {
                let dx = (x as f32) - half_w;

                // Convert to normalized coordinates based on the shortest dimension
                let nx = dx / min_dim;
                let ny = dy / min_dim;

                // Calculate distance from center
                let r_sq = nx * nx + ny * ny;

                // Apply Fisheye (Barrel) Distortion formula: r' = r * (1 + strength * r^2)
                // By factoring out the `r`, we can compute the scaling factor directly.
                let distortion_factor = 1.0 + strength * r_sq;

                // Calculate the new mapped normalized coordinates
                let nx_new = nx * distortion_factor;
                let ny_new = ny * distortion_factor;

                // Convert back to pixel coordinates
                let src_x = (nx_new * min_dim + half_w) as i32;
                let src_y = (ny_new * min_dim + half_h) as i32;

                // Check bounds to ensure we sample valid pixels
                if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                    let src_idx = (src_y as usize) * width + (src_x as usize);
                    *pixel = src_pixels_slice[src_idx];
                } else {
                    // Out of bounds (edges shrunk by fisheye mapping) become black
                    *pixel = 0xFF00_0000;
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fisheye_identity() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_FFFF);
        // Apply with 0 strength should do nothing
        apply_fisheye(&mut fb, 0.0);
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFFFF_FFFF);
        }
    }

    #[test]
    fn test_fisheye_distortion() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_FFFF);
        // Draw a black dot in the center
        fb.set_pixel(5, 5, 0xFF00_0000);

        // Apply strong fisheye
        apply_fisheye(&mut fb, 0.5);

        // It shouldn't crash and the buffer should still be primarily white
        let has_black = fb.as_slice().iter().any(|&p| p == 0xFF00_0000);
        assert!(has_black);
    }
}
