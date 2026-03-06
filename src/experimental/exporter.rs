use std::fs::File;
use std::io::{BufWriter, Error, Write};
use std::path::Path;

use crate::framebuffer::Framebuffer;

/// Exports the given Framebuffer to a PPM (Portable Pixmap) file.
///
/// PPM is a very simple image format that stores uncompressed RGB pixel data.
/// It is supported by many image viewers and tools (e.g., ImageMagick, GIMP).
///
/// # Arguments
///
/// * `fb` - The framebuffer to export.
/// * `path` - The file path to write the image to.
///
/// # Errors
///
/// Returns an `std::io::Error` if the file cannot be created or written to.
pub fn export_ppm<P: AsRef<Path>>(fb: &Framebuffer, path: P) -> Result<(), Error> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let width = fb.width();
    let height = fb.height();

    // Write the PPM header
    // P6: Binary RGB format
    // width height
    // 255 (max color value)
    writeln!(writer, "P6\n{width} {height}\n255")?;

    // Write the pixel data (RGB)
    for pixel in fb.as_slice() {
        // Framebuffer uses ARGB (0xAARRGGBB), but PPM needs RGB (3 bytes per pixel).
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;

        writer.write_all(&[r, g, b])?;
    }

    // Flush the writer to ensure all data is written to disk
    writer.flush()?;

    Ok(())
}

/// Exports the given Framebuffer to a TGA (Truevision TGA) file.
///
/// TGA is a widely supported, relatively simple image format that
/// handles uncompressed pixel data (in BGR or BGRA format).
///
/// # Arguments
///
/// * `fb` - The framebuffer to export.
/// * `path` - The file path to write the image to.
///
/// # Errors
///
/// Returns an `std::io::Error` if the file cannot be created or written to.
pub fn export_tga<P: AsRef<Path>>(fb: &Framebuffer, path: P) -> Result<(), Error> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let width = fb.width() as u16;
    let height = fb.height() as u16;

    // TGA Header (18 bytes)
    // 0: ID length (0 = no image ID field)
    // 1: Color map type (0 = no color map)
    // 2: Image type (2 = uncompressed true-color image)
    // 3-7: Color map specification (ignored, 5 bytes of 0)
    // 8-9: X-origin (0)
    // 10-11: Y-origin (0)
    // 12-13: Image width
    // 14-15: Image height
    // 16: Pixel depth (24 bits per pixel)
    // 17: Image descriptor (0x20 = Top-down orientation)
    let header = [
        0,
        0,
        2,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        (width & 0xFF) as u8,
        (width >> 8) as u8,
        (height & 0xFF) as u8,
        (height >> 8) as u8,
        24,
        0x20, // 24 bpp, top-down
    ];

    writer.write_all(&header)?;

    // Write the pixel data (BGR)
    for pixel in fb.as_slice() {
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;

        writer.write_all(&[b, g, r])?;
    }

    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_export_ppm() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set red
        fb.set_pixel(0, 0, 0xFFFF0000);
        // Set green
        fb.set_pixel(1, 0, 0xFF00FF00);
        // Set blue
        fb.set_pixel(0, 1, 0xFF0000FF);
        // Set white
        fb.set_pixel(1, 1, 0xFFFFFFFF);

        let temp_file = "test_export.ppm";
        export_ppm(&fb, temp_file).unwrap();

        let content = fs::read(temp_file).unwrap();

        // P6\n2 2\n255\n
        let header = b"P6\n2 2\n255\n";
        assert_eq!(&content[0..header.len()], header);

        let pixel_data = &content[header.len()..];
        // Red
        assert_eq!(pixel_data[0..3], [255, 0, 0]);
        // Green
        assert_eq!(pixel_data[3..6], [0, 255, 0]);
        // Blue
        assert_eq!(pixel_data[6..9], [0, 0, 255]);
        // White
        assert_eq!(pixel_data[9..12], [255, 255, 255]);

        fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_export_tga() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set red
        fb.set_pixel(0, 0, 0xFFFF0000);
        // Set green
        fb.set_pixel(1, 0, 0xFF00FF00);
        // Set blue
        fb.set_pixel(0, 1, 0xFF0000FF);
        // Set white
        fb.set_pixel(1, 1, 0xFFFFFFFF);

        let temp_file = "test_export.tga";
        export_tga(&fb, temp_file).unwrap();

        let content = fs::read(temp_file).unwrap();

        let header = [0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 2, 0, 24, 0x20];
        assert_eq!(&content[0..18], &header);

        let pixel_data = &content[18..];
        // Red in BGR
        assert_eq!(pixel_data[0..3], [0, 0, 255]);
        // Green in BGR
        assert_eq!(pixel_data[3..6], [0, 255, 0]);
        // Blue in BGR
        assert_eq!(pixel_data[6..9], [255, 0, 0]);
        // White in BGR
        assert_eq!(pixel_data[9..12], [255, 255, 255]);

        fs::remove_file(temp_file).unwrap();
    }
}
