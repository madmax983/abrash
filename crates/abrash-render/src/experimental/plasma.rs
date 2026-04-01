//! Plasma pattern effect.
//!
//! Simulates a classic demoscene plasma effect using sine waves.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Plasma stylization filter to the framebuffer.
///
/// This filter converts the image into a shifting, colorful pattern based on
/// mathematical sine functions, mimicking a classic demoscene effect.
///
/// * `fb`: The Framebuffer to modify.
/// * `time`: A time variable used to animate the plasma.
/// * `scale`: A scaling factor for the sine waves (e.g., 0.05).
pub fn apply_plasma(fb: &mut Framebuffer, time: f32, scale: f32) {
    let width = fb.width() as usize;
    if width == 0 || fb.height() == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;

        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let x_f32 = x as f32;

            // Calculate plasma value using multiple sine waves
            let mut v = 0.0;
            v += (x_f32 * scale + time).sin();
            v += (y_f32 * scale + time).sin();
            v += ((x_f32 * scale + time).sin() + (y_f32 * scale + time).cos()).sin();

            // Map the value from [-3.0, 3.0] to roughly [0.0, 1.0]
            // We use PI to create cyclical colors
            let c = (v * std::f32::consts::PI).sin() * 0.5 + 0.5;

            // Map the normalized value to RGB colors
            // Simple color palette generation based on the phase
            let r = ((c * 255.0) as u32).min(255);
            let g = (((c + 0.33) % 1.0 * 255.0) as u32).min(255);
            let b = (((c + 0.66) % 1.0 * 255.0) as u32).min(255);

            *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_plasma_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        apply_plasma(&mut fb, 0.0, 0.05);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_apply_plasma_standard() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_00_00_00);
        apply_plasma(&mut fb, 0.0, 0.05);

        // Verify that the framebuffer has been altered and is no longer black.
        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Framebuffer should not be entirely black after plasma effect"
        );
    }
}
