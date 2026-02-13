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

/// Maximum supported dimension for a framebuffer (16K resolution).
pub const MAX_DIMENSION: u32 = 16384;

impl Framebuffer {
    /// Creates a new framebuffer with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// *   Dimensions exceed `MAX_DIMENSION` (16384).
    /// *   The total pixel count overflows `u32` (preventing huge allocations).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::framebuffer::Framebuffer;
    /// let fb = Framebuffer::new(800, 600).unwrap();
    /// ```
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err("Buffer dimensions too large (max 16384)");
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
        let x = x.max(0) as u32;
        let y = y.max(0) as u32;

        if x >= self.width || y >= self.height {
            return;
        }

        let x_end = x.saturating_add(width).min(self.width);
        let y_end = y.saturating_add(height).min(self.height);

        for row in y..y_end {
            let start = (row * self.width + x) as usize;
            let end = (row * self.width + x_end) as usize;
            self.pixels[start..end].fill(color);
        }
    }
}
