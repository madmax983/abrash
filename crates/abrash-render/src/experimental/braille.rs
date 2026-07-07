//! Braille Terminal Display Effect
//!
//! Converts a Framebuffer into a dense Braille unicode string, mapping 2x4 pixel
//! blocks to single Braille characters.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

pub struct BrailleConverter<'a> {
    framebuffer: &'a Framebuffer,
}

impl<'a> BrailleConverter<'a> {
    #[must_use]
    pub const fn new(framebuffer: &'a Framebuffer) -> Self {
        Self { framebuffer }
    }

    #[must_use]
    pub fn to_string(&self) -> String {
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;
        let mut result = String::with_capacity((width / 2 + 1) * (height / 4));

        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(2) {
                let mut bits = 0;
                let dot_coords = [
                    (0, 0, 0x01), (1, 0, 0x08),
                    (0, 1, 0x02), (1, 1, 0x10),
                    (0, 2, 0x04), (1, 2, 0x20),
                    (0, 3, 0x40), (1, 3, 0x80),
                ];

                for (dx, dy, bit) in dot_coords {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width && py < height {
                        if let Some(pixel) = self.framebuffer.get_pixel(px as i32, py as i32) {
                            if pixel_luminance(pixel) > 127 {
                                bits |= bit;
                            }
                        }
                    }
                }
                result.push(std::char::from_u32(0x2800 + bits).unwrap_or(' '));
            }
            result.push('\n');
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_braille_output() {
        let mut fb = Framebuffer::new(2, 4).unwrap();
        fb.clear(0xFF_000000);
        fb.set_pixel(0, 0, 0xFF_FFFFFF); // Top-left (0x01)
        fb.set_pixel(1, 1, 0xFF_FFFFFF); // Mid-right (0x10)
        let converter = BrailleConverter::new(&fb);
        let out = converter.to_string();
        assert!(out.starts_with(std::char::from_u32(0x2800 + 0x11).unwrap()));
    }
}
