//! Braille Exporter
//!
//! A module for converting a framebuffer into high-density Unicode Braille characters
//! for terminal output, maximizing apparent resolution by mapping 2x4 pixel blocks
//! to single characters.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the Braille Exporter.
#[derive(Debug, Clone, Copy)]
pub struct BrailleExportConfig {
    /// Luminance threshold (0-255). Pixels with luminance >= this are considered "on".
    pub threshold: u8,
    /// If true, inverted logic (pixels < threshold are "on").
    pub invert: bool,
}

impl Default for BrailleExportConfig {
    fn default() -> Self {
        Self {
            threshold: 128,
            invert: false,
        }
    }
}

/// Exports the given framebuffer to a multi-line Braille string.
///
/// Maps each 2x4 block of pixels to a corresponding Unicode Braille character (U+2800 to U+28FF).
///
/// # Panics
///
/// Panics if the generated unicode character is invalid (which should never happen
/// since the offset is strictly bounded to valid Braille blocks U+2800 to U+28FF).
#[must_use]
pub fn export_to_braille(fb: &Framebuffer, config: &BrailleExportConfig) -> String {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return String::new();
    }

    let braille_width = (width + 1) / 2;
    let braille_height = (height + 3) / 4;

    // Each Braille character is 3 bytes in UTF-8, plus newlines
    let mut out = String::with_capacity(braille_height * (braille_width * 3 + 1));

    let pixels = fb.as_slice();

    for by in 0..braille_height {
        for bx in 0..braille_width {
            let mut offset = 0u32;

            for dy in 0..4 {
                for dx in 0..2 {
                    let px = bx * 2 + dx;
                    let py = by * 4 + dy;

                    if px < width && py < height {
                        let pixel = pixels[py * width + px];
                        let lum = pixel_luminance(pixel);

                        let is_on = if config.invert {
                            lum < config.threshold
                        } else {
                            lum >= config.threshold
                        };

                        if is_on {
                            // Braille dot mapping:
                            // 0 3
                            // 1 4
                            // 2 5
                            // 6 7
                            let dot_idx = match (dx, dy) {
                                (0, 0) => 0,
                                (0, 1) => 1,
                                (0, 2) => 2,
                                (1, 0) => 3,
                                (1, 1) => 4,
                                (1, 2) => 5,
                                (0, 3) => 6,
                                (1, 3) => 7,
                                _ => unreachable!(),
                            };
                            offset |= 1 << dot_idx;
                        }
                    }
                }
            }

            // Base Unicode character for Braille pattern is U+2800
            let c = std::char::from_u32(0x2800 + offset).unwrap();
            out.push(c);
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_export() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // Clear to black
        fb.clear(0xFF_000000);
        // Set top-left and bottom-right pixels to white
        fb.set_pixel(0, 0, 0xFF_FFFFFF); // dot 1 (0x01)
        fb.set_pixel(1, 3, 0xFF_FFFFFF); // dot 8 (0x80)

        let config = BrailleExportConfig {
            threshold: 128,
            invert: false,
        };

        let out = export_to_braille(&fb, &config);

        // 0x2800 + 0x01 + 0x80 = 0x2881 (⢁)
        // Note: the string ends with \n
        assert_eq!(out, "⢁\n");
    }
}
