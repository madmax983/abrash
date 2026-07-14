//! Braille Art Exporter
//!
//! A module to convert framebuffers into high-resolution Braille text art.
//! Each Braille character represents a 2x4 grid of pixels, allowing for
//! higher perceived resolution in text-based environments compared to
//! standard ASCII art.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::fmt::{self, Write};
use std::fs::File;
use std::io::{self, BufWriter, Write as IoWrite};
use std::path::Path;

/// Configuration for the Braille export.
#[derive(Debug, Clone)]
pub struct BrailleConfig {
    /// The luminance threshold (0-255) above which a pixel is considered "on".
    pub threshold: u8,
    /// Whether to invert the thresholding (e.g. black pixels become dots).
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

/// Converts a Framebuffer into a Braille text representation.
pub struct BrailleConverter<'a> {
    framebuffer: &'a Framebuffer,
    config: BrailleConfig,
}

impl<'a> BrailleConverter<'a> {
    /// Creates a new `BrailleConverter`.
    #[must_use]
    pub const fn new(framebuffer: &'a Framebuffer, config: BrailleConfig) -> Self {
        Self {
            framebuffer,
            config,
        }
    }

    /// Converts a 2x4 grid of pixels at the given coordinates to a Braille character.
    fn cell_to_braille(&self, start_x: u32, start_y: u32) -> char {
        let mut braille_val = 0x2800;
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();

        // Braille dot mapping (Unicode 0x2800 to 0x28FF)
        // Col 1: dots 1, 2, 3, 7 -> bits 0, 1, 2, 6 (0x01, 0x02, 0x04, 0x40)
        // Col 2: dots 4, 5, 6, 8 -> bits 3, 4, 5, 7 (0x08, 0x10, 0x20, 0x80)
        let dot_map = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

        for dy in 0..4 {
            for dx in 0..2 {
                let x = start_x + dx;
                let y = start_y + dy;

                if x < width && y < height {
                    if let Some(pixel) = self.framebuffer.get_pixel(x as i32, y as i32) {
                        let lum = pixel_luminance(pixel);
                        let is_on = if self.config.invert {
                            lum < self.config.threshold
                        } else {
                            lum >= self.config.threshold
                        };

                        if is_on {
                            braille_val |= dot_map[dy as usize][dx as usize];
                        }
                    }
                }
            }
        }

        std::char::from_u32(braille_val).unwrap_or(' ')
    }

    /// Converts the framebuffer to a String containing the Braille art.
    #[must_use]
    pub fn to_string(&self) -> String {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();

        let cols = (width + 1) / 2;
        let rows = (height + 3) / 4;

        // Estimate capacity: rows * (cols + 1 for newline) chars
        // Each Braille char is 3 bytes in UTF-8
        let mut result = String::with_capacity((rows * (cols + 1) * 3) as usize);

        for row in 0..rows {
            for col in 0..cols {
                let ch = self.cell_to_braille(col * 2, row * 4);
                result.push(ch);
            }
            result.push('\n');
        }

        result
    }

    /// Exports the Braille text to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written to.
    pub fn export_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let braille_text = self.to_string();
        writer.write_all(braille_text.as_bytes())?;

        Ok(())
    }
}

impl fmt::Display for BrailleConverter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();

        let cols = (width + 1) / 2;
        let rows = (height + 3) / 4;

        for row in 0..rows {
            for col in 0..cols {
                let ch = self.cell_to_braille(col * 2, row * 4);
                f.write_char(ch)?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_conversion() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // Set all pixels to white
        fb.clear(0xFF_FFFFFF);

        let config = BrailleConfig {
            threshold: 128,
            invert: false,
        };

        let converter = BrailleConverter::new(&fb, config);

        let s = converter.to_string();
        // Fully white 2x4 should be full Braille block U+28FF (⣿)
        assert_eq!(s, "⣿\n");
    }

    #[test]
    fn test_braille_conversion_empty() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // Set all pixels to black
        fb.clear(0xFF_000000);

        let config = BrailleConfig {
            threshold: 128,
            invert: false,
        };

        let converter = BrailleConverter::new(&fb, config);

        let s = converter.to_string();
        // Fully black 2x4 should be empty Braille block U+2800 (⠀)
        assert_eq!(s, "⠀\n");
    }

    #[test]
    fn test_braille_conversion_invert() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF_FFFFFF); // White

        let config = BrailleConfig {
            threshold: 128,
            invert: true, // White is ignored, black is dot
        };

        let converter = BrailleConverter::new(&fb, config);

        let s = converter.to_string();
        // Fully white but inverted should be empty Braille block U+2800
        assert_eq!(s, "⠀\n");
    }
}
