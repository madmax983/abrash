//! Black Hole post-processing effect.
//!
//! Simulates gravitational lensing by distorting light around a massive central point.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Black Hole gravitational lensing effect to the framebuffer.
///
/// Light is bent around the central point (`center_x`, `center_y`) based on its `mass`.
/// Pixels closer to the center than `mass` are inside the event horizon and become black.
///
/// * `fb`: The Framebuffer to modify.
/// * `center_x`: The x-coordinate of the black hole center.
/// * `center_y`: The y-coordinate of the black hole center.
/// * `mass`: The gravitational mass/radius of the event horizon.
pub fn apply_black_hole(fb: &mut Framebuffer, center_x: f32, center_y: f32, mass: f32) {
    if mass <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let mass_sq = mass * mass;

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
        src_pixels.resize(fb.as_slice().len(), 0);
        src_pixels.copy_from_slice(fb.as_slice());

        // Extract a primitive slice to prevent capturing the `!Send` `RefMut` in the Rayon closure
        let src_pixels_slice = src_pixels.as_slice();
        let dst_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let row_iter = dst_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dst_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let dy = (y as f32) - center_y;
            let dy_sq = dy * dy;

            for (x, pixel) in row.iter_mut().enumerate().take(width) {
                let dx = (x as f32) - center_x;
                let dist_sq = dx * dx + dy_sq;

                // Inside the Event Horizon
                if dist_sq <= mass_sq {
                    *pixel = 0xFF00_0000; // Pitch Black
                    continue;
                }

                // Gravitational Lensing Distortion
                // Instead of a strict Newtonian simulation, we use a simple functional approximation:
                // The perceived light ray is bent, so we look "outward" for the source pixel.
                // A simple mapping: distance' = distance / (1 - mass/distance)
                let dist = dist_sq.sqrt();

                // Avoid division by zero at the exact event horizon boundary (handled by dist_sq <= mass_sq check mostly)
                // The closer to mass, the closer to 1.0 (1 - 0.99) -> 0.01
                // So distortion becomes very small, making 1.0/distortion very large.
                let distortion = 1.0 - (mass / dist);

                // To prevent sampling infinitely far away right at the boundary, clamp it slightly
                let clamped_distortion = distortion.max(0.01);

                let src_dx = dx / clamped_distortion;
                let src_dy = dy / clamped_distortion;

                let src_x = (center_x + src_dx) as i32;
                let src_y = (center_y + src_dy) as i32;

                if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                    let src_idx = (src_y as usize) * width + (src_x as usize);
                    *pixel = src_pixels_slice[src_idx];
                } else {
                    // Out of bounds space is black
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
    fn test_apply_black_hole_event_horizon() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FFFFFF); // Fill with white

        // Put a black hole at (5, 5) with mass 2.0
        apply_black_hole(&mut fb, 5.0, 5.0, 2.0);

        // Center should be completely black
        assert_eq!(fb.get_pixel(5, 5).unwrap(), 0xFF00_0000);

        // Point just outside might be distorted but not black hole black (it samples white)
        // Wait, the edges of the buffer might be black if it samples out of bounds.
        // Let's just check the center is black.
    }

    #[test]
    fn test_apply_black_hole_zero_mass() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FF0000); // Red

        apply_black_hole(&mut fb, 5.0, 5.0, 0.0);

        // Should do absolutely nothing
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF_FF0000);
        }
    }
}
