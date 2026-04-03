//! Halftone pattern effect.
//!
//! Simulates CMYK or grayscale halftone printing processes.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Halftone stylization filter to the framebuffer.
///
/// This filter converts the image into a pattern of black dots on a white background,
/// similar to old newspaper printing techniques. The size of the dots depends on the
/// luminance of the underlying pixels.
///
/// * `fb`: The Framebuffer to modify.
/// * `dot_size`: The maximum radius of the halftone dots (e.g., 5.0).
/// * `angle_radians`: The rotation angle of the dot grid (e.g., 45 degrees or PI/4).
pub fn apply_halftone(fb: &mut Framebuffer, dot_size: f32, angle_radians: f32) {
    let width = fb.width() as usize;

    // We'll process each pixel independently.
    // To do this properly, we need to map screen coordinates to a rotated grid.

    let (sin_a, cos_a) = angle_radians.sin_cos();
    let max_dist_sq = (dot_size * dot_size) / 2.0;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        let y_sin_a = y_f32 * sin_a;
        let y_cos_a = y_f32 * cos_a;

        let mut rx = -y_sin_a;
        let mut ry = y_cos_a;

        let inv_dot_size = 1.0 / dot_size;

        for pixel in row.iter_mut() {
            let p = *pixel;

            // Fast integer luminance (0 to 255)
            // L = 0.299*R + 0.587*G + 0.114*B
            // Approximate with (2 * R + 5 * G + 1 * B) / 8 or similar fast int math
            // Actually, keeping standard Rec.601 via integer: (77 * r + 150 * g + 29 * b) >> 8
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            let lum_i = (77 * r + 150 * g + 29 * b) >> 8;
            let lum = lum_i as f32 * (1.0 / 255.0);

            // Find the center of the nearest halftone cell in the rotated space
            let cx = (rx * inv_dot_size).round() * dot_size;
            let cy = (ry * inv_dot_size).round() * dot_size;

            let dx = rx - cx;
            let dy = ry - cy;

            // Calculate the distance from the pixel to the cell center
            let dist_sq = dx * dx + dy * dy;

            // The radius of the dot we should draw (squared).
            let dot_radius_sq = (1.0 - lum) * max_dist_sq;

            // If the pixel is inside the dot radius, it's black. Otherwise, white.
            *pixel = if dist_sq < dot_radius_sq {
                0xFF00_0000
            } else {
                0xFFFF_FFFF
            };

            rx += cos_a;
            ry += sin_a;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_halftone_black_image() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Clear with black
        fb.clear(0xFF_00_00_00);

        apply_halftone(&mut fb, 5.0, 0.0);

        // A black image has luminance 0, so dot radius is maximum.
        // The nearest cell center will be at (0,0) for the top-left pixels.
        // It should mostly become black, maybe some white around edges of dots
        // but let's just ensure it doesn't crash and alters the buffer.
        let has_black = fb.as_slice().iter().any(|&p| p == 0xFF00_0000);
        assert!(has_black);
    }

    #[test]
    fn test_apply_halftone_white_image() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Clear with white
        fb.clear(0xFF_FF_FF_FF);

        apply_halftone(&mut fb, 5.0, 0.0);

        // A white image has luminance 1, so dot radius is 0.
        // Every pixel should become white because distance is always >= 0.
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFFFF_FFFF);
        }
    }
}
