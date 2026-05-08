//! Braille Display Filter.
//!
//! Converts a [`Framebuffer`] into a Braille art representation.
//! Each Unicode Braille character encodes a 2x4 grid of pixels,
//! dramatically increasing the perceived resolution in text environments.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::fmt::Write;

/// Converts a Framebuffer into a string of Braille characters.
///
/// Pixels with luminance >= `threshold` are considered "set" (dot raised).
///
/// # Arguments
///
/// * `fb` - The framebuffer to convert.
/// * `threshold` - Luminance threshold (0-255). Pixels >= this value are rendered.
///
/// # Returns
/// A `String` containing the Braille representation, with newlines separating rows.
#[must_use]
pub fn to_braille_string(fb: &Framebuffer, threshold: u8) -> String {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return String::new();
    }

    // A braille character covers 2 columns and 4 rows.
    let braille_cols = (width + 1) / 2;
    let braille_rows = (height + 3) / 4;

    // Pre-allocate: braille_cols characters + 1 newline per braille row
    // Each braille char is 3 bytes in UTF-8
    let mut result = String::with_capacity(braille_rows * (braille_cols * 3 + 1));

    let pixels = fb.as_slice();

    for br_y in 0..braille_rows {
        for br_x in 0..braille_cols {
            let mut dot_pattern = 0u8;

            let fb_x_base = br_x * 2;
            let fb_y_base = br_y * 4;

            // Braille dot mapping based on Unicode U+2800..U+28FF
            // Array of (dx, dy, bit_flag)
            let dots = [
                (0, 0, 0x01), // Dot 1 (top-left)
                (0, 1, 0x02), // Dot 2 (mid-top-left)
                (0, 2, 0x04), // Dot 3 (mid-bottom-left)
                (1, 0, 0x08), // Dot 4 (top-right)
                (1, 1, 0x10), // Dot 5 (mid-top-right)
                (1, 2, 0x20), // Dot 6 (mid-bottom-right)
                (0, 3, 0x40), // Dot 7 (bottom-left)
                (1, 3, 0x80), // Dot 8 (bottom-right)
            ];

            for (dx, dy, flag) in dots {
                let px = fb_x_base + dx;
                let py = fb_y_base + dy;

                if px < width && py < height {
                    let pixel_idx = py * width + px;
                    let pixel = pixels[pixel_idx];
                    let lum = pixel_luminance(pixel);
                    if lum >= threshold {
                        dot_pattern |= flag;
                    }
                }
            }

            // Braille characters start at U+2800
            let braille_char = std::char::from_u32(0x2800 + u32::from(dot_pattern)).unwrap_or(' ');
            result.push(braille_char);
        }
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_empty() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF00_0000); // Black

        let s = to_braille_string(&fb, 128);
        // All pixels are 0 luminance < 128 -> empty braille (U+2800)
        assert_eq!(s, "⠀\n"); // That character is U+2800 (Braille pattern blank)
    }

    #[test]
    fn test_braille_full() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFFFF_FFFF); // White

        let s = to_braille_string(&fb, 128);
        // All pixels are 255 luminance >= 128 -> full braille (U+28FF)
        assert_eq!(s, "⣿\n"); // U+28FF
    }

    #[test]
    fn test_braille_pattern() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF00_0000);

        // Set top-left (dot 1) and bottom-right (dot 8)
        fb.set_pixel(0, 0, 0xFFFF_FFFF); // Dot 1 = 0x01
        fb.set_pixel(1, 3, 0xFFFF_FFFF); // Dot 8 = 0x80

        let s = to_braille_string(&fb, 128);
        // 0x2800 + 0x01 + 0x80 = 0x2881
        assert_eq!(s, "⢁\n"); // U+2881
    }

    #[test]
    fn test_braille_odd_size() {
        let mut fb = Framebuffer::new(3, 5).unwrap();
        fb.clear(0xFFFF_FFFF); // White

        let s = to_braille_string(&fb, 128);
        // Size 3x5 should take 2x2 braille characters
        // The first row of braille chars has 1 full char and 1 half char (right half missing, dot 4, 5, 6, 8 missing)
        // Actually for the 2nd char (cols 2,3), column 3 is out of bounds, so dots 4(0x08), 5(0x10), 6(0x20), 8(0x80) are unset.
        // Dot 1(0x01), 2(0x02), 3(0x04), 7(0x40) are set. 0x01|0x02|0x04|0x40 = 0x47. U+2847 is ⡇

        // Second braille row (y=4,5), row 5 is out of bounds.
        // For the 1st char, dots 7(0x40) and 8(0x80) are out of bounds.
        // Wait, y=4 is row 5. The braille char covers y=4..7. So only y=4 is in bounds.
        // Dots in bounds: 1(0x01), 4(0x08). 0x01|0x08 = 0x09. U+2809 is ⠉
        // For the 2nd char in row 2, x=2..3, y=4..7. In bounds: x=2, y=4.
        // That is dot 1(0x01). U+2801 is ⠁

        let expected = "⣿⡇\n⠉⠁\n";
        assert_eq!(s, expected);
    }
}
