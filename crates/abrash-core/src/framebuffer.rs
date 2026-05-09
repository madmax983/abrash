//! Pixel buffer management.
//!
//! Framebuffer stores pixels in row-major order as 32-bit RGBA values.
//!
//! # Pixel Format
//!
//! Colors are stored as `u32` in **0xAARRGGBB** format.
//!
//! *   **Alpha**: High byte (0xFF......)
//! *   **Red**: Second byte (0x..FF....)
//! *   **Green**: Third byte (0x....FF..)
//! *   **Blue**: Low byte (0x......FF)
//!
//! On Little-Endian machines (x86, ARM), this maps to `[B, G, R, A]` in memory.

/// A 32-bit pixel buffer.
pub struct Framebuffer {
    /// Pixel data in row-major order (0xAARRGGBB).
    pixels: Vec<u32>,
    /// Width of the buffer in pixels.
    width: u32,
    /// Height of the buffer in pixels.
    height: u32,
}

impl Framebuffer {
    /// Creates a new framebuffer with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// *   Dimensions exceed `i32::MAX` (for signed coordinate compatibility).
    /// *   The total pixel count overflows `u32` (preventing huge allocations).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    /// let fb = Framebuffer::new(800, 600).unwrap();
    /// ```
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err("Buffer dimensions too large (max i32::MAX)");
        }

        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .ok_or("Buffer size overflow")? as usize;

        Ok(Self {
            pixels: vec![0xFF00_0000; size], // Black with full alpha
            width,
            height,
        })
    }

    /// The width of the framebuffer in pixels.
    ///
    /// Essential when computing row bounds during scanline rendering or projecting NDC back to screen space.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let fb = Framebuffer::new(800, 600).unwrap();
    /// assert_eq!(fb.width(), 800);
    /// ```
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// The height of the framebuffer in pixels.
    ///
    /// Essential when computing row bounds during scanline rendering or projecting NDC back to screen space.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let fb = Framebuffer::new(800, 600).unwrap();
    /// assert_eq!(fb.height(), 600);
    /// ```
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Exposes the underlying pixel buffer as an immutable slice.
    ///
    /// This is highly useful for copying the rendered frame into texture memory
    /// or passing it directly to windowing system backends for display.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let fb = Framebuffer::new(2, 2).unwrap();
    /// let pixels = fb.as_slice();
    /// assert_eq!(pixels.len(), 4);
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    /// Exposes the underlying pixel buffer as a mutable slice.
    ///
    /// Direct mutable access to the backing slice allows for highly optimized
    /// memory operations (like `par_chunks_mut` in Rayon) for post-processing effects,
    /// bypassing bounds-checking overhead.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let mut fb = Framebuffer::new(10, 10).unwrap();
    /// let slice = fb.as_mut_slice();
    /// slice.fill(0xFF00_00FF); // Fill the screen with blue efficiently
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    /// Fills the entire framebuffer with a single color.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let mut fb = Framebuffer::new(100, 100).unwrap();
    /// // Clear the screen to a deep purple
    /// fb.clear(0xFF80_0080);
    /// ```
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Sets a pixel at (x, y) to the given color.
    ///
    /// Silently ignores out-of-bounds coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let mut fb = Framebuffer::new(100, 100).unwrap();
    /// // Set pixel at (10, 10) to Red
    /// fb.set_pixel(10, 10, 0xFFFF0000);
    ///
    /// assert_eq!(fb.get_pixel(10, 10), Some(0xFFFF0000));
    /// ```
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return; // Bounds check - silently ignore out of bounds
        }

        let index = (y as u32 * self.width + x as u32) as usize;
        self.pixels[index] = color;
    }

    /// Safely gets the pixel color at (x, y) if the coordinates are within bounds.
    ///
    /// The engine relies on optional returns for boundary conditions rather than panicking,
    /// enabling fast error-tolerant filtering algorithms.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let mut fb = Framebuffer::new(100, 100).unwrap();
    /// fb.set_pixel(10, 10, 0xFF00_FF00); // Set to Green
    ///
    /// assert_eq!(fb.get_pixel(10, 10), Some(0xFF00_FF00));
    /// assert_eq!(fb.get_pixel(200, 200), None); // Out of bounds
    /// ```
    #[inline]
    #[must_use]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }

        let index = (y as u32 * self.width + x as u32) as usize;
        Some(self.pixels[index])
    }

    /// Set pixel color without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure `x < width` and `y < height`.
    /// Calling this with out-of-bounds coordinates results in Undefined Behavior
    /// (likely a buffer overflow or segmentation fault).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let mut fb = Framebuffer::new(100, 100).unwrap();
    /// let x = 50;
    /// let y = 50;
    /// let color = 0xFFFF0000;
    ///
    /// if (x as u32) < fb.width() && (y as u32) < fb.height() {
    ///     unsafe {
    ///         fb.set_pixel_unchecked(x, y, color);
    ///     }
    /// }
    /// ```
    pub unsafe fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: u32) {
        let idx = y * self.width as usize + x;
        // SAFETY: Caller guarantees bounds
        unsafe {
            *self.pixels.get_unchecked_mut(idx) = color;
        }
    }

    /// Get pixel color without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure `x < width` and `y < height`.
    /// Calling this with out-of-bounds coordinates results in Undefined Behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let fb = Framebuffer::new(100, 100).unwrap();
    /// let x = 10;
    /// let y = 10;
    ///
    /// if (x as u32) < fb.width() && (y as u32) < fb.height() {
    ///     let color = unsafe { fb.get_pixel_unchecked(x, y) };
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn get_pixel_unchecked(&self, x: usize, y: usize) -> u32 {
        let idx = y * self.width as usize + x;
        // SAFETY: Caller guarantees bounds
        unsafe { *self.pixels.get_unchecked(idx) }
    }

    /// Clears a specific rectangular region of the screen to a given color.
    ///
    /// The coordinates automatically clamp to the visible screen bounds, making it perfectly safe
    /// for clearing HUD elements or dirty rectangles that might straddle the screen edge.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::framebuffer::Framebuffer;
    ///
    /// let mut fb = Framebuffer::new(800, 600).unwrap();
    /// fb.clear(0xFF00_0000); // Clear to Black
    ///
    /// // Draw a 100x100 Red square at the center of the screen
    /// fb.clear_rect(350, 250, 100, 100, 0xFFFF_0000);
    /// ```
    pub fn clear_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let x1 = x;
        let y1 = y;

        // Prevent overflow when adding width to x
        // Use i64 for intermediate calculation to avoid wrapping
        let x2_i64 = i64::from(x) + i64::from(width);
        let y2_i64 = i64::from(y) + i64::from(height);

        let x2 = if x2_i64 > i64::from(i32::MAX) {
            i32::MAX
        } else {
            x2_i64 as i32
        };
        let y2 = if y2_i64 > i64::from(i32::MAX) {
            i32::MAX
        } else {
            y2_i64 as i32
        };

        let start_x = x1.max(0).min(self.width as i32) as u32;
        let start_y = y1.max(0).min(self.height as i32) as u32;
        let end_x = x2.max(0).min(self.width as i32) as u32;
        let end_y = y2.max(0).min(self.height as i32) as u32;

        if start_x >= end_x || start_y >= end_y {
            return;
        }

        let sx = start_x as usize;
        let ex = end_x as usize;
        let sy = start_y as usize;
        let ey = end_y as usize;
        let w = self.width as usize;

        let start_idx = sy * w;
        let end_idx = ey * w;

        if sx == 0 && ex == w {
            // Fast path for full-width clears (avoids chunking overhead)
            self.pixels[start_idx..end_idx].fill(color);
        } else {
            let len = ex - sx;
            let mut offset = start_idx + sx;
            let slice = self.pixels.as_mut_slice();
            for _ in sy..ey {
                // SAFETY: sy..ey and sx..ex are verified to be within bounds
                unsafe {
                    slice.get_unchecked_mut(offset..offset + len).fill(color);
                }
                offset += w;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let fb = Framebuffer::new(100, 200).expect("Should create valid buffer");
        assert_eq!(fb.width(), 100);
        assert_eq!(fb.height(), 200);
        assert_eq!(fb.as_slice().len(), 20000);
        // Initial state should be black with full alpha
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
    }

    #[test]
    fn test_new_overflow_dimensions() {
        assert!(Framebuffer::new(i32::MAX as u32 + 1, 10).is_err());
        assert!(Framebuffer::new(10, i32::MAX as u32 + 1).is_err());
        // 65536 * 65536 = 4294967296 (exceeds u32::MAX by 1)
        assert!(Framebuffer::new(65536, 65536).is_err());
    }

    #[test]
    fn test_set_get_pixel_in_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Default color
        assert_eq!(fb.get_pixel(5, 5), Some(0xFF00_0000));

        // Set and get
        fb.set_pixel(5, 5, 0xAABBCCDD);
        assert_eq!(fb.get_pixel(5, 5), Some(0xAABBCCDD));

        // Other pixels remain unchanged
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
    }

    #[test]
    fn test_set_get_pixel_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Should return None, no panic
        assert_eq!(fb.get_pixel(-1, 5), None);
        assert_eq!(fb.get_pixel(5, -1), None);
        assert_eq!(fb.get_pixel(10, 5), None);
        assert_eq!(fb.get_pixel(5, 10), None);

        // Setting out of bounds should be a no-op, no panic
        fb.set_pixel(-1, 5, 0xFFFFFFFF);
        fb.set_pixel(5, -1, 0xFFFFFFFF);
        fb.set_pixel(10, 5, 0xFFFFFFFF);
        fb.set_pixel(5, 10, 0xFFFFFFFF);

        // Everything should still be default
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFF00_0000));
            }
        }
    }

    #[test]
    fn test_clear_entire_buffer() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        fb.clear(0x12345678);

        for y in 0..5 {
            for x in 0..5 {
                assert_eq!(fb.get_pixel(x, y), Some(0x12345678));
            }
        }
    }

    #[test]
    fn test_clear_rect_within_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        fb.clear_rect(2, 2, 3, 3, 0xFFFFFFFF);

        for y in 0..10 {
            for x in 0..10 {
                let expected = if x >= 2 && x < 5 && y >= 2 && y < 5 {
                    0xFFFFFFFF
                } else {
                    0xFF00_0000
                };
                assert_eq!(fb.get_pixel(x, y), Some(expected), "Mismatch at {x}, {y}");
            }
        }
    }

    #[test]
    fn test_clear_rect_partial_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Start inside, extend outside
        fb.clear_rect(8, 8, 5, 5, 0xFFFFFFFF);

        for y in 0..10 {
            for x in 0..10 {
                let expected = if x >= 8 && y >= 8 {
                    0xFFFFFFFF
                } else {
                    0xFF00_0000
                };
                assert_eq!(fb.get_pixel(x, y), Some(expected), "Mismatch at {x}, {y}");
            }
        }
    }

    #[test]
    fn test_clear_rect_negative_coordinates() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Start outside (negative), extend inside
        fb.clear_rect(-2, -2, 5, 5, 0xFFFFFFFF);

        for y in 0..10 {
            for x in 0..10 {
                let expected = if x < 3 && y < 3 {
                    0xFFFFFFFF
                } else {
                    0xFF00_0000
                };
                assert_eq!(fb.get_pixel(x, y), Some(expected), "Mismatch at {x}, {y}");
            }
        }
    }

    #[test]
    fn test_clear_rect_fully_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Completely outside
        fb.clear_rect(15, 15, 5, 5, 0xFFFFFFFF);
        fb.clear_rect(-10, -10, 5, 5, 0xFFFFFFFF);

        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFF00_0000));
            }
        }
    }

    #[test]
    fn test_unsafe_set_get_pixel() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        unsafe {
            fb.set_pixel_unchecked(5, 5, 0xAABBCCDD);
            assert_eq!(fb.get_pixel_unchecked(5, 5), 0xAABBCCDD);
        }
    }

    #[test]
    fn test_clear_rect_full_width() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Clear full width but only a few rows
        fb.clear_rect(0, 2, 10, 3, 0xFFFFFFFF);

        for y in 0..10 {
            for x in 0..10 {
                let expected = if y >= 2 && y < 5 {
                    0xFFFFFFFF
                } else {
                    0xFF00_0000
                };
                assert_eq!(fb.get_pixel(x, y), Some(expected), "Mismatch at {x}, {y}");
            }
        }
    }
}

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

impl Framebuffer {
    /// Exports the framebuffer to a Portable Pixmap (PPM) file.
    ///
    /// PPM is a simple, uncompressed RGB format.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    pub fn export_ppm<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        if self.width() == 0 || self.height() == 0 {
            return Ok(());
        }

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write PPM header (P6 = binary RGB)
        writeln!(writer, "P6")?;
        writeln!(writer, "{} {}", self.width(), self.height())?;
        writeln!(writer, "255")?;

        // Write pixel data
        let pixels = self.as_slice();
        let mut row_buffer = vec![0u8; (self.width() * 3) as usize];

        for row in pixels
            .chunks_exact(self.width() as usize)
            .take(self.height() as usize)
        {
            // ⚡ Bolt: Eliminate dynamic `extend_from_slice` capacity checks by pre-allocating
            // a zeroed row buffer and writing directly via iterators to elide bounds checks.
            for (&pixel, out) in row.iter().zip(row_buffer.chunks_exact_mut(3)) {
                out[0] = ((pixel >> 16) & 0xFF) as u8;
                out[1] = ((pixel >> 8) & 0xFF) as u8;
                out[2] = (pixel & 0xFF) as u8;
            }
            writer.write_all(&row_buffer)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Exports the framebuffer to an uncompressed Truevision TGA file.
    ///
    /// TGA is a widely supported format that stores uncompressed RGB/RGBA data.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written to.
    pub fn export_tga<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        if self.width() == 0 || self.height() == 0 {
            return Ok(());
        }

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
        let mut row_buffer = vec![0u8; (self.width() * 3) as usize];

        for row in pixels
            .chunks_exact(self.width() as usize)
            .take(self.height() as usize)
        {
            // ⚡ Bolt: Eliminate dynamic `extend_from_slice` capacity checks by pre-allocating
            // a zeroed row buffer and writing directly via iterators to elide bounds checks.
            for (&pixel, out) in row.iter().zip(row_buffer.chunks_exact_mut(3)) {
                out[0] = (pixel & 0xFF) as u8;
                out[1] = ((pixel >> 8) & 0xFF) as u8;
                out[2] = ((pixel >> 16) & 0xFF) as u8;
            }
            writer.write_all(&row_buffer)?;
        }

        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod export_tests {
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
