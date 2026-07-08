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

const fn generate_plasma_lut() -> [u32; 1024] {
    let mut lut = [0u32; 1024];
    let mut i = 0;
    while i < 1024 {
        let r = (i * 255) / 1023;

        let mut g_c = i + 338;
        if g_c >= 1024 { g_c -= 1024; }
        let g = (g_c * 255) / 1023;

        let mut b_c = i + 676;
        if b_c >= 1024 { b_c -= 1024; }
        let b = (b_c * 255) / 1023;

        lut[i as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        i += 1;
    }
    lut
}
const PLASMA_LUT: [u32; 1024] = generate_plasma_lut();

pub fn apply_plasma(fb: &mut Framebuffer, time: f32, scale: f32) {
    if fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let width = fb.width() as usize;
    if width == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    // Precalculate x_sin for all x columns
    let mut x_sins = std::vec::Vec::with_capacity(width);
    for x in 0..width {
        let x_f32 = x as f32;
        let x_scaled_time = (x_f32 * scale + time) % std::f32::consts::TAU;
        x_sins.push(fast_sin_cos(x_scaled_time).0);
    }

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        let y_scaled_time = (y_f32 * scale + time) % std::f32::consts::TAU;
        let (y_sin, y_cos) = fast_sin_cos(y_scaled_time);

        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let x_sin = x_sins[x];

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

            // Use the precomputed LUT for color mapping
            let t = (c * 1023.0) as u32;
            let t = t.min(1023);
            *pixel = unsafe { *PLASMA_LUT.get_unchecked(t as usize) };
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
