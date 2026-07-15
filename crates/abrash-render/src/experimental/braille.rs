//! Braille Exporter
//!
//! Converts a `Framebuffer` into a String containing Unicode Braille characters.
//! Each Braille character represents a 2x4 pixel block.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the Braille exporter.
#[derive(Debug, Clone, Copy)]
pub struct BrailleConfig {
    /// The luminance threshold (0-255) to consider a pixel "on".
    /// Pixels with luminance >= threshold are drawn as raised dots.
    pub threshold: u8,
    /// Invert the threshold logic (pixels < threshold are drawn).
    pub invert: bool,
}

impl Default for BrailleConfig {
    fn default() -> Self {
        Self {
            threshold: 128,
            invert: false,
        }
    }
}

/// Converts a framebuffer into a Braille string.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn to_braille(fb: &Framebuffer, config: &BrailleConfig) -> String {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return String::new();
    }

    let cols = (width + 1) / 2;
    let rows = (height + 3) / 4;

    // Estimate capacity: cols chars per row + 1 newline per row.
    // Braille characters are 3 bytes each in UTF-8, plus 1 for newline.
    let mut output = String::with_capacity(rows * (cols * 3 + 1));

    let pixels = fb.as_slice();

    for ry in 0..rows {
        for cx in 0..cols {
            let mut char_val: u32 = 0x2800; // Base Braille block

            for dy in 0..4 {
                for dx in 0..2 {
                    let px = cx * 2 + dx;
                    let py = ry * 4 + dy;

                    if px < width && py < height {
                        let idx = py * width + px;
                        let color = pixels[idx];
                        let lum = pixel_luminance(color);

                        let is_on = if config.invert {
                            lum < config.threshold
                        } else {
                            lum >= config.threshold
                        };

                        if is_on {
                            // Map (dx, dy) to Braille dot pattern
                            let dot_val = match (dx, dy) {
                                (0, 0) => 0x01,
                                (0, 1) => 0x02,
                                (0, 2) => 0x04,
                                (0, 3) => 0x40,
                                (1, 0) => 0x08,
                                (1, 1) => 0x10,
                                (1, 2) => 0x20,
                                (1, 3) => 0x80,
                                _ => 0,
                            };
                            char_val |= dot_val;
                        }
                    }
                }
            }

            // Safely convert u32 to char. The range 0x2800..=0x28FF is valid Unicode.
            output.push(char::from_u32(char_val).unwrap());
        }

        if ry < rows - 1 {
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_empty() {
        let fb = Framebuffer::new(0, 0).unwrap();
        let s = to_braille(&fb, &BrailleConfig::default());
        assert_eq!(s, "");
    }

    #[test]
    fn test_braille_basic() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // Set all to black (luminance 0)
        fb.clear(0xFF_000000);

        // Turn on top-left (dx=0, dy=0 -> 0x01) and bottom-right (dx=1, dy=3 -> 0x80)
        fb.set_pixel(0, 0, 0xFF_FFFFFF);
        fb.set_pixel(1, 3, 0xFF_FFFFFF);

        let s = to_braille(&fb, &BrailleConfig::default());
        // 0x2800 | 0x01 | 0x80 = 0x2881
        assert_eq!(s, char::from_u32(0x2881).unwrap().to_string());
    }
}
