//! Image Export Tools
//!
//! A set of utilities for exporting the [`Framebuffer`] to common image formats without relying
//! on massive external dependencies like the `image` crate. Useful for debugging, creating assets,
//! or dumping renders to disk.
//!
//! # Formats Supported
//!
//! *   **PPM (Portable PixMap)**: Simplest uncompressed RGB format.
//! *   **TGA (Truevision Targa)**: Uncompressed BGR format with a small header, widely supported.
//! *   **ANSI (.ans)**: A text file containing terminal color escape codes for rendering images in the console using `cat`.

use crate::ascii::{AsciiCharset, AsciiConverter};
use crate::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// A utility struct for exporting framebuffers to various formats.
pub struct ImageExporter;

impl ImageExporter {
    /// Saves the framebuffer as a binary PPM (P6) file.
    ///
    /// PPM is an extremely simple image format. It starts with a short text header
    /// followed by uncompressed raw RGB bytes.
    ///
    /// # Arguments
    ///
    /// * `framebuffer` - The framebuffer to export.
    /// * `path` - The path to save the file to.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the file cannot be created or written to.
    pub fn save_ppm<P: AsRef<Path>>(framebuffer: &Framebuffer, path: P) -> io::Result<()> {
        let width = framebuffer.width();
        let height = framebuffer.height();

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // PPM P6 header
        writeln!(writer, "P6\n{width} {height}\n255")?;

        // Write raw RGB data
        for y in 0..height {
            for x in 0..width {
                let pixel = framebuffer.get_pixel(x as i32, y as i32).unwrap_or(0);
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;
                writer.write_all(&[r, g, b])?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    /// Saves the framebuffer as an uncompressed Truevision Targa (TGA) file.
    ///
    /// TGA is an old but very widely supported image format. It stores pixels in BGR order
    /// natively, making it a good fit for uncompressed color data and debugging assets.
    ///
    /// # Arguments
    ///
    /// * `framebuffer` - The framebuffer to export.
    /// * `path` - The path to save the file to.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the file cannot be created or written to.
    pub fn save_tga<P: AsRef<Path>>(framebuffer: &Framebuffer, path: P) -> io::Result<()> {
        let width = framebuffer.width();
        let height = framebuffer.height();

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // 18-byte TGA Header
        let mut header = [0u8; 18];
        header[2] = 2; // Image Type: 2 (Uncompressed True-Color Image)
        header[12] = (width & 0xFF) as u8;
        header[13] = ((width >> 8) & 0xFF) as u8;
        header[14] = (height & 0xFF) as u8;
        header[15] = ((height >> 8) & 0xFF) as u8;
        header[16] = 24; // Bits per pixel (RGB)
        header[17] = 0x20; // Image Descriptor: Top-to-bottom (0x20)

        writer.write_all(&header)?;

        // Write BGR data (TGA is natively BGR)
        for y in 0..height {
            for x in 0..width {
                let pixel = framebuffer.get_pixel(x as i32, y as i32).unwrap_or(0);
                let b = (pixel & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let r = ((pixel >> 16) & 0xFF) as u8;
                writer.write_all(&[b, g, r])?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    /// Saves the framebuffer as an ANSI colored text file (.ans).
    ///
    /// This is an experimental mashup connecting the Framebuffer with the AsciiConverter.
    /// The resulting file contains terminal escape codes and can be viewed directly
    /// in a terminal by using `cat filename.ans`.
    ///
    /// # Arguments
    ///
    /// * `framebuffer` - The framebuffer to export.
    /// * `path` - The path to save the file to.
    /// * `charset` - The character set to use for luminance mapping.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the file cannot be created or written to.
    pub fn save_ansi<P: AsRef<Path>>(
        framebuffer: &Framebuffer,
        path: P,
        charset: AsciiCharset,
    ) -> io::Result<()> {
        let converter = AsciiConverter::new(framebuffer, charset);
        let ansi_string = converter.to_colored_string();

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writer.write_all(ansi_string.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn create_test_framebuffer() -> Framebuffer {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Fill with a recognizable color pattern (Red and Blue)
        for y in 0..4 {
            for x in 0..4 {
                if (x + y) % 2 == 0 {
                    fb.set_pixel(x, y, 0x00FF_0000); // Red
                } else {
                    fb.set_pixel(x, y, 0x0000_00FF); // Blue
                }
            }
        }
        fb
    }

    #[test]
    fn test_save_ppm() {
        let fb = create_test_framebuffer();
        let path = env::temp_dir().join("test_output.ppm");

        assert!(ImageExporter::save_ppm(&fb, &path).is_ok());

        let data = fs::read(&path).expect("Failed to read test_output.ppm");
        // Header + raw data (4x4 = 16 pixels * 3 bytes)
        assert!(data.len() > 16 * 3);

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_save_tga() {
        let fb = create_test_framebuffer();
        let path = env::temp_dir().join("test_output.tga");

        assert!(ImageExporter::save_tga(&fb, &path).is_ok());

        let data = fs::read(&path).expect("Failed to read test_output.tga");
        // 18-byte header + raw data (4x4 = 16 pixels * 3 bytes)
        assert_eq!(data.len(), 18 + 16 * 3);
        assert_eq!(data[2], 2); // Uncompressed true color

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_save_ansi() {
        let fb = create_test_framebuffer();
        let path = env::temp_dir().join("test_output.ans");

        assert!(ImageExporter::save_ansi(&fb, &path, AsciiCharset::Standard).is_ok());

        let data = fs::read_to_string(&path).expect("Failed to read test_output.ans");
        // Check if ANSI color codes are present
        assert!(data.contains("\x1b[38;2;"));
        assert!(data.contains("\x1b[0m\n"));

        // Cleanup
        let _ = fs::remove_file(path);
    }
}
