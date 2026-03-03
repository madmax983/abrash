//! Kaleidoscope effect.
//!
//! A post-processing effect that creates a symmetrical kaleidoscope pattern
//! by mapping pixels to polar coordinates and folding the angle.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a kaleidoscope effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `segments` - The number of kaleidoscope segments (e.g., 6 for a hexagon pattern).
/// * `rotation` - The rotation of the pattern in radians.
pub fn apply_kaleidoscope(fb: &mut Framebuffer, segments: u32, rotation: f32) {
    if segments <= 1 || fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Copy the original framebuffer to read from safely
    let source = pixels.to_vec();

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let segment_angle = std::f32::consts::TAU / (segments as f32);
    let half_segment = segment_angle / 2.0;

    #[cfg(feature = "parallel")]
    let chunk_iter = pixels.par_chunks_mut(width);
    #[cfg(not(feature = "parallel"))]
    let chunk_iter = pixels.chunks_mut(width);

    chunk_iter.enumerate().for_each(|(y, row)| {
        let dy = y as f32 - cy;
        for (x, item) in row.iter_mut().enumerate().take(width) {
            let dx = x as f32 - cx;
            let distance = (dx * dx + dy * dy).sqrt();
            let mut angle = dy.atan2(dx);

            // Apply rotation and modulo
            angle -= rotation;

            // Normalize angle to [0, segment_angle]
            let mut mod_angle = angle % segment_angle;
            if mod_angle < 0.0 {
                mod_angle += segment_angle;
            }

            // Mirroring for symmetry
            let mirrored_angle = (mod_angle - half_segment).abs();
            let final_angle = mirrored_angle + rotation;

            let src_x = (cx + distance * final_angle.cos()).round() as i32;
            let src_y = (cy + distance * final_angle.sin()).round() as i32;

            // Clamp to avoid out of bounds
            let src_x = src_x.clamp(0, (width - 1) as i32) as usize;
            let src_y = src_y.clamp(0, (height - 1) as i32) as usize;

            *item = source[src_y * width + src_x];
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_kaleidoscope() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF000000); // Black

        // Draw a white dot at (8, 5) which is right of center (5, 5)
        fb.set_pixel(8, 5, 0xFFFFFFFF);

        // Apply kaleidoscope with 4 segments
        apply_kaleidoscope(&mut fb, 4, 0.0);

        // The white dot should now be mirrored into multiple quadrants.
        // It was at distance 3, angle 0 (relative to 5, 5).
        // Let's check if the buffer is modified beyond the original single pixel.
        let mut white_pixels = 0;
        for y in 0..10 {
            for x in 0..10 {
                if fb.get_pixel(x, y).unwrap() == 0xFFFFFFFF {
                    white_pixels += 1;
                }
            }
        }

        assert!(
            white_pixels > 1,
            "Kaleidoscope should copy pixels to multiple segments, got {white_pixels}"
        );
    }
}
