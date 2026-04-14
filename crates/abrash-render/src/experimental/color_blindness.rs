//! Color Blindness Simulator Filter
//!
//! A post-processing effect that simulates various types of color vision deficiencies (CVD).

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// The types of color blindness that can be simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBlindnessType {
    /// Red-blindness.
    Protanopia,
    /// Green-blindness.
    Deuteranopia,
    /// Blue-blindness.
    Tritanopia,
    /// Complete color blindness (monochromacy).
    Achromatopsia,
}

/// Applies a color blindness simulation filter to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `cvd_type` - The type of color blindness to simulate.
pub fn apply_color_blindness(fb: &mut Framebuffer, cvd_type: ColorBlindnessType) {
    let pixels = fb.as_mut_slice();

    // The transformation matrices are based on standard CVD models (e.g. Machado et al. 2009).
    // The values are pre-calculated for severe forms of each CVD type.
    let matrix: [f32; 9] = match cvd_type {
        ColorBlindnessType::Protanopia => [
            0.56667, 0.43333, 0.00000, 0.55833, 0.44167, 0.00000, 0.00000, 0.24167, 0.75833,
        ],
        ColorBlindnessType::Deuteranopia => [
            0.62500, 0.37500, 0.00000, 0.70000, 0.30000, 0.00000, 0.00000, 0.30000, 0.70000,
        ],
        ColorBlindnessType::Tritanopia => [
            0.95000, 0.05000, 0.00000, 0.00000, 0.43333, 0.56667, 0.00000, 0.47500, 0.52500,
        ],
        ColorBlindnessType::Achromatopsia => [
            0.29900, 0.58700, 0.11400, 0.29900, 0.58700, 0.11400, 0.29900, 0.58700, 0.11400,
        ],
    };

    // Pre-convert to fixed point integers (x65536) to avoid slow f32 operations
    // in the hot per-pixel inner loop.
    let m00 = (matrix[0] * 65536.0) as i32;
    let m01 = (matrix[1] * 65536.0) as i32;
    let m02 = (matrix[2] * 65536.0) as i32;
    let m10 = (matrix[3] * 65536.0) as i32;
    let m11 = (matrix[4] * 65536.0) as i32;
    let m12 = (matrix[5] * 65536.0) as i32;
    let m20 = (matrix[6] * 65536.0) as i32;
    let m21 = (matrix[7] * 65536.0) as i32;
    let m22 = (matrix[8] * 65536.0) as i32;

    #[cfg(feature = "parallel")]
    let pixel_iter = pixels.par_iter_mut();
    #[cfg(not(feature = "parallel"))]
    let pixel_iter = pixels.iter_mut();

    pixel_iter.for_each(|pixel| {
        let p = *pixel;
        let a = p & 0xFF00_0000;
        let r = ((p >> 16) & 0xFF) as i32;
        let g = ((p >> 8) & 0xFF) as i32;
        let b = (p & 0xFF) as i32;

        let new_r = ((r * m00 + g * m01 + b * m02) >> 16).max(0).min(255) as u32;
        let new_g = ((r * m10 + g * m11 + b * m12) >> 16).max(0).min(255) as u32;
        let new_b = ((r * m20 + g * m21 + b * m22) >> 16).max(0).min(255) as u32;

        *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_color_blindness_achromatopsia() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        // Red pixel
        fb.set_pixel(0, 0, 0xFFFF0000);
        // Blue pixel
        fb.set_pixel(1, 0, 0xFF0000FF);

        apply_color_blindness(&mut fb, ColorBlindnessType::Achromatopsia);

        let p0 = fb.get_pixel(0, 0).unwrap();
        let p1 = fb.get_pixel(1, 0).unwrap();

        // R = G = B for achromatopsia
        assert_eq!((p0 >> 16) & 0xFF, (p0 >> 8) & 0xFF);
        assert_eq!((p0 >> 8) & 0xFF, p0 & 0xFF);

        assert_eq!((p1 >> 16) & 0xFF, (p1 >> 8) & 0xFF);
        assert_eq!((p1 >> 8) & 0xFF, p1 & 0xFF);
    }

    #[test]
    fn test_apply_color_blindness_protanopia() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        // Red pixel
        fb.set_pixel(0, 0, 0xFFFF0000);

        apply_color_blindness(&mut fb, ColorBlindnessType::Protanopia);

        let p = fb.get_pixel(0, 0).unwrap();
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;

        // For Protanopia, red and green should be severely shifted.
        // A pure red should map to a brownish/dark yellow hue.
        assert_ne!(p, 0xFFFF0000, "Protanopia should alter pure red.");
        assert!(r > 0 && g > 0, "Red should map to some mix of R and G.");
    }
}
