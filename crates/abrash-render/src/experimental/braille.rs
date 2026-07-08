//! High-Resolution Braille Output Converter.
//!
//! Converts a [`Framebuffer`] into high-resolution Unicode Braille art.
//! Braille patterns map a 2x4 pixel grid into a single character, providing
//! a 4x denser resolution compared to standard ASCII output.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// A threshold value used to determine if a pixel is "on" or "off".
const LUMINANCE_THRESHOLD: u8 = 128;

pub struct BrailleConverter<'a> {
    framebuffer: &'a Framebuffer,
}

impl<'a> BrailleConverter<'a> {
    /// Creates a new Braille converter for the given framebuffer.
    #[must_use]
    pub const fn new(framebuffer: &'a Framebuffer) -> Self {
        Self { framebuffer }
    }

    /// Converts the framebuffer to a String using Unicode Braille characters.
    #[must_use]
    pub fn to_string(&self) -> String {
        let fb_width = self.framebuffer.width() as usize;
        let fb_height = self.framebuffer.height() as usize;

        let cols = fb_width / 2;
        let rows = fb_height / 4;

        let mut result = String::with_capacity((cols + 1) * rows);

        for r in 0..rows {
            for c in 0..cols {
                let start_x = c * 2;
                let start_y = r * 4;

                let mut braille_char_offset: u32 = 0;

                // Unicode Braille Patterns map bits to a 2x4 grid:
                // 1 4
                // 2 5
                // 3 6
                // 7 8

                let bit_mapping = [
                    (0, 0, 0),
                    (0, 1, 1),
                    (0, 2, 2),
                    (0, 3, 6),
                    (1, 0, 3),
                    (1, 1, 4),
                    (1, 2, 5),
                    (1, 3, 7),
                ];

                for &(dx, dy, bit_shift) in &bit_mapping {
                    let px = start_x + dx;
                    let py = start_y + dy;

                    if let Some(pixel) = self.framebuffer.get_pixel(px as i32, py as i32) {
                        if pixel_luminance(pixel) >= LUMINANCE_THRESHOLD {
                            braille_char_offset |= 1 << bit_shift;
                        }
                    }
                }

                // Base braille character is U+2800
                let braille_char = char::from_u32(0x2800 + braille_char_offset).unwrap_or(' ');
                result.push(braille_char);
            }
            result.push('\n');
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_mapping() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF_000000); // Black

        // Turn on top-left (bit 0) and bottom-right (bit 7)
        fb.set_pixel(0, 0, 0xFF_FFFFFF);
        fb.set_pixel(1, 3, 0xFF_FFFFFF);

        let converter = BrailleConverter::new(&fb);
        let s = converter.to_string();

        // 1 << 0 (1) + 1 << 7 (128) = 129
        // 0x2800 + 129 = 0x2881 => ⢁
        assert_eq!(s, "⢁\n");
    }
}
