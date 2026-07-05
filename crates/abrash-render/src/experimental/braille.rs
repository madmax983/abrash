//! Braille Display Renderer
//!
//! Converts a framebuffer into a high-resolution terminal output by mapping
//! 2x4 pixel grids to Unicode Braille characters. This allows for a 4x denser
//! display resolution compared to standard ASCII block characters.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Converts the framebuffer into a String of Braille characters.
///
/// Every 2x4 block of pixels is converted into a single Unicode Braille character,
/// providing effectively 4x the resolution of standard ASCII conversion in the terminal.
#[must_use]
pub fn render_braille(fb: &Framebuffer, threshold: u8) -> String {
    let w = fb.width() as usize;
    let h = fb.height() as usize;

    // Output dimensions in characters
    let out_w = (w + 1) / 2;
    let out_h = (h + 3) / 4;

    let mut result = String::with_capacity(out_w * out_h * 4 + out_h);

    for cy in 0..out_h {
        for cx in 0..out_w {
            let mut braille_char = 0x2800;

            // Map 2x4 grid
            // Dot 1 (top left): 0x01
            // Dot 2 (mid-top left): 0x02
            // Dot 3 (mid-bottom left): 0x04
            // Dot 4 (top right): 0x08
            // Dot 5 (mid-top right): 0x10
            // Dot 6 (mid-bottom right): 0x20
            // Dot 7 (bottom left): 0x40
            // Dot 8 (bottom right): 0x80

            let px = cx * 2;
            let py = cy * 4;

            let mut is_on = |x: usize, y: usize| -> bool {
                if x >= w || y >= h {
                    false
                } else {
                    fb.get_pixel(x as i32, y as i32)
                        .map_or(false, |pixel| pixel_luminance(pixel) >= threshold)
                }
            };

            if is_on(px, py) {
                braille_char |= 0x01;
            }
            if is_on(px, py + 1) {
                braille_char |= 0x02;
            }
            if is_on(px, py + 2) {
                braille_char |= 0x04;
            }
            if is_on(px + 1, py) {
                braille_char |= 0x08;
            }
            if is_on(px + 1, py + 1) {
                braille_char |= 0x10;
            }
            if is_on(px + 1, py + 2) {
                braille_char |= 0x20;
            }
            if is_on(px, py + 3) {
                braille_char |= 0x40;
            }
            if is_on(px + 1, py + 3) {
                braille_char |= 0x80;
            }

            result.push(std::char::from_u32(braille_char).unwrap_or(' '));
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
        let result = render_braille(&fb, 128);
        assert_eq!(result, "⠀\n"); // 0x2800 is '⠀'
    }

    #[test]
    fn test_braille_full() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFFFF_FFFF); // White
        let result = render_braille(&fb, 128);
        assert_eq!(result, "⣿\n"); // 0x28FF is '⣿'
    }

    #[test]
    fn test_braille_partial() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF00_0000); // Black
        fb.set_pixel(0, 0, 0xFFFF_FFFF);
        fb.set_pixel(1, 3, 0xFFFF_FFFF);

        let result = render_braille(&fb, 128);
        let expected_char = std::char::from_u32(0x2800 | 0x01 | 0x80).unwrap();
        assert_eq!(result, format!("{}\n", expected_char));
    }
}
