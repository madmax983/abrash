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
//! use abrash_core::framebuffer::Framebuffer;
//! use abrash_render::ascii::{AsciiConverter, AsciiCharset};
//!
//! let mut fb = Framebuffer::new(10, 5).unwrap();
//! fb.clear(0x00FF_FFFFFF); // White
//!
//! let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
//! let art = converter.to_string();
//! assert!(art.contains('@')); // White maps to dense characters
//! ```

use std::fmt::{self, Write as FmtWrite};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(any(feature = "backend-tui", feature = "backend-wasm"))]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Predefined character sets for ASCII conversion, ordered from darkest to lightest.
#[derive(Debug, Clone, Copy)]
pub enum AsciiCharset {
    /// 10 characters, standard progression.
    Standard,
    /// 70 characters, dense progression for more detail.
    Detailed,
    /// 3 characters, very minimal progression.
    Minimal,
    /// 2 characters, just black and white.
    Binary,
    /// Unicode block elements for solid shapes.
    Blocks,
}

impl AsciiCharset {
    /// Returns the character array for this set.
    #[must_use]
    pub const fn chars(&self) -> &'static [char] {
        match self {
            Self::Standard => &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'],
            Self::Detailed => &[
                ' ', '.', '\'', '`', '^', '"', ',', ':', ';', 'I', 'l', '!', 'i', '>', '<', '~',
                '+', '_', '-', '?', ']', '[', '}', '{', '1', ')', '(', '|', '\\', '/', 't', 'f',
                'j', 'r', 'x', 'n', 'u', 'v', 'c', 'z', 'X', 'Y', 'U', 'J', 'C', 'L', 'Q', '0',
                'O', 'Z', 'm', 'w', 'q', 'p', 'd', 'b', 'k', 'h', 'a', 'o', '*', '#', 'M', 'W',
                '&', '8', '%', 'B', '@', '$',
            ],
            Self::Minimal => &[' ', '.', ':'],
            Self::Binary => &[' ', '1'],
            Self::Blocks => &[' ', '░', '▒', '▓', '█'],
        }
    }

    /// Maps a luminance value (0 to 255) to a character in this set.
    #[must_use]
    pub fn map(&self, luminance: u8) -> char {
        let chars = self.chars();
        let idx = (luminance as usize * chars.len()) / 256;
        chars[idx.clamp(0, chars.len() - 1)]
    }
}

/// Converts a [`Framebuffer`] into an ASCII representation.
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

    /// Converts the framebuffer to a colored String with ANSI escape codes.
    ///
    /// The resulting string can be printed directly to an ANSI-compatible terminal
    /// to display a colored ASCII representation of the framebuffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    /// use abrash_render::ascii::{AsciiConverter, AsciiCharset};
    ///
    /// let mut fb = Framebuffer::new(2, 1).unwrap();
    /// fb.set_pixel(0, 0, 0xFFFF_0000); // Red
    /// fb.set_pixel(1, 0, 0xFF00_FF00); // Green
    ///
    /// let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
    /// let art = converter.to_colored_string();
    /// // The output contains ANSI escape codes for red and green.
    /// assert!(art.contains("\x1b[38;2;255;0;0m"));
    /// assert!(art.contains("\x1b[38;2;0;255;0m"));
    /// ```
    #[must_use]
    pub fn to_colored_string(&self) -> String {
        // ⚡ Bolt: Custom allocation-free integer formatting to avoid the massive overhead
        // of `write!` macro trait dispatch and formatting on hot per-pixel paths.
        fn push_u8(s: &mut String, mut n: u8) {
            if n == 0 {
                s.push('0');
                return;
            }
            let mut buf = [0u8; 3];
            let mut i = 3;
            while n > 0 {
                i -= 1;
                buf[i] = b'0' + (n % 10);
                n /= 10;
            }
            let s_slice = unsafe { std::str::from_utf8_unchecked(&buf[i..]) };
            s.push_str(s_slice);
        }

        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        // Estimate capacity: (width * (chars per pixel + overhead)) * height
        // ANSI sequence is roughly "\x1b[38;2;RRR;GGG;BBBmC" -> ~20 chars
        let mut result = String::with_capacity(((width * 20) * height) as usize);

        for row in self.framebuffer.as_slice().chunks_exact(width as usize) {
            for &pixel in row {
                let ch = self.charset.map(pixel_luminance(pixel));
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;

                result.push_str("\x1b[38;2;");
                push_u8(&mut result, r);
                result.push(';');
                push_u8(&mut result, g);
                result.push(';');
                push_u8(&mut result, b);
                result.push('m');
                result.push(ch);
            }
            // Reset color at end of line
            result.push_str("\x1b[0m\n");
        }
        result
    }

    /// Exports the framebuffer as plain text ASCII art to a file.
    ///
    /// The brightness of each pixel determines the chosen ASCII character.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    pub fn export_ascii<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let width = self.framebuffer.width();
        let mut char_buf = [0u8; 4];
        for row in self.framebuffer.as_slice().chunks_exact(width as usize) {
            for &pixel in row {
                let ch = self.charset.map(pixel_luminance(pixel));
                writer.write_all(ch.encode_utf8(&mut char_buf).as_bytes())?;
            }
            writer.write_all(b"\n")?;
        }

        Ok(())
    }

    /// Exports the framebuffer as colored ANSI text to a file.
    ///
    /// This uses the same characters as `export_ascii`, but adds ANSI escape codes
    /// so that it appears in color when printed in a terminal.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    pub fn export_ansi<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        // ⚡ Bolt: Custom allocation-free integer formatting for I/O
        // to avoid `write!` overhead when exporting large ANSI files.
        fn write_u8(w: &mut impl io::Write, mut n: u8) -> io::Result<()> {
            if n == 0 {
                return w.write_all(b"0");
            }
            let mut buf = [0u8; 3];
            let mut i = 3;
            while n > 0 {
                i -= 1;
                buf[i] = b'0' + (n % 10);
                n /= 10;
            }
            w.write_all(&buf[i..])
        }

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let width = self.framebuffer.width();
        for row in self.framebuffer.as_slice().chunks_exact(width as usize) {
            for &pixel in row {
                let ch = self.charset.map(pixel_luminance(pixel));
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;

                writer.write_all(b"\x1b[38;2;")?;
                write_u8(&mut writer, r)?;
                writer.write_all(b";")?;
                write_u8(&mut writer, g)?;
                writer.write_all(b";")?;
                write_u8(&mut writer, b)?;
                writer.write_all(b"m")?;
                let mut char_buf = [0u8; 4];
                writer.write_all(ch.encode_utf8(&mut char_buf).as_bytes())?;
            }
            writer.write_all(b"\x1b[0m\n")?;
        }

        Ok(())
    }
}

impl fmt::Display for AsciiConverter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.framebuffer.width();

        for row in self.framebuffer.as_slice().chunks_exact(width as usize) {
            for &pixel in row {
                f.write_char(self.charset.map(pixel_luminance(pixel)))?;
            }
            f.write_char('\n')?;
        }
        Ok(())
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
    fn test_ascii_mapping_minimal() {
        let charset = AsciiCharset::Minimal;
        assert_eq!(charset.chars(), &[' ', '.', ':']);
        assert_eq!(charset.map(0), ' ');
        assert_eq!(charset.map(128), '.');
        assert_eq!(charset.map(255), ':');
    }

    #[test]
    fn test_ascii_mapping_binary() {
        let charset = AsciiCharset::Binary;
        assert_eq!(charset.chars(), &[' ', '1']);
        assert_eq!(charset.map(0), ' ');
        assert_eq!(charset.map(255), '1');
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
    fn test_to_colored_string() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000); // Red -> mid brightness char
        fb.set_pixel(1, 0, 0xFF00_FF00); // Green -> mid-high brightness char

        let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
        let s = converter.to_colored_string();

        // Should contain ANSI color codes for Red and Green
        assert!(s.contains("\x1b[38;2;255;0;0m"));
        assert!(s.contains("\x1b[38;2;0;255;0m"));
        // Should reset at the end of the line
        assert!(s.ends_with("\x1b[0m\n"));
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

    #[test]
    fn test_export_ascii_file() {
        use std::io::Read;
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_FFFF); // White
        fb.set_pixel(1, 0, 0xFF00_0000); // Black
        fb.set_pixel(0, 1, 0xFF00_0000); // Black
        fb.set_pixel(1, 1, 0xFFFF_FFFF); // White

        let path = "test_ascii_export.txt";
        let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
        converter.export_ascii(path).unwrap();

        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "@ \n @\n");

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_export_ansi_file() {
        use std::io::Read;
        let mut fb = Framebuffer::new(2, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000); // Red
        fb.set_pixel(1, 0, 0xFF00_FF00); // Green

        let path = "test_ansi_export.ans";
        let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
        converter.export_ansi(path).unwrap();

        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert!(contents.contains("\x1b[38;2;255;0;0m"));
        assert!(contents.contains("\x1b[38;2;0;255;0m"));

        std::fs::remove_file(path).unwrap();
    }
}
