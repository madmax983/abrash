//! Depth buffer for hidden surface removal.
//!
//! Stores depth values per pixel for proper 3D occlusion.
//!
//! # Depth Convention
//!
//! *   **Smaller Z** is closer to the camera.
//! *   **Larger Z** is further away.
//! *   The buffer is initialized to `f32::INFINITY`.

/// Z-buffer for depth testing
pub struct ZBuffer {
    depths: Vec<f32>,
    width: u32,
    height: u32,
}

impl ZBuffer {
    /// Create a new z-buffer initialized to maximum depth.
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
    #[inline]
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

    /// Get depth at pixel without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure x and y are within bounds.
    #[inline]
    pub unsafe fn get_depth_unchecked(&self, x: usize, y: usize) -> f32 {
        let idx = y * self.width as usize + x;
        // SAFETY: Caller guarantees bounds
        unsafe { *self.depths.get_unchecked(idx) }
    }

    /// Get depth at pixel
    #[inline]
    #[must_use]
    pub fn get_depth(&self, x: i32, y: i32) -> Option<f32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        Some(self.depths[idx])
    }

    /// Get the raw depth buffer as a slice.
    #[must_use]
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

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}
