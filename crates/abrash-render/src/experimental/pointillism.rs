//! Pointillism Filter
//!
//! A non-photorealistic post-processing effect that converts the image into a Pointillist painting.
//! Instead of drawing rectangular pixels, it draws overlapping, slightly randomized circles (dots)
//! based on the underlying image colors, leaving a canvas color visible in the gaps.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static POINTILLISM_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Pointillism effect.
#[derive(Debug, Clone, Copy)]
pub struct PointillismConfig {
    /// The spacing between dots in pixels.
    pub dot_spacing: f32,
    /// The radius of each dot in pixels.
    pub dot_radius: f32,
    /// The maximum random jitter applied to a dot's position.
    pub jitter_amount: f32,
    /// The background canvas color (ARGB) visible between dots.
    pub canvas_color: u32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            dot_spacing: 8.0,
            dot_radius: 5.0,
            jitter_amount: 3.0,
            canvas_color: 0xFF_F0F0D0, // Off-white / warm paper
        }
    }
}

/// Applies a Pointillism effect to the framebuffer.
///
/// This filter calculates which randomly-jittered dot (if any) covers each pixel.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration parameters.
pub fn apply_pointillism(fb: &mut Framebuffer, config: &PointillismConfig) {
    if config.dot_spacing <= 0.0 || config.dot_radius <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let total_pixels = width.saturating_mul(height);

    // Cache the original image
    POINTILLISM_BUFFER.with(|buffer_ref| {
        let mut buffer = buffer_ref.borrow_mut();
        let size = total_pixels;
        if buffer.len() < size {
            buffer.resize(size, 0);
        }

        let src_fb = &mut buffer[..size];
        src_fb.copy_from_slice(fb.as_slice());

        // Fast parallel map over all pixels
        let dest_pixels = fb.as_mut_slice();

        // We cannot borrow `buffer` (which is a RefMut) across a parallel boundary.
        // Instead, we just pass the slice!
        let src_fb = &buffer[..size];

        #[cfg(feature = "parallel")]
        let iter = dest_pixels.par_iter_mut().enumerate();

        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels.iter_mut().enumerate();

        iter.for_each(|(idx, pixel)| {
            // Re-borrow the slice inside the closure
            let src_slice = src_fb;
            let px = (idx % width) as f32;
            let py = (idx / width) as f32;

            let cell_x = (px / config.dot_spacing).floor() as i32;
            let cell_y = (py / config.dot_spacing).floor() as i32;

            let mut best_color = config.canvas_color;
            let mut min_dist_sq = f32::MAX;

            // Check the 3x3 neighborhood of cells to see if their dot covers this pixel
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let cx = cell_x + dx;
                    let cy = cell_y + dy;

                    // Pseudo-random seed based on cell coordinates
                    let seed = ((cx.wrapping_mul(73856093)) ^ (cy.wrapping_mul(19349663))) as u32;
                    let mut rng = XorShift32::new(if seed == 0 { 1 } else { seed });

                    // Generate jitter [-1.0, 1.0]
                    let jx = rng.next_f32() * 2.0 - 1.0;
                    let jy = rng.next_f32() * 2.0 - 1.0;

                    let dot_center_x = (cx as f32 + 0.5) * config.dot_spacing + jx * config.jitter_amount;
                    let dot_center_y = (cy as f32 + 0.5) * config.dot_spacing + jy * config.jitter_amount;

                    let dx_dist = px - dot_center_x;
                    let dy_dist = py - dot_center_y;
                    let dist_sq = dx_dist * dx_dist + dy_dist * dy_dist;

                    if dist_sq <= config.dot_radius * config.dot_radius {
                        // In case of overlapping dots, we can pick the closest center
                        if dist_sq < min_dist_sq {
                            min_dist_sq = dist_sq;

                            // Sample original image at the dot's center
                            let sx = dot_center_x.clamp(0.0, width as f32 - 1.0) as usize;
                            let sy = dot_center_y.clamp(0.0, height as f32 - 1.0) as usize;
                            best_color = src_slice[sy * width + sx];
                        }
                    }
                }
            }

            *pixel = best_color;
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointillism_basic() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF_FF0000); // Red image

        let config = PointillismConfig {
            dot_spacing: 8.0,
            dot_radius: 3.0,
            jitter_amount: 0.0,
            canvas_color: 0xFF_FFFFFF, // White canvas
        };

        apply_pointillism(&mut fb, &config);

        // Center of dot at (4.0, 4.0). Radius 3, so pixel (4,4) should be Red.
        assert_eq!(fb.get_pixel(4, 4), Some(0xFF_FF0000));

        // Pixel at (0,0) is distance sqrt(32) > 3, so should be canvas color.
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF_FFFFFF));
    }
}
