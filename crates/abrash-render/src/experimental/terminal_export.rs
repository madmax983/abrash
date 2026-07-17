use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Converts a Framebuffer to a string of Braille characters.
/// Uses 2x4 pixel blocks per character for high-density terminal output.
#[must_use]
pub fn framebuffer_to_braille(fb: &Framebuffer, luminance_threshold: u8) -> String {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Braille characters are 2x4 pixels
    let out_width = width / 2;
    let out_height = height / 4;

    let mut output = String::with_capacity(out_width * out_height + out_height);

    for char_y in 0..out_height {
        for char_x in 0..out_width {
            let base_x = char_x * 2;
            let base_y = char_y * 4;

            let mut braille_char = 0u8;

            // The Unicode Braille pattern is based on this dot numbering:
            // 1 4
            // 2 5
            // 3 6
            // 7 8
            let dot_map = [
                (0, 0, 0),
                (0, 1, 1),
                (0, 2, 2),
                (1, 0, 3),
                (1, 1, 4),
                (1, 2, 5),
                (0, 3, 6),
                (1, 3, 7),
            ];

            for &(dx, dy, bit) in &dot_map {
                let px = base_x + dx;
                let py = base_y + dy;

                if px < width && py < height {
                    if let Some(pixel) = fb.get_pixel(px as i32, py as i32) {
                        if pixel_luminance(pixel) >= luminance_threshold {
                            braille_char |= 1 << bit;
                        }
                    }
                }
            }

            let c = std::char::from_u32(0x2800 + u32::from(braille_char)).unwrap_or(' ');
            output.push(c);
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_export() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        // Clear to black
        fb.clear(0xFF000000);
        // Set top-left dot to white
        fb.set_pixel(0, 0, 0xFFFFFFFF);
        // Set bottom-right dot to white
        fb.set_pixel(1, 3, 0xFFFFFFFF);

        let s = framebuffer_to_braille(&fb, 128);

        // Dot 1 (x=0, y=0) is bit 0 -> 1
        // Dot 8 (x=1, y=3) is bit 7 -> 128
        // Total = 129 (0x81)
        // 0x2800 + 0x81 = 0x2881 => ⢁
        assert_eq!(s.trim_end(), "\u{2881}");
    }
}
