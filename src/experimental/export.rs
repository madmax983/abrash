//! Image Export Module
//!
//! Provides zero-dependency functionality to export a [`Framebuffer`] to standard image formats.
//! Supported formats include PPM (Portable Pixmap) and TGA (Truevision TGA).
//!
//! # Examples
//!
//! ```
//! use abrash::framebuffer::Framebuffer;
//! use abrash::experimental::export::ImageExporter;
//! use std::fs;
//!
//! let mut fb = Framebuffer::new(10, 10).unwrap();
//! fb.clear(0xFFFF0000); // Red
//!
//! // Export to TGA
//! fb.export_tga("test_output.tga").unwrap();
//!
//! // Cleanup
//! fs::remove_file("test_output.tga").unwrap();
//! ```

use crate::framebuffer::Framebuffer;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Trait providing image export capabilities to the `Framebuffer`.
pub trait ImageExporter {
    /// Exports the framebuffer to a Portable Pixmap (PPM) file.
    ///
    /// PPM is a simple, uncompressed RGB format.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    fn export_ppm<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;

    /// Exports the framebuffer to an uncompressed Truevision TGA file.
    ///
    /// TGA is a widely supported format that stores uncompressed RGB/RGBA data.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    fn export_tga<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}

impl ImageExporter for Framebuffer {
    fn export_ppm<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write PPM header (P6 = binary RGB)
        writeln!(writer, "P6")?;
        writeln!(writer, "{} {}", self.width(), self.height())?;
        writeln!(writer, "255")?;

        // Write pixel data
        let pixels = self.as_slice();
        let mut row_buffer = Vec::with_capacity((self.width() * 3) as usize);

        for y in 0..self.height() {
            row_buffer.clear();
            for x in 0..self.width() {
                // Get pixel at (x, y)
                let pixel = pixels[(y * self.width() + x) as usize];

                // Extract RGB components (0xAARRGGBB)
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;

                row_buffer.push(r);
                row_buffer.push(g);
                row_buffer.push(b);
            }
            writer.write_all(&row_buffer)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn export_tga<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // TGA Header (18 bytes)
        let mut header = [0u8; 18];
        header[2] = 2; // Uncompressed, true-color image

        // Width and height (little-endian)
        let width = self.width() as u16;
        let height = self.height() as u16;
        header[12] = (width & 0xFF) as u8;
        header[13] = (width >> 8) as u8;
        header[14] = (height & 0xFF) as u8;
        header[15] = (height >> 8) as u8;

        header[16] = 24; // 24 bits per pixel (BGR)
        header[17] = 0x20; // Top-down image (origin in upper left)

        writer.write_all(&header)?;

        // Write pixel data (TGA stores data in BGR format)
        let pixels = self.as_slice();
        let mut row_buffer = Vec::with_capacity((self.width() * 3) as usize);

        for y in 0..self.height() {
            row_buffer.clear();
            for x in 0..self.width() {
                let pixel = pixels[(y * self.width() + x) as usize];

                // Extract BGR components (0xAARRGGBB)
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;

                // TGA uses BGR
                row_buffer.push(b);
                row_buffer.push(g);
                row_buffer.push(r);
            }
            writer.write_all(&row_buffer)?;
        }

        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_export_ppm() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Red, Green
        // Blue, White
        fb.set_pixel(0, 0, 0xFFFF0000); // R
        fb.set_pixel(1, 0, 0xFF00FF00); // G
        fb.set_pixel(0, 1, 0xFF0000FF); // B
        fb.set_pixel(1, 1, 0xFFFFFFFF); // W

        let path = "test_image.ppm";
        fb.export_ppm(path).unwrap();

        // Read and verify
        let mut file = File::open(path).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        // Check header (P6\n2 2\n255\n)
        let header = b"P6\n2 2\n255\n";
        assert_eq!(&contents[..header.len()], header);

        // Check pixel data
        let pixel_data = &contents[header.len()..];
        let expected_data = vec![
            255, 0, 0, // R
            0, 255, 0, // G
            0, 0, 255, // B
            255, 255, 255, // W
        ];
        assert_eq!(pixel_data, expected_data.as_slice());

        // Cleanup
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_export_tga() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Red, Green
        // Blue, White
        fb.set_pixel(0, 0, 0xFFFF0000); // R
        fb.set_pixel(1, 0, 0xFF00FF00); // G
        fb.set_pixel(0, 1, 0xFF0000FF); // B
        fb.set_pixel(1, 1, 0xFFFFFFFF); // W

        let path = "test_image.tga";
        fb.export_tga(path).unwrap();

        // Read and verify
        let mut file = File::open(path).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        // Check TGA Header length
        assert!(contents.len() >= 18);
        assert_eq!(contents[2], 2); // Uncompressed true-color
        assert_eq!(contents[12], 2); // Width
        assert_eq!(contents[13], 0);
        assert_eq!(contents[14], 2); // Height
        assert_eq!(contents[15], 0);
        assert_eq!(contents[16], 24); // 24 BPP
        assert_eq!(contents[17], 0x20); // Top-down

        // Check pixel data (BGR)
        let pixel_data = &contents[18..];
        let expected_data = vec![
            0, 0, 255, // R (BGR)
            0, 255, 0, // G (BGR)
            255, 0, 0, // B (BGR)
            255, 255, 255, // W (BGR)
        ];
        assert_eq!(pixel_data, expected_data.as_slice());

        // Cleanup
        fs::remove_file(path).unwrap();
    }
}
