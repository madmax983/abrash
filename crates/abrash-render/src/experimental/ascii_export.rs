//! ASCII Export extension trait for Framebuffer.

use crate::ascii::{AsciiCharset, AsciiConverter};
use crate::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Trait to allow exporting a Framebuffer to ASCII files.
pub trait AsciiExporter {
    /// Exports the framebuffer as plain text ASCII art to a file.
    ///
    /// The brightness of each pixel determines the chosen ASCII character.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    fn export_ascii<P: AsRef<Path>>(&self, path: P, charset: AsciiCharset) -> io::Result<()>;

    /// Exports the framebuffer as colored ANSI text to a file.
    ///
    /// This uses the same characters as `export_ascii`, but adds ANSI escape codes
    /// so that it appears in color when printed in a terminal.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    fn export_ansi<P: AsRef<Path>>(&self, path: P, charset: AsciiCharset) -> io::Result<()>;
}

impl AsciiExporter for Framebuffer {
    fn export_ascii<P: AsRef<Path>>(&self, path: P, charset: AsciiCharset) -> io::Result<()> {
        let converter = AsciiConverter::new(self, charset);
        let ascii_str = converter.to_string();

        let mut file = File::create(path)?;
        file.write_all(ascii_str.as_bytes())?;

        Ok(())
    }

    fn export_ansi<P: AsRef<Path>>(&self, path: P, charset: AsciiCharset) -> io::Result<()> {
        let converter = AsciiConverter::new(self, charset);
        let ansi_str = converter.to_colored_string();

        let mut file = File::create(path)?;
        file.write_all(ansi_str.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_export_ascii_file() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_FFFF); // White
        fb.set_pixel(1, 0, 0xFF00_0000); // Black
        fb.set_pixel(0, 1, 0xFF00_0000); // Black
        fb.set_pixel(1, 1, 0xFFFF_FFFF); // White

        let path = "test_ascii_export.txt";
        fb.export_ascii(path, AsciiCharset::Standard).unwrap();

        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "@ \n @\n");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_export_ansi_file() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000); // Red
        fb.set_pixel(1, 0, 0xFF00_FF00); // Green

        let path = "test_ansi_export.ans";
        fb.export_ansi(path, AsciiCharset::Standard).unwrap();

        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert!(contents.contains("\x1b[38;2;255;0;0m"));
        assert!(contents.contains("\x1b[38;2;0;255;0m"));

        fs::remove_file(path).unwrap();
    }
}
