//! Polar Inversion ("Tiny Planet") Filter
//!
//! A post-processing effect that transforms Cartesian screen coordinates into
//! polar coordinates, inverts the radius, and samples back to create a stereographic
//! distortion, commonly known as a "Tiny Planet" or "Black Hole" effect.

use abrash_core::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static POLAR_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Polar Inversion effect.
#[derive(Debug, Clone, Copy)]
pub struct PolarInversionConfig {
    /// Center of the inversion in normalized coordinates (0.0 to 1.0).
    pub center_x: f32,
    /// Center of the inversion in normalized coordinates (0.0 to 1.0).
    pub center_y: f32,
    /// The base radius of the effect.
    pub radius: f32,
    /// Zoom factor to scale the distorted result.
    pub zoom: f32,
}

impl Default for PolarInversionConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 100.0,
            zoom: 1.0,
        }
    }
}

/// Applies a Polar Inversion effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration containing center, radius, and zoom.
pub fn apply_polar_inversion(fb: &mut Framebuffer, config: &PolarInversionConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let center_x_px = config.center_x * width as f32;
    let center_y_px = config.center_y * height as f32;
    let radius2 = config.radius * config.radius;
    let zoom = config.zoom.max(0.001); // Prevent division by zero

    // Clone the source framebuffer into thread-local storage to prevent read/write aliasing
    POLAR_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();
        buf.extend_from_slice(fb.as_slice());

        let src = buf.as_slice();
        let pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let iter = pixels.chunks_exact_mut(width).enumerate().par_bridge();
        #[cfg(not(feature = "parallel"))]
        let iter = pixels.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            let dy = y as f32 - center_y_px;

            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - center_x_px;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq == 0.0 {
                    // At the exact center, map to infinity (or an edge)
                    // We can just keep it or map to a fixed color.
                    // For now, let's keep it untouched or pick a border color.
                    continue;
                }

                // Invert the radius: r' = r0^2 / r
                // But in Cartesian, this is:
                // x' = x * (r0^2 / r^2)
                // y' = y * (r0^2 / r^2)
                let factor = (radius2 / dist_sq) / zoom;

                let src_x = center_x_px + dx * factor;
                let src_y = center_y_px + dy * factor;

                let ix = src_x as i32;
                let iy = src_y as i32;

                // Sample if within bounds
                if ix >= 0 && ix < width as i32 && iy >= 0 && iy < height as i32 {
                    let idx = (iy as usize) * width + (ix as usize);
                    *pixel = src[idx];
                } else {
                    // Out of bounds: fill with black or a specific background
                    *pixel = 0xFF_00_00_00;
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polar_inversion_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = PolarInversionConfig::default();
        apply_polar_inversion(&mut fb, &config);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_polar_inversion_modification() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FF_FF_FF); // White

        // A large radius ensures points very close to the center
        // get mapped out of bounds (which turn black).
        let config = PolarInversionConfig {
            center_x: 0.5,
            center_y: 0.5,
            radius: 50.0, // force many pixels out of bounds
            zoom: 1.0,
        };

        apply_polar_inversion(&mut fb, &config);

        let mut has_black = false;

        for &pixel in fb.as_slice() {
            if pixel == 0xFF_00_00_00 {
                has_black = true; // Our out-of-bounds mapping
            }
        }

        assert!(
            has_black,
            "Should have mapped some areas to black (out-of-bounds)"
        );
    }
}
