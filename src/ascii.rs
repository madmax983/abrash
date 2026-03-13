//! ASCII Art Converter.
//!
//! Converts a [`Framebuffer`] into ASCII art for terminal output or TUI rendering.
//!
//! This module provides the [`AsciiConverter`] struct which can transform pixel data
//! into character-based representations using luminance mapping.
//!
//! # Features
//!
//! *   **Standard Output**: Convert any framebuffer to a `String`.
//! *   **TUI Widget**: Implements `ratatui::widgets::Widget` (requires `backend-tui` or `backend-wasm`).
//!
//! # Examples
//!
//! ```
//! use abrash::framebuffer::Framebuffer;
//! use abrash::ascii::{AsciiConverter, AsciiCharset};
//!
//! let mut fb = Framebuffer::new(10, 5).unwrap();
//! fb.clear(0xFFFFFFFF); // White
//!
//! let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
//! let art = converter.to_string();
//! assert!(art.contains('@')); // White maps to dense characters
//! ```

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;
use std::fmt::{self, Write};

#[cfg(any(feature = "backend-tui", feature = "backend-wasm"))]
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

/// Character set used for luminance mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiCharset {
    /// Standard ASCII gradient: ` .:-=+*#%@`
    Standard,
    /// Block characters: ` ░▒▓█`
    Blocks,
    /// Minimal set: ` .:`
    Minimal,
    /// Binary set: ` 1`
    Binary,
}

impl AsciiCharset {
    /// Returns the characters in the set, ordered from darkest to brightest.
    #[must_use]
    pub const fn chars(self) -> &'static [char] {
        match self {
            Self::Standard => &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'],
            Self::Blocks => &[' ', '░', '▒', '▓', '█'],
            Self::Minimal => &[' ', '.', ':'],
            Self::Binary => &[' ', '1'],
        }
    }

    /// Maps a luminance value (0-255) to a character in the set.
    #[must_use]
    pub fn map(self, luminance: u8) -> char {
        let chars = self.chars();
        let len = chars.len();
        // Calculate index: (luminance * len) / 256
        // Use u16 to prevent overflow before division
        let index = (u16::from(luminance) * len as u16) >> 8;
        chars[index.min((len - 1) as u16) as usize]
    }
}

/// Converter for rendering a [`Framebuffer`] as ASCII art.
pub struct AsciiConverter<'a> {
    framebuffer: &'a Framebuffer,
    charset: AsciiCharset,
}

impl<'a> AsciiConverter<'a> {
    /// Creates a new ASCII converter for the given framebuffer.
    #[must_use]
    pub const fn new(framebuffer: &'a Framebuffer, charset: AsciiCharset) -> Self {
        Self {
            framebuffer,
            charset,
        }
    }

    /// Converts the framebuffer to a string with ANSI color codes.
    ///
    /// This method generates a string where each character is prefixed with an ANSI
    /// escape code for its color (RGB), resetting color at the end of each line.
    #[must_use]
    pub fn to_colored_string(&self) -> String {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        // Estimate capacity: (width * (chars per pixel + overhead)) * height
        // ANSI sequence is roughly "\x1b[38;2;RRR;GGG;BBBmC" -> ~20 chars
        let mut result = String::with_capacity(((width * 20) * height) as usize);

        for y in 0..height {
            for x in 0..width {
                if let Some(pixel) = self.framebuffer.get_pixel(x as i32, y as i32) {
                    let luminance = pixel_luminance(pixel);
                    let ch = self.charset.map(luminance);

                    let r = (pixel >> 16) & 0xFF;
                    let g = (pixel >> 8) & 0xFF;
                    let b = pixel & 0xFF;

                    // ANSI 24-bit color: ESC[38;2;R;G;Bm
                    // Note: This allocation inside loop is not ideal for performance but acceptable for ASCII resolution.
                    // A better approach would be to write directly to the buffer.
                    let _ = write!(result, "\x1b[38;2;{r};{g};{b}m{ch}");
                } else {
                    result.push(' ');
                }
            }
            // Reset color at end of line
            result.push_str("\x1b[0m\n");
        }
        result
    }
}

impl fmt::Display for AsciiConverter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        // Estimate capacity: (width + 1) * height
        let mut result = String::with_capacity(((width + 1) * height) as usize);

        for y in 0..height {
            for x in 0..width {
                if let Some(pixel) = self.framebuffer.get_pixel(x as i32, y as i32) {
                    let luminance = pixel_luminance(pixel);
                    result.push(self.charset.map(luminance));
                }
            }
            result.push('\n');
        }
        write!(f, "{result}")
    }
}

#[cfg(any(feature = "backend-tui", feature = "backend-wasm"))]
impl Widget for AsciiConverter<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let term_w = area.width as usize;
        let term_h = area.height as usize;
        let fb_w = self.framebuffer.width() as usize;
        let fb_h = self.framebuffer.height() as usize;

        for y in 0..term_h {
            for x in 0..term_w {
                // Nearest neighbor sampling
                let fb_x = (x * fb_w) / term_w;
                let fb_y = (y * fb_h) / term_h;

                if fb_x >= fb_w || fb_y >= fb_h {
                    continue;
                }

                if let Some(pixel) = self.framebuffer.get_pixel(fb_x as i32, fb_y as i32) {
                    let luminance = pixel_luminance(pixel);
                    let ch = self.charset.map(luminance);

                    // Extract color for FG
                    let r = ((pixel >> 16) & 0xFF) as u8;
                    let g = ((pixel >> 8) & 0xFF) as u8;
                    let b = (pixel & 0xFF) as u8;

                    if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + y as u16)) {
                        cell.set_char(ch).set_fg(Color::Rgb(r, g, b));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luminance_calculation() {
        // White
        assert_eq!(pixel_luminance(0xFFFF_FFFF), 255);
        // Black
        assert_eq!(pixel_luminance(0xFF00_0000), 0);
        // Red (pure) -> ~76
        assert_eq!(pixel_luminance(0xFFFF_0000), 76);
        // Green (pure) -> ~149
        assert_eq!(pixel_luminance(0xFF00_FF00), 149);
    }

    #[test]
    fn test_ascii_mapping() {
        let charset = AsciiCharset::Standard;
        // 0 -> ' '
        assert_eq!(charset.map(0), ' ');
        // 255 -> '@'
        assert_eq!(charset.map(255), '@');
        // Mid-grey -> somewhere in middle
        let mid = charset.map(128);
        assert!(mid == '+' || mid == '=' || mid == '*', "Got {mid}");
    }

    #[test]
    fn test_to_string() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_FFFF); // White -> @
        fb.set_pixel(1, 0, 0xFF00_0000); // Black -> ' '
        fb.set_pixel(0, 1, 0xFF00_0000); // Black -> ' '
        fb.set_pixel(1, 1, 0xFFFF_FFFF); // White -> @

        let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
        let s = converter.to_string();

        assert_eq!(s, "@ \n @\n");
    }

    #[test]
    fn test_block_charset() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF80_8080); // Mid-grey

        let converter = AsciiConverter::new(&fb, AsciiCharset::Blocks);
        let s = converter.to_string();

        // Should use one of the middle block characters
        assert!(s.contains('▒') || s.contains('▓'), "Got {s}");
    }
}
