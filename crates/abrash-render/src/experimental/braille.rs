//! High-Density Braille Exporter
//!
//! Converts a framebuffer into a string of Unicode Braille characters (U+2800 to U+28FF)
//! by mathematically mapping 2x4 pixel blocks to corresponding braille dots based on a
//! luminance threshold.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the Braille Exporter.
#[derive(Debug, Clone, Copy)]
pub struct BrailleConfig {
    /// Luminance threshold (0-255). Pixels brighter than this will become a raised dot.
    pub threshold: u8,
}

impl Default for BrailleConfig {
    fn default() -> Self {
        Self { threshold: 128 }
    }
}

/// Converts the given framebuffer into a high-density Braille string representation.
///
/// Every 2x4 block of pixels in the framebuffer is mapped to a single Unicode Braille
/// character (U+2800 to U+28FF).
#[must_use]
pub fn export_to_braille(fb: &Framebuffer, config: &BrailleConfig) -> String {
    let w = fb.width();
    let h = fb.height();

    // Braille characters represent a 2x4 pixel grid.
    let out_w = w / 2;
    let out_h = h / 4;

    // Each row of output has `out_w` chars + '\n' (total `out_w + 1`)
    let capacity = (out_w + 1) as usize * out_h as usize;
    let mut result = String::with_capacity(capacity * 3); // Braille characters are 3 bytes in UTF-8

    let threshold = config.threshold;

    for y in 0..out_h {
        for x in 0..out_w {
            let px0 = fb.get_pixel((x * 2) as i32, (y * 4) as i32).unwrap_or(0);
            let px1 = fb
                .get_pixel((x * 2) as i32, (y * 4 + 1) as i32)
                .unwrap_or(0);
            let px2 = fb
                .get_pixel((x * 2) as i32, (y * 4 + 2) as i32)
                .unwrap_or(0);
            let px3 = fb
                .get_pixel((x * 2 + 1) as i32, (y * 4) as i32)
                .unwrap_or(0);
            let px4 = fb
                .get_pixel((x * 2 + 1) as i32, (y * 4 + 1) as i32)
                .unwrap_or(0);
            let px5 = fb
                .get_pixel((x * 2 + 1) as i32, (y * 4 + 2) as i32)
                .unwrap_or(0);
            let px6 = fb
                .get_pixel((x * 2) as i32, (y * 4 + 3) as i32)
                .unwrap_or(0);
            let px7 = fb
                .get_pixel((x * 2 + 1) as i32, (y * 4 + 3) as i32)
                .unwrap_or(0);

            let mut dot_mask = 0u32;

            if pixel_luminance(px0) >= threshold {
                dot_mask |= 0x01;
            } // Dot 1 (top left)
            if pixel_luminance(px1) >= threshold {
                dot_mask |= 0x02;
            } // Dot 2 (middle left)
            if pixel_luminance(px2) >= threshold {
                dot_mask |= 0x04;
            } // Dot 3 (bottom left)
            if pixel_luminance(px3) >= threshold {
                dot_mask |= 0x08;
            } // Dot 4 (top right)
            if pixel_luminance(px4) >= threshold {
                dot_mask |= 0x10;
            } // Dot 5 (middle right)
            if pixel_luminance(px5) >= threshold {
                dot_mask |= 0x20;
            } // Dot 6 (bottom right)
            if pixel_luminance(px6) >= threshold {
                dot_mask |= 0x40;
            } // Dot 7 (lowest left)
            if pixel_luminance(px7) >= threshold {
                dot_mask |= 0x80;
            } // Dot 8 (lowest right)

            let braille_char = char::from_u32(0x2800 + dot_mask).unwrap_or(' ');
            result.push(braille_char);
        }
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_braille_export() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF000000); // Black

        // Turn on top-left pixel
        fb.set_pixel(0, 0, 0xFFFFFFFF);

        let braille = export_to_braille(&fb, &BrailleConfig::default());
        assert_eq!(braille.trim(), "⠁"); // U+2801
    }
}
