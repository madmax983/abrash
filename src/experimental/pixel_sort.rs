//! Pixel Sorting Effect
//!
//! A popular "glitch art" effect that sorts pixels along rows or columns
//! based on a metric like luminance, creating a "melting" or "tearing" aesthetic.

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

/// Applies a pixel sort effect to the framebuffer.
///
/// Iterates over rows (or columns) and identifies contiguous segments of pixels
/// whose luminance exceeds `threshold`. These segments are then sorted in-place
/// based on their luminance.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `threshold` - The luminance threshold (0.0 to 1.0) above which pixels are sorted.
/// * `vertical` - If true, pixels are sorted vertically (columns). If false, horizontally (rows).
/// * `reverse` - If true, sort in descending order (brightest first). Otherwise, ascending.
/// Configuration for the Pixel Sort effect.
#[derive(Clone, Copy, Debug)]
pub struct PixelSortConfig {
    /// Luminance threshold (0.0 to 1.0) below which pixels are sorted.
    pub threshold: f32,
    /// Sort vertically instead of horizontally.
    pub vertical: bool,
    /// Reverse the sort order (bright to dark).
    pub reverse: bool,
}

impl Default for PixelSortConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            vertical: false,
            reverse: false,
        }
    }
}

/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate

/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_pixel_sort(fb: &mut Framebuffer, config: &PixelSortConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Convert 0.0-1.0 threshold to 0-255 luminance
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let lum_threshold = (config.threshold.clamp(0.0, 1.0) * 255.0) as u8;

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;

        if config.vertical {
            // Vertical sorting
            // We need to extract columns, sort them, and put them back.
            // Using Rayon, we can process columns in parallel. We wrap the raw pointer
            // to bypass the borrow checker since columns represent disjoint memory locations.
            #[derive(Clone, Copy)]
            struct SendPtr(*mut u32);
            unsafe impl Send for SendPtr {}
            unsafe impl Sync for SendPtr {}

            let pixels_ptr = SendPtr(pixels.as_mut_ptr());

            // ⚡ Bolt: Eliminate per-thread dynamic heap allocation in par_iter by using a thread_local buffer.
            std::thread_local! {
                static PIXEL_SORT_COL_BUFFER: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
            }

            (0..width).into_par_iter().for_each(|x| {
                PIXEL_SORT_COL_BUFFER.with(|buffer| {
                    let mut col_buffer = buffer.take();
                    if col_buffer.len() < height {
                        col_buffer.resize(height, 0);
                    }

                    let col_slice = &mut col_buffer[..height];

                    // Accessing the SendPtr instead of the raw pointer allows it to cross the boundary
                    // and then we extract the inner raw pointer.
                    let ptr = pixels_ptr;

                    // Extract column
                    for (y, item) in col_slice.iter_mut().enumerate() {
                        unsafe {
                            *item = *ptr.0.add(y * width + x);
                        }
                    }

                    // Sort segments in column
                    sort_segments(col_slice, lum_threshold, config.reverse);

                    // Put column back
                    for (y, item) in col_slice.iter().enumerate() {
                        unsafe {
                            *ptr.0.add(y * width + x) = *item;
                        }
                    }
                });
            });
        } else {
            // Horizontal sorting
            // We can operate directly on contiguous chunks (rows)
            pixels.par_chunks_exact_mut(width).for_each(|row| {
                sort_segments(row, lum_threshold, config.reverse);
            });
        }
    }

    #[cfg(not(feature = "parallel"))]
    {
        if config.vertical {
            // Vertical sorting
            // We need to extract columns, sort them, and put them back.
            // Doing this in-place with strided access is tricky in Rust,
            // so we'll use a temporary buffer for each column.
            let mut col_buffer = vec![0u32; height];

            for x in 0..width {
                // Extract column
                for y in 0..height {
                    col_buffer[y] = pixels[y * width + x];
                }

                // Sort segments in column
                sort_segments(&mut col_buffer, lum_threshold, config.reverse);

                // Put column back
                for y in 0..height {
                    pixels[y * width + x] = col_buffer[y];
                }
            }
        } else {
            // Horizontal sorting
            // We can operate directly on contiguous chunks (rows)
            pixels.chunks_exact_mut(width).for_each(|row| {
                sort_segments(row, lum_threshold, config.reverse);
            });
        }
    }
}

/// Sorts segments of pixels in a 1D slice based on luminance.
fn sort_segments(slice: &mut [u32], threshold: u8, reverse: bool) {
    let mut start = 0;
    let len = slice.len();

    while start < len {
        // Find start of a sortable segment
        while start < len && pixel_luminance(slice[start]) < threshold {
            start += 1;
        }

        if start >= len {
            break;
        }

        // Find end of the sortable segment
        let mut end = start + 1;
        while end < len && pixel_luminance(slice[end]) >= threshold {
            end += 1;
        }

        // Sort the segment
        let segment = &mut slice[start..end];
        if reverse {
            segment.sort_unstable_by_key(|&p| std::cmp::Reverse(pixel_luminance(p)));
        } else {
            segment.sort_unstable_by_key(|&p| pixel_luminance(p));
        }

        start = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pixel_sort_horizontal() {
        let mut fb = Framebuffer::new(5, 1).unwrap();
        // Setup: Dark, Bright (Middle), Bright (Low), Dark, Bright (High)
        // Values are encoded as grayscale (R=G=B=Lum).
        let dark = 0xFF000000;
        let b_mid = 0xFF808080; // Lum: ~128
        let b_low = 0xFF404040; // Lum: ~64
        let b_high = 0xFFC0C0C0; // Lum: ~192

        fb.set_pixel(0, 0, dark);
        fb.set_pixel(1, 0, b_mid);
        fb.set_pixel(2, 0, b_low);
        fb.set_pixel(3, 0, dark);
        fb.set_pixel(4, 0, b_high);

        // Threshold = 0.2 (Lum ~51). So b_low, b_mid, b_high are all above threshold.
        // Segments: [b_mid, b_low] and [b_high].
        let config = PixelSortConfig {
            threshold: 0.2,
            vertical: false,
            reverse: false,
        };
        apply_pixel_sort(&mut fb, &config);

        // Expected sorting (ascending luminance):
        // First segment: [b_mid, b_low] sorts to [b_low, b_mid].
        // Second segment: [b_high] remains [b_high].
        assert_eq!(fb.get_pixel(0, 0).unwrap(), dark);
        assert_eq!(fb.get_pixel(1, 0).unwrap(), b_low);
        assert_eq!(fb.get_pixel(2, 0).unwrap(), b_mid);
        assert_eq!(fb.get_pixel(3, 0).unwrap(), dark);
        assert_eq!(fb.get_pixel(4, 0).unwrap(), b_high);
    }

    #[test]
    fn test_apply_pixel_sort_vertical() {
        let mut fb = Framebuffer::new(1, 5).unwrap();
        let dark = 0xFF000000;
        let b_mid = 0xFF808080;
        let b_low = 0xFF404040;
        let b_high = 0xFFC0C0C0;

        fb.set_pixel(0, 0, dark);
        fb.set_pixel(0, 1, b_mid);
        fb.set_pixel(0, 2, b_low);
        fb.set_pixel(0, 3, dark);
        fb.set_pixel(0, 4, b_high);

        // Vertical sort, ascending
        let config = PixelSortConfig {
            threshold: 0.2,
            vertical: true,
            reverse: false,
        };
        apply_pixel_sort(&mut fb, &config);

        assert_eq!(fb.get_pixel(0, 0).unwrap(), dark);
        assert_eq!(fb.get_pixel(0, 1).unwrap(), b_low);
        assert_eq!(fb.get_pixel(0, 2).unwrap(), b_mid);
        assert_eq!(fb.get_pixel(0, 3).unwrap(), dark);
        assert_eq!(fb.get_pixel(0, 4).unwrap(), b_high);
    }

    #[test]
    fn test_apply_pixel_sort_reverse() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        let b_low = 0xFF404040;
        let b_mid = 0xFF808080;
        let b_high = 0xFFC0C0C0;

        fb.set_pixel(0, 0, b_low);
        fb.set_pixel(1, 0, b_mid);
        fb.set_pixel(2, 0, b_high);

        // Horizontal sort, reverse (descending)
        let config = PixelSortConfig {
            threshold: 0.1,
            vertical: false,
            reverse: true,
        };
        apply_pixel_sort(&mut fb, &config);

        // All are above threshold. Whole row is sorted descending.
        assert_eq!(fb.get_pixel(0, 0).unwrap(), b_high);
        assert_eq!(fb.get_pixel(1, 0).unwrap(), b_mid);
        assert_eq!(fb.get_pixel(2, 0).unwrap(), b_low);
    }
}
