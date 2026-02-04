//! Experimental depth-based outline detection.
//!
//! This module implements a post-processing effect that generates outlines
//! by detecting discontinuities in the depth buffer.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Applies an outline effect based on depth discontinuities.
///
/// This function detects edges in the depth buffer and draws them into the framebuffer.
/// It uses a simple gradient magnitude check: `|d(x,y) - d(x+1,y)| + |d(x,y) - d(x,y+1)|`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify (draw outlines onto).
/// * `zb` - The z-buffer to read depth values from.
/// * `color` - The color of the outline (e.g., 0xFF000000 for black).
/// * `threshold` - The depth difference threshold to trigger an edge.
///   Lower values = more sensitive.
pub fn apply_outline(fb: &mut Framebuffer, zb: &ZBuffer, color: u32, threshold: f32) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let width = fb.width();
    let height = fb.height();
    let depths = zb.as_slice();
    let pixels = fb.as_mut_slice();

    // Iterate up to width-1 and height-1 to avoid boundary checks for neighbors
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let idx = (y * width + x) as usize;
            let idx_right = idx + 1;
            let idx_down = idx + width as usize;

            let d_center = depths[idx];
            let d_right = depths[idx_right];
            let d_down = depths[idx_down];

            // Check horizontal difference
            let diff_x = (d_center - d_right).abs();
            // Check vertical difference
            let diff_y = (d_center - d_down).abs();

            // Note: If both are INFINITY, result is NaN. NaN > threshold is false.
            // If one is INFINITY, result is INFINITY. INFINITY > threshold is true.
            if diff_x > threshold || diff_y > threshold {
                pixels[idx] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn test_outline_detection() {
        let w = 4;
        let h = 4;
        let mut fb = Framebuffer::new(w, h).unwrap();
        let mut zb = ZBuffer::new(w, h).unwrap();

        // Clear FB to white
        fb.clear(0xFFFFFFFF);

        // Set up ZBuffer: Left half is near (1.0), Right half is far (10.0)
        // x=0,1 -> 1.0
        // x=2,3 -> 10.0
        for y in 0..h {
            zb.test_and_set(0, y as i32, 1.0);
            zb.test_and_set(1, y as i32, 1.0);
            zb.test_and_set(2, y as i32, 10.0);
            zb.test_and_set(3, y as i32, 10.0);
        }

        let outline_color = 0xFF000000; // Black
        apply_outline(&mut fb, &zb, outline_color, 5.0);

        // We expect an edge at x=1 (because x=1 is 1.0, x=2 is 10.0, diff=9.0 > 5.0)
        // apply_outline iterates 0..w-1.
        // At x=1: d_center=1.0, d_right=10.0. Diff=9.0. Outline drawn at x=1.

        let p0 = fb.get_pixel(0, 0).unwrap();
        let p1 = fb.get_pixel(1, 0).unwrap();
        let p2 = fb.get_pixel(2, 0).unwrap();

        assert_ne!(
            p0, outline_color,
            "x=0 should not be outlined (diff x=0 (1-1), y=0)"
        );
        assert_eq!(
            p1, outline_color,
            "x=1 should be outlined (diff x=9 (1-10))"
        );
        assert_ne!(
            p2, outline_color,
            "x=2 should not be outlined (diff x=0 (10-10))"
        );
    }
}
