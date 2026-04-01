//! Halftone pattern effect.
//!
//! Simulates CMYK or grayscale halftone printing processes.

use abrash_core::framebuffer::Framebuffer;

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
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_halftone(fb: &mut Framebuffer, dot_size: f32, angle_radians: f32) {
    let width = fb.width() as usize;

    // We'll process each pixel independently.
    // To do this properly, we need to map screen coordinates to a rotated grid.

    let sin_a = angle_radians.sin();
    let cos_a = angle_radians.cos();
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

        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let p = *pixel;

            // Extract RGB
            let r = ((p >> 16) & 0xFF) as f32;
            let g = ((p >> 8) & 0xFF) as f32;
            let b = (p & 0xFF) as f32;

            // Calculate luminance (0.0 to 1.0)
            let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;

            // Map (x, y) to the rotated grid coordinates
            let x_f32 = x as f32;
            let rx = x_f32 * cos_a - y_sin_a;
            let ry = x_f32 * sin_a + y_cos_a;

            // Find the center of the nearest halftone cell in the rotated space
            let cx = (rx / dot_size).round() * dot_size;
            let cy = (ry / dot_size).round() * dot_size;

            // Calculate the distance from the pixel to the cell center
            let dist_sq = (rx - cx) * (rx - cx) + (ry - cy) * (ry - cy);

            // The radius of the dot we should draw (squared).
            // Lighter pixels (lum closer to 1.0) have smaller black dots.
            // Darker pixels (lum closer to 0.0) have larger black dots.
            let dot_radius_sq = (1.0 - lum) * max_dist_sq;

            // If the pixel is inside the dot radius, it's black. Otherwise, white.
            if dist_sq <= dot_radius_sq {
                *pixel = 0xFF00_0000; // Black
            } else {
                *pixel = 0xFFFF_FFFF; // White
            }
        }
    });
}
