//! Sobel Edge Detection Filter
//!
//! A post-processing effect that applies the Sobel operator to detect edges in the framebuffer.

use abrash_core::framebuffer::Framebuffer;

/// Applies a Sobel edge detection filter to the framebuffer.
/// It converts the image to luminance and detects high frequency changes.
pub fn apply_sobel(fb: &mut Framebuffer) {
    let width = fb.width();
    let height = fb.height();
    if width < 3 || height < 3 {
        return;
    }

    // Allocate a scratch buffer to store the original image while we overwrite fb.
    let mut scratch = vec![0u32; (width * height) as usize];
    scratch.copy_from_slice(fb.as_slice());

    let out_pixels = fb.as_mut_slice();

    // Helper to get luminance
    let get_luma = |color: u32| -> i32 {
        let r = (color >> 16) & 0xFF;
        let g = (color >> 8) & 0xFF;
        let b = color & 0xFF;
        // Simple fast luminance approximation
        ((77 * r + 150 * g + 29 * b) >> 8) as i32
    };

    // Note: The outer edge of the image (1 pixel border) is left untouched or black.
    // For simplicity, we just zero them out in out_pixels.
    for y in 0..height {
        for x in 0..width {
            let out_idx = y * width + x;
            if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                out_pixels[out_idx as usize] = 0xFF00_0000;
                continue;
            }

            let idx = y * width + x;

            // 3x3 window around pixel
            let tl = get_luma(scratch[(idx - width - 1) as usize]);
            let tc = get_luma(scratch[(idx - width) as usize]);
            let tr = get_luma(scratch[(idx - width + 1) as usize]);
            let ml = get_luma(scratch[(idx - 1) as usize]);
            let mr = get_luma(scratch[(idx + 1) as usize]);
            let bl = get_luma(scratch[(idx + width - 1) as usize]);
            let bc = get_luma(scratch[(idx + width) as usize]);
            let br = get_luma(scratch[(idx + width + 1) as usize]);

            // Sobel X kernel
            let gx = -tl + tr - (2 * ml) + (2 * mr) - bl + br;

            // Sobel Y kernel
            let gy = tl + (2 * tc) + tr - bl - (2 * bc) - br;

            // Magnitude approximation (fast absolute sum instead of slow sqrt)
            let magnitude = gx.abs() + gy.abs();
            let clamped = magnitude.min(255) as u32;

            // Output as grayscale edge
            out_pixels[out_idx as usize] = 0xFF00_0000 | (clamped << 16) | (clamped << 8) | clamped;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sobel_edge_detection() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // Create a white square in the middle of a black image
        for y in 1..4 {
            for x in 1..4 {
                fb.set_pixel(x, y, 0xFF_FFFFFF);
            }
        }

        apply_sobel(&mut fb);

        // The center of the white square (2, 2) is surrounded by white, so the edge detection should be 0.
        let center = fb.get_pixel(2, 2).unwrap();
        assert_eq!(center & 0xFF_FFFF, 0, "Center of solid shape should have no edge");

        // The edge of the square at (1, 2) should detect an edge
        let left_edge = fb.get_pixel(1, 2).unwrap();
        assert!(left_edge & 0xFF_FFFF > 0, "Edge of shape should have an edge detected");

        // Outer bounds should be black
        let outer = fb.get_pixel(0, 0).unwrap();
        assert_eq!(outer, 0xFF00_0000, "Border pixels should be black");
    }
}
