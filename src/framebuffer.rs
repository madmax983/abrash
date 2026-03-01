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
    /// use abrash::framebuffer::Framebuffer;
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

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

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
    /// use abrash::framebuffer::Framebuffer;
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
    /// use abrash::framebuffer::Framebuffer;
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
    /// use abrash::framebuffer::Framebuffer;
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

    /// Clear a rectangular region
    pub fn clear_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let x1 = x;
        let y1 = y;

        // Prevent overflow using saturating add, but cap at i32::MAX.
        // Convert width to i64 to avoid overflow when adding to x
        let x2 = (x as i64 + width as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let y2 = (y as i64 + height as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        let start_x = x1.clamp(0, self.width as i32) as u32;
        let start_y = y1.clamp(0, self.height as i32) as u32;
        let end_x = x2.clamp(0, self.width as i32) as u32;
        let end_y = y2.clamp(0, self.height as i32) as u32;

        for row in start_y..end_y {
            let start = (row * self.width + start_x) as usize;
            let end = (row * self.width + end_x) as usize;
            if start <= end && end <= self.pixels.len() {
                self.pixels[start..end].fill(color);
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
                assert_eq!(
                    fb.get_pixel(x, y),
                    Some(expected),
                    "Mismatch at {}, {}",
                    x,
                    y
                );
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
                assert_eq!(
                    fb.get_pixel(x, y),
                    Some(expected),
                    "Mismatch at {}, {}",
                    x,
                    y
                );
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
                assert_eq!(
                    fb.get_pixel(x, y),
                    Some(expected),
                    "Mismatch at {}, {}",
                    x,
                    y
                );
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
}
