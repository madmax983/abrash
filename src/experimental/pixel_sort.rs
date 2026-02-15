//! Pixel Sorting Post-Process Effect
//!
//! Applies a "pixel sort" glitch effect to the framebuffer.
//!
//! The algorithm iterates over each row of pixels and sorts contiguous segments
//! of pixels that exceed a luminance threshold. This creates a "smearing" or "melting" look.

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

/// Applies pixel sorting to the framebuffer.
///
/// For each row, pixels with luminance greater than `threshold` are grouped into spans
/// and sorted by their luminance. Darker pixels (below threshold) remain fixed and act as barriers.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `threshold` - Luminance threshold (0-255). Pixels brighter than this will be sorted.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::experimental::pixel_sort::apply_pixel_sort;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// // ... draw something ...
/// apply_pixel_sort(&mut fb, 100);
/// ```
pub fn apply_pixel_sort(fb: &mut Framebuffer, threshold: u8) {
    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    // Iterate over rows directly as the Framebuffer is row-major.
    for row in pixels.chunks_mut(width) {
        let mut i = 0;
        while i < width {
            // Find start of a segment (luminance > threshold)
            if pixel_luminance(row[i]) <= threshold {
                i += 1;
                continue;
            }

            let start = i;
            i += 1;

            // Find end of the segment
            while i < width && pixel_luminance(row[i]) > threshold {
                i += 1;
            }
            let end = i;

            // Sort the segment [start..end]
            // We use sort_by_key on luminance.
            // Note: sort_by_key is stable, so identical luminance pixels keep relative order.
            row[start..end].sort_by_key(|&p| pixel_luminance(p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_sort() {
        let width = 5;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Row: [Dark, Bright2, Bright1, Dark, Bright3]
        // Luminance approx:
        // Dark: 0
        // Bright2: 200
        // Bright1: 100
        // Bright3: 150

        // pixel_luminance uses: (77*R + 150*G + 29*B) >> 8
        // If R=G=B=V, lum = (256*V) >> 8 = V.

        let val_dark = 10;
        let val_bright1 = 100;
        let val_bright2 = 200;
        let val_bright3 = 150;

        let c_dark = 0xFF000000 | (val_dark << 16) | (val_dark << 8) | val_dark;
        let c_bright1 = 0xFF000000 | (val_bright1 << 16) | (val_bright1 << 8) | val_bright1;
        let c_bright2 = 0xFF000000 | (val_bright2 << 16) | (val_bright2 << 8) | val_bright2;
        let c_bright3 = 0xFF000000 | (val_bright3 << 16) | (val_bright3 << 8) | val_bright3;

        // Setup: [Dark, Bright2, Bright1, Dark, Bright3]
        // Indices: 0, 1, 2, 3, 4
        fb.set_pixel(0, 0, c_dark);
        fb.set_pixel(1, 0, c_bright2);
        fb.set_pixel(2, 0, c_bright1);
        fb.set_pixel(3, 0, c_dark);
        fb.set_pixel(4, 0, c_bright3);

        // Threshold = 50.
        // Segment 1: Indices 1, 2 (Bright2, Bright1) -> Both > 50.
        // Sorted: [Bright1, Bright2] (100, 200)
        // Segment 2: Index 4 (Bright3) -> > 50. Sorted: [Bright3] (no change)

        apply_pixel_sort(&mut fb, 50);

        // Expected: [Dark, Bright1, Bright2, Dark, Bright3]
        assert_eq!(fb.get_pixel(0, 0).unwrap(), c_dark, "Index 0 should be Dark");
        assert_eq!(
            fb.get_pixel(1, 0).unwrap(),
            c_bright1,
            "Index 1 should be Bright1 (100)"
        );
        assert_eq!(
            fb.get_pixel(2, 0).unwrap(),
            c_bright2,
            "Index 2 should be Bright2 (200)"
        );
        assert_eq!(fb.get_pixel(3, 0).unwrap(), c_dark, "Index 3 should be Dark");
        assert_eq!(
            fb.get_pixel(4, 0).unwrap(),
            c_bright3,
            "Index 4 should be Bright3 (150)"
        );
    }
}
