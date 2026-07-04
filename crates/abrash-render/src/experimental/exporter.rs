//! Framebuffer Exporter
//!
//! Experimental module for exporting framebuffers to various image formats (BMP, PPM)
//! and ASCII representations without relying on heavy external dependencies.

use crate::framebuffer::Framebuffer;
use std::io::{self, Write};

/// Export the framebuffer to a PPM (Portable PixMap) format.
/// This format is uncompressed and very easy to parse.
pub fn export_ppm<W: Write>(fb: &Framebuffer, writer: &mut W) -> io::Result<()> {
    let width = fb.width();
    let height = fb.height();

    // PPM Header: P3 = ASCII RGB, width, height, max color value (255)
    writeln!(writer, "P3")?;
    writeln!(writer, "{} {}", width, height)?;
    writeln!(writer, "255")?;

    let pixels = fb.as_slice();
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let pixel = pixels[idx];
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            write!(writer, "{} {} {} ", r, g, b)?;
        }
        writeln!(writer)?;
    }

    Ok(())
}

/// Export the framebuffer to a BMP (Bitmap) format.
/// This creates an uncompressed 24-bit BMP image.
pub fn export_bmp<W: Write>(fb: &Framebuffer, writer: &mut W) -> io::Result<()> {
    let width = fb.width();
    let height = fb.height();

    // BMP lines must be padded to a multiple of 4 bytes
    let row_bytes = width * 3;
    let padding = (4 - (row_bytes % 4)) % 4;
    let padded_row_bytes = row_bytes + padding;

    let pixel_data_size = padded_row_bytes * height;
    let file_size = 54 + pixel_data_size; // 14 bytes file header + 40 bytes info header

    // 1. File Header (14 bytes)
    writer.write_all(b"BM")?; // Signature
    writer.write_all(&file_size.to_le_bytes())?; // File size
    writer.write_all(&[0, 0, 0, 0])?; // Reserved
    writer.write_all(&54u32.to_le_bytes())?; // Pixel data offset

    // 2. Info Header (40 bytes)
    writer.write_all(&40u32.to_le_bytes())?; // Header size
    writer.write_all(&(width as i32).to_le_bytes())?; // Image width
    writer.write_all(&(height as i32).to_le_bytes())?; // Image height (positive means bottom-up)
    writer.write_all(&1u16.to_le_bytes())?; // Planes
    writer.write_all(&24u16.to_le_bytes())?; // Bits per pixel
    writer.write_all(&0u32.to_le_bytes())?; // Compression (0 = none)
    writer.write_all(&pixel_data_size.to_le_bytes())?; // Image size
    writer.write_all(&2835u32.to_le_bytes())?; // X pixels per meter (~72 DPI)
    writer.write_all(&2835u32.to_le_bytes())?; // Y pixels per meter
    writer.write_all(&0u32.to_le_bytes())?; // Colors in color table
    writer.write_all(&0u32.to_le_bytes())?; // Important color count

    // 3. Pixel Data
    // BMP stores pixels bottom-up, and BGR format
    let pixels = fb.as_slice();
    let pad_bytes = [0u8; 3];

    for y in (0..height).rev() {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let pixel = pixels[idx];
            let r = ((pixel >> 16) & 0xFF) as u8;
            let g = ((pixel >> 8) & 0xFF) as u8;
            let b = (pixel & 0xFF) as u8;

            // BMP expects BGR order
            writer.write_all(&[b, g, r])?;
        }

        // Write padding
        if padding > 0 {
            writer.write_all(&pad_bytes[..padding as usize])?;
        }
    }

    Ok(())
}

/// Export the framebuffer to an ASCII art representation.
pub fn export_ascii<W: Write>(fb: &Framebuffer, writer: &mut W) -> io::Result<()> {
    let width = fb.width();
    let height = fb.height();
    let pixels = fb.as_slice();

    // Standard ASCII ramp from dark to light
    let ascii_ramp = b" .:-=+*#%@";

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let pixel = pixels[idx];
            let r = ((pixel >> 16) & 0xFF) as u32;
            let g = ((pixel >> 8) & 0xFF) as u32;
            let b = (pixel & 0xFF) as u32;

            // Calculate luminance using standard weights
            let luminance = (r * 299 + g * 587 + b * 114) / 1000;

            // Map luminance (0-255) to ASCII character
            let ramp_idx = (luminance * (ascii_ramp.len() as u32 - 1) / 255) as usize;
            let char = ascii_ramp[ramp_idx];

            // Write character twice to account for typical terminal character aspect ratio (tall)
            writer.write_all(&[char, char])?;
        }
        writeln!(writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppm_export() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFF_FF0000); // Red

        let mut out = Vec::new();
        export_ppm(&fb, &mut out).unwrap();

        let ppm_str = String::from_utf8(out).unwrap();
        assert!(ppm_str.starts_with("P3"));
        assert!(ppm_str.contains("255 0 0"));
    }

    #[test]
    fn test_bmp_export() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFF_00FF00); // Green

        let mut out = Vec::new();
        export_bmp(&fb, &mut out).unwrap();

        // BMP signature
        assert_eq!(&out[0..2], b"BM");
        // Ensure some pixel data is green in BGR format
        assert!(out.windows(3).any(|w| w == [0, 255, 0]));
    }

    #[test]
    fn test_ascii_export() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF_000000); // Black
        fb.set_pixel(1, 0, 0xFF_808080); // Gray
        fb.set_pixel(2, 0, 0xFF_FFFFFF); // White

        let mut out = Vec::new();
        export_ascii(&fb, &mut out).unwrap();

        let ascii_str = String::from_utf8(out).unwrap();

        // The ramp maps 0 to space, 128 to something in the middle (like + or =), 255 to @
        assert!(ascii_str.starts_with("  ")); // Black -> 2 spaces
        assert!(ascii_str.contains("@@")); // White -> 2 @s
    }
}
