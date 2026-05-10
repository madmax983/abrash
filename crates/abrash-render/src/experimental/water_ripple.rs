//! Water Ripple Filter
//!
//! A post-processing effect that applies a radial sine-wave displacement to the
//! framebuffer to simulate a water droplet ripple.

#![cfg(feature = "nova")]

use crate::framebuffer::Framebuffer;

/// Configuration for the Water Ripple effect.
#[derive(Debug, Clone, Copy)]
pub struct RippleConfig {
    /// The center X coordinate of the ripple (0.0 to 1.0).
    pub center_x: f32,
    /// The center Y coordinate of the ripple (0.0 to 1.0).
    pub center_y: f32,
    /// The amplitude (strength) of the displacement.
    pub amplitude: f32,
    /// The frequency (tightness) of the waves.
    pub frequency: f32,
    /// The phase (animation offset) of the waves.
    pub phase: f32,
    /// The maximum radius of the ripple effect (0.0 to 1.0).
    pub radius: f32,
}

impl Default for RippleConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            amplitude: 10.0,
            frequency: 50.0,
            phase: 0.0,
            radius: 0.5,
        }
    }
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies the Water Ripple effect to the given framebuffer.
pub fn apply_water_ripple(fb: &mut Framebuffer, config: RippleConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    let center_x_px = config.center_x * width as f32;
    let center_y_px = config.center_y * height as f32;
    let max_radius_px = config.radius * width.max(height) as f32;
    let inv_freq = 1.0 / config.frequency;

    // Clone the source framebuffer because non-linear displacement causes aliasing
    // when reading and writing to the same buffer concurrently.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    SOURCE_PIXELS.with(|buf| {
        let mut src_pixels = buf.borrow_mut();
        src_pixels.clear();
        src_pixels.extend_from_slice(fb.as_slice());

        let src_buffer = src_pixels.as_slice();
        let dest_buffer = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let iter = dest_buffer.par_chunks_exact_mut(width as usize).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = dest_buffer.chunks_exact_mut(width as usize).enumerate();

        iter.for_each(|(y, row)| {
            let y_f32 = y as f32;
            let dy = y_f32 - center_y_px;
            let dy_sq = dy * dy;
            let max_radius_px_sq = max_radius_px * max_radius_px;

            for (x, pixel) in row.iter_mut().enumerate() {
                let x_f32 = x as f32;
                let dx = x_f32 - center_x_px;
                let dist_sq = dx * dx + dy_sq;

                if dist_sq < max_radius_px_sq {
                    let distance = dist_sq.sqrt();
                    // Calculate displacement amount using a sine wave based on distance and phase
                    // Dampen the amplitude based on distance to the edge of the radius
                    let damping = 1.0 - (distance / max_radius_px);
                    let amount =
                        (distance * inv_freq - config.phase).sin() * config.amplitude * damping;

                    // Displacement vector (normalized dx, dy)
                    let inv_dist = if distance > 0.0 { 1.0 / distance } else { 0.0 };
                    let dir_x = dx * inv_dist;
                    let dir_y = dy * inv_dist;

                    // Calculate source pixel coordinates using fast float-to-int cast
                    let src_x = (x_f32 + dir_x * amount) as i32;
                    let src_y = (y_f32 + dir_y * amount) as i32;

                    // Clamp to screen boundaries
                    let src_x = src_x.max(0).min(width - 1);
                    let src_y = src_y.max(0).min(height - 1);

                    let src_idx = (src_y * width + src_x) as usize;
                    *pixel = src_buffer[src_idx];
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_ripple_changes_buffer() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Fill the framebuffer with a simple gradient pattern
        for y in 0..height {
            for x in 0..width {
                let color = 0xFF00_0000 | (x << 16) | (y << 8);
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        let original_fb = fb.as_slice().to_vec();

        let config = RippleConfig {
            center_x: 0.5,
            center_y: 0.5,
            amplitude: 10.0,
            frequency: 50.0,
            phase: 0.0,
            radius: 0.5,
        };

        apply_water_ripple(&mut fb, config);

        // Verify that the buffer has been changed
        let mut changed = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != original_fb[i] {
                changed = true;
                break;
            }
        }

        assert!(changed, "Water Ripple Filter should alter the framebuffer");
    }
}
