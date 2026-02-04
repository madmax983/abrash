//! Depth buffer for hidden surface removal.
//!
//! Stores depth values per pixel for proper 3D occlusion.

/// Z-buffer for depth testing
pub struct ZBuffer {
    depths: Vec<f32>,
    width: u32,
    height: u32,
}

impl ZBuffer {
    /// Create a new z-buffer initialized to maximum depth
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::zbuffer::ZBuffer;
    ///
    /// let zb = ZBuffer::new(800, 600).unwrap();
    /// assert_eq!(zb.width(), 800);
    /// ```
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        let size = (width as u64)
            .checked_mul(height as u64)
            .filter(|&s| s <= u32::MAX as u64)
            .ok_or("Buffer size overflow")? as usize;

        Ok(Self {
            depths: vec![f32::INFINITY; size],
            width,
            height,
        })
    }

    /// Clear the z-buffer to maximum depth
    pub fn clear(&mut self) {
        self.depths.fill(f32::INFINITY);
    }

    /// Test and set depth at pixel. Returns true if pixel should be drawn.
    ///
    /// Returns `true` if the new depth is closer (less than) the existing depth,
    /// updating the buffer. Returns `false` otherwise or if coordinates are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::zbuffer::ZBuffer;
    ///
    /// let mut zb = ZBuffer::new(100, 100).unwrap();
    ///
    /// // First draw (depth 5.0) - should succeed
    /// assert!(zb.test_and_set(50, 50, 5.0));
    ///
    /// // Further draw (depth 10.0) - should fail
    /// assert!(!zb.test_and_set(50, 50, 10.0));
    ///
    /// // Closer draw (depth 2.0) - should succeed
    /// assert!(zb.test_and_set(50, 50, 2.0));
    /// ```
    pub fn test_and_set(&mut self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }

        let idx = (y as u32 * self.width + x as u32) as usize;
        if depth < self.depths[idx] {
            self.depths[idx] = depth;
            true
        } else {
            false
        }
    }

    /// Get depth at pixel
    pub fn get_depth(&self, x: i32, y: i32) -> Option<f32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        Some(self.depths[idx])
    }

    /// Get the raw depth buffer as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.depths
    }

    /// Get the raw depth buffer as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.depths
    }

    /// Test and set depth at pixel without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure x and y are within bounds.
    pub unsafe fn test_and_set_unchecked(&mut self, x: usize, y: usize, depth: f32) -> bool {
        let idx = y * self.width as usize + x;
        // SAFETY: Caller guarantees bounds
        let d = unsafe { self.depths.get_unchecked_mut(idx) };
        if depth < *d {
            *d = depth;
            true
        } else {
            false
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
}
