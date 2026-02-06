//! Pixel buffer management.
//!
//! Framebuffer stores pixels in row-major order as 32-bit RGBA values.
//! Color format: 0xAARRGGBB (little-endian: BB GG RR AA in memory).

pub struct Framebuffer {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}

impl Framebuffer {
    /// Creates a new framebuffer with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions exceed `i32::MAX` or the total pixel count overflows `u32`.
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
    /// Caller must ensure x and y are within bounds.
    pub unsafe fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: u32) {
        let idx = y * self.width as usize + x;
        // SAFETY: Caller guarantees bounds
        unsafe {
            *self.pixels.get_unchecked_mut(idx) = color;
        }
    }

    /// Clear a rectangular region
    pub fn clear_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        let x = x.max(0) as u32;
        let y = y.max(0) as u32;
        let x_end = (x + width).min(self.width);
        let y_end = (y + height).min(self.height);

        for row in y..y_end {
            let start = (row * self.width + x) as usize;
            let end = (row * self.width + x_end) as usize;
            self.pixels[start..end].fill(color);
        }
    }
}
