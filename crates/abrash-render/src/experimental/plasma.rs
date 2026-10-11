//! Plasma pattern effect.
//!
//! Simulates a classic demoscene plasma effect using sine waves.

use crate::framebuffer::Framebuffer;
use abrash_core::math::fast_sin_cos;

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
    if fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let width = fb.width() as usize;
    if width == 0 {
        return;
    }

    // x_sin depends only on the column, so compute it once per frame instead
    // of once per pixel (bit-identical to the per-pixel computation).
    let x_sins: Vec<f32> = (0..width)
        .map(|x| {
            let x_scaled_time = (x as f32 * scale + time) % std::f32::consts::TAU;
            fast_sin_cos(x_scaled_time).0
        })
        .collect();

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        let y_scaled_time = (y_f32 * scale + time) % std::f32::consts::TAU;
        let (y_sin, y_cos) = fast_sin_cos(y_scaled_time);

        for (pixel, &x_sin) in row.iter_mut().zip(x_sins.iter()) {
            // Calculate plasma value using multiple sine waves
            let mut v = 0.0;
            v += x_sin;
            v += y_sin;
            let (v_sin, _) = fast_sin_cos(x_sin + y_cos);
            v += v_sin;

            // Map the value from [-3.0, 3.0] to roughly [0.0, 1.0]
            // We use PI to create cyclical colors
            let (c_sin, _) = fast_sin_cos(v * std::f32::consts::PI);
            let c = c_sin * 0.5 + 0.5;

            // Map the normalized value to RGB colors
            // Simple color palette generation based on the phase
            let r = ((c * 255.0) as u32).min(255);
            // c is in [0, 1], so `% 1.0` of c + offset (offset < 1) is exactly
            // a conditional subtract; avoids a libm fmodf call per channel.
            let g_phase = c + 0.33;
            let b_phase = c + 0.66;
            let g_phase = if g_phase >= 1.0 {
                g_phase - 1.0
            } else {
                g_phase
            };
            let b_phase = if b_phase >= 1.0 {
                b_phase - 1.0
            } else {
                b_phase
            };
            let g = ((g_phase * 255.0) as u32).min(255);
            let b = ((b_phase * 255.0) as u32).min(255);

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
