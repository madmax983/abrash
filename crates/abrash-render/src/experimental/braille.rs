//! Braille Display Exporter
//!
//! A post-processing effect that converts the framebuffer into a Braille art display,
//! rendering high-resolution text using the 2x4 Unicode Braille Patterns block.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the Braille Display effect.
#[derive(Debug, Clone, Copy)]
pub struct BrailleConfig {
    /// Luminance threshold (0-255) for determining if a dot is "on".
    pub threshold: u8,
    /// Whether to apply ANSI color codes to the output.
    pub colored: bool,
    /// Invert the threshold (dark pixels become dots instead of light ones).
    pub invert: bool,
}

impl Default for BrailleConfig {
    fn default() -> Self {
        Self {
            threshold: 128,
            colored: false,
            invert: false,
        }
    }
}

/// Generates a Braille string representation of the given Framebuffer.
#[must_use]
pub fn generate_braille(fb: &Framebuffer, config: BrailleConfig) -> String {
    use std::fmt::Write;
    let width = fb.width();
    let height = fb.height();
    let mut result = String::with_capacity((width * height / 8) as usize);

    // Braille characters are 2x4 pixels
    for y in (0..height).step_by(4) {
        for x in (0..width).step_by(2) {
            let mut braille_char_offset = 0u32;
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut count = 0u32;

            // Unicode Braille dot mapping:
            // 0 3
            // 1 4
            // 2 5
            // 6 7
            let dot_map = [[0, 3], [1, 4], [2, 5], [6, 7]];

            for dy in 0..4 {
                for dx in 0..2 {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width && py < height {
                        if let Some(pixel) = fb.get_pixel(px as i32, py as i32) {
                            let luma = pixel_luminance(pixel);
                            let is_on = if config.invert {
                                luma < config.threshold
                            } else {
                                luma >= config.threshold
                            };

                            if is_on {
                                let bit = dot_map[dy as usize][dx as usize];
                                braille_char_offset |= 1 << bit;

                                if config.colored {
                                    r_sum += (pixel >> 16) & 0xFF;
                                    g_sum += (pixel >> 8) & 0xFF;
                                    b_sum += pixel & 0xFF;
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }

            let ch = std::char::from_u32(0x2800 + braille_char_offset).unwrap_or(' ');

            if config.colored {
                if count > 0 {
                    let r = r_sum / count;
                    let g = g_sum / count;
                    let b = b_sum / count;
                    let _ = write!(result, "\x1b[38;2;{r};{g};{b}m{ch}");
                } else {
                    // No dots on, so no color
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }
        if config.colored {
            result.push_str("\x1b[0m\n");
        } else {
            result.push('\n');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_braille_empty() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // All black (under threshold)
        fb.clear(0xFF000000);
        let config = BrailleConfig::default();
        let result = generate_braille(&fb, config);
        // The empty Braille character is U+2800 (10240 in decimal)
        assert_eq!(result, "\u{2800}\n");
    }

    #[test]
    fn test_generate_braille_full() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // All white (above threshold)
        fb.clear(0xFFFFFFFF);
        let config = BrailleConfig::default();
        let result = generate_braille(&fb, config);
        // The full Braille character is U+28FF
        assert_eq!(result, "\u{28FF}\n");
    }

    #[test]
    fn test_generate_braille_colored() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFFFF0000); // Red
        let config = BrailleConfig {
            colored: true,
            threshold: 50,
            ..Default::default()
        };
        let result = generate_braille(&fb, config);
        // Should contain ANSI red and the full braille character, ending with a reset
        assert!(result.contains("\x1b[38;2;255;0;0m"));
        assert!(result.contains('\u{28FF}'));
        assert!(result.ends_with("\x1b[0m\n"));
    }
}
