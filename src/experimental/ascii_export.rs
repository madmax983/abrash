//! ASCII Image Exporter
//!
//! A mashup of `Framebuffer` and `AsciiConverter`.
//! Allows you to export rendered frames directly to `.txt` (raw ASCII)
//! or `.ans` (ANSI colored text) files. The `.ans` files can be viewed
//! directly in standard terminals using `cat`.

use crate::ascii::{AsciiCharset, AsciiConverter};
use crate::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Trait to extend `Framebuffer` with ASCII and ANSI export capabilities.
pub trait AsciiExporter {
    /// Exports the framebuffer to a plain text file using standard ASCII character mapping.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to write the `.txt` file to.
    ///
    /// # Errors
    ///
    /// Returns an error if file creation or writing fails.
    fn export_txt<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;

    /// Exports the framebuffer to an ANSI colored text file.
    /// This file can be viewed in standard terminals (e.g., via `cat`).
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to write the `.ans` file to.
    ///
    /// # Errors
    ///
    /// Returns an error if file creation or writing fails.
    fn export_ansi<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}

impl AsciiExporter for Framebuffer {
    fn export_txt<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let converter = AsciiConverter::new(self, AsciiCharset::Standard);
        let content = converter.to_string();
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())
    }

    fn export_ansi<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let converter = AsciiConverter::new(self, AsciiCharset::Standard);
        let content = converter.to_colored_string();
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_export_txt() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFFFFFFFF); // White -> '@'

        let test_path = "test_output.txt";
        fb.export_txt(test_path).unwrap();

        let content = fs::read_to_string(test_path).unwrap();
        assert!(content.contains("@@@@"));
        assert!(content.contains('\n'));

        // Clean up
        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_export_ansi() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFFFF0000); // Red

        let test_path = "test_output.ans";
        fb.export_ansi(test_path).unwrap();

        let content = fs::read_to_string(test_path).unwrap();
        // Check for ANSI color code for Red (255;0;0)
        assert!(content.contains("\x1b[38;2;255;0;0m"));
        // Check for reset code
        assert!(content.contains("\x1b[0m"));

        // Clean up
        let _ = fs::remove_file(test_path);
    }
}
