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
    /// Creates a new Framebuffer with the specified dimensions.
    ///
    /// The buffer is initialized to opaque black (`0xFF000000`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::framebuffer::Framebuffer;
    /// let fb = Framebuffer::new(800, 600);
    /// assert_eq!(fb.width(), 800);
    /// ```
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            pixels: vec![0xFF00_0000; size], // Black with full alpha
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    /// Clears the entire framebuffer with a single color.
    ///
    /// # Arguments
    ///
    /// * `color` - The 32-bit color (`0xAARRGGBB`) to fill the buffer with.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::framebuffer::Framebuffer;
    /// let mut fb = Framebuffer::new(100, 100);
    /// fb.clear(0xFFFF0000); // Red
    /// ```
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Sets a pixel at (x, y) to the specified color.
    ///
    /// Performs bounds checking. If coordinates are out of bounds, the operation is ignored.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (0 is left).
    /// * `y` - Y coordinate (0 is top).
    /// * `color` - The 32-bit color (`0xAARRGGBB`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::framebuffer::Framebuffer;
    /// let mut fb = Framebuffer::new(100, 100);
    /// fb.set_pixel(50, 50, 0xFFFFFFFF); // White dot in center
    /// ```
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return; // Bounds check - silently ignore out of bounds
        }

        let index = (y as u32 * self.width + x as u32) as usize;
        self.pixels[index] = color;
    }

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
