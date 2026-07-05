//! Braille Art Converter
//!
//! Converts a [`Framebuffer`] into Braille unicode characters.
//! Each Braille character can represent a 2x4 pixel grid,
//! providing higher resolution for terminal outputs than standard ASCII art.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Converts a Framebuffer to a Braille string.
pub struct BrailleConverter<'a> {
    framebuffer: &'a Framebuffer,
    threshold: u8,
}

impl<'a> BrailleConverter<'a> {
    /// Creates a new `BrailleConverter`.
    /// `threshold` determines the luminance cutoff (0-255) for a pixel to be "on".
    #[must_use]
    pub const fn new(framebuffer: &'a Framebuffer, threshold: u8) -> Self {
        Self {
            framebuffer,
            threshold,
        }
    }

    /// Converts the framebuffer to a multiline Braille string.
    #[must_use]
    pub fn to_string(&self) -> String {
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;

        // A braille character is 2x4 pixels.
        let braille_width = (width + 1) / 2;
        let braille_height = (height + 3) / 4;

        let mut out = String::with_capacity(braille_height * (braille_width + 1));

        for by in 0..braille_height {
            for bx in 0..braille_width {
                let mut char_code = 0x2800;

                // Braille dot layout relative to the character cell:
                // Dot 1: 0,0  -> bit 0 (0x01)
                // Dot 2: 0,1  -> bit 1 (0x02)
                // Dot 3: 0,2  -> bit 2 (0x04)
                // Dot 4: 1,0  -> bit 3 (0x08)
                // Dot 5: 1,1  -> bit 4 (0x10)
                // Dot 6: 1,2  -> bit 5 (0x20)
                // Dot 7: 0,3  -> bit 6 (0x40)
                // Dot 8: 1,3  -> bit 7 (0x80)

                let offsets = [
                    (0, 0, 0x01),
                    (0, 1, 0x02),
                    (0, 2, 0x04),
                    (1, 0, 0x08),
                    (1, 1, 0x10),
                    (1, 2, 0x20),
                    (0, 3, 0x40),
                    (1, 3, 0x80),
                ];

                for &(dx, dy, bit) in &offsets {
                    let px = bx * 2 + dx;
                    let py = by * 4 + dy;

                    if px < width && py < height {
                        if let Some(pixel) = self.framebuffer.get_pixel(px as i32, py as i32) {
                            if pixel_luminance(pixel) >= self.threshold {
                                char_code |= bit;
                            }
                        }
                    }
                }

                out.push(std::char::from_u32(char_code).unwrap_or(' '));
            }
            out.push('\n');
        }

        out
    }
}

impl std::fmt::Display for BrailleConverter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_conversion_empty() {
        let fb = Framebuffer::new(4, 4).unwrap();
        let converter = BrailleConverter::new(&fb, 128);
        let result = converter.to_string();
        // An empty 4x4 should be 2x1 braille characters of empty dots (0x2800 = '⠀')
        assert_eq!(result, "⠀⠀\n");
    }

    #[test]
    fn test_braille_conversion_dots() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // Turn on top-left pixel
        fb.set_pixel(0, 0, 0xFFFFFFFF);
        // Turn on bottom-right pixel
        fb.set_pixel(1, 3, 0xFFFFFFFF);

        let converter = BrailleConverter::new(&fb, 128);
        let result = converter.to_string();

        // Top-left is dot 1 (0x01)
        // Bottom-right is dot 8 (0x80)
        // 0x2800 | 0x01 | 0x80 = 0x2881 ('⢁')
        assert_eq!(result, "⢁\n");
    }
}
