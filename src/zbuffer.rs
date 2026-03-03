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

    /// Clear the z-buffer to a specific value.
    /// Useful for benchmarking occlusion culling.
    pub fn clear_val(&mut self, val: f32) {
        self.depths.fill(val);
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
    #[must_use]
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
    /// Caller must ensure `x < width` and `y < height`.
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

    /// Returns the width of the depth buffer in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Returns the height of the depth buffer in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let zb = ZBuffer::new(100, 200).expect("Should create valid buffer");
        assert_eq!(zb.width(), 100);
        assert_eq!(zb.height(), 200);
        assert_eq!(zb.as_slice().len(), 20000);
        // Initial state should be infinity
        assert!(zb.get_depth(0, 0).unwrap().is_infinite());
    }

    #[test]
    fn test_new_overflow() {
        // Test dimensions exceeding i32::MAX
        assert!(ZBuffer::new(i32::MAX as u32 + 1, 10).is_err());
        assert!(ZBuffer::new(10, i32::MAX as u32 + 1).is_err());

        // Test total size overflow (u32::MAX pixels)
        // 65536 * 65536 = 4294967296 (exceeds u32::MAX by 1)
        assert!(ZBuffer::new(65536, 65536).is_err());
    }

    #[test]
    fn test_clear() {
        let mut zb = ZBuffer::new(2, 2).unwrap();
        zb.test_and_set(0, 0, 1.0);
        assert_eq!(zb.get_depth(0, 0), Some(1.0));

        zb.clear();
        assert!(zb.get_depth(0, 0).unwrap().is_infinite());
    }

    #[test]
    fn test_test_and_set() {
        let mut zb = ZBuffer::new(2, 2).unwrap();

        // 1. Initial set (infinity -> 10.0) -> Pass
        assert!(zb.test_and_set(0, 0, 10.0));
        assert_eq!(zb.get_depth(0, 0), Some(10.0));

        // 2. Set closer (10.0 -> 5.0) -> Pass
        assert!(zb.test_and_set(0, 0, 5.0));
        assert_eq!(zb.get_depth(0, 0), Some(5.0));

        // 3. Set further (5.0 -> 8.0) -> Fail
        assert!(!zb.test_and_set(0, 0, 8.0));
        assert_eq!(zb.get_depth(0, 0), Some(5.0));

        // 4. Set equal (5.0 -> 5.0) -> Fail (strict inequality)
        assert!(!zb.test_and_set(0, 0, 5.0));
        assert_eq!(zb.get_depth(0, 0), Some(5.0));

        // 5. Boundary checks
        assert!(!zb.test_and_set(-1, 0, 1.0));
        assert!(!zb.test_and_set(0, -1, 1.0));
        assert!(!zb.test_and_set(2, 0, 1.0)); // Width is 2, so index 2 is OOB
        assert!(!zb.test_and_set(0, 2, 1.0));
    }

    #[test]
    fn test_get_depth() {
        let mut zb = ZBuffer::new(10, 10).unwrap();
        zb.test_and_set(5, 5, 0.5);

        assert_eq!(zb.get_depth(5, 5), Some(0.5));
        assert!(zb.get_depth(0, 0).unwrap().is_infinite());

        assert_eq!(zb.get_depth(-1, 0), None);
        assert_eq!(zb.get_depth(10, 0), None);
    }

    #[test]
    fn test_test_and_set_unchecked() {
        let mut zb = ZBuffer::new(2, 2).unwrap();
        unsafe {
            // Should pass
            assert!(zb.test_and_set_unchecked(0, 0, 10.0));
            assert_eq!(zb.get_depth(0, 0), Some(10.0));

            // Should fail
            assert!(!zb.test_and_set_unchecked(0, 0, 15.0));
            assert_eq!(zb.get_depth(0, 0), Some(10.0));
        }
    }

    #[test]
    fn test_as_slice_mut() {
        let mut zb = ZBuffer::new(2, 1).unwrap();
        let slice = zb.as_mut_slice();
        slice[0] = 0.1;
        slice[1] = 0.2;

        assert_eq!(zb.get_depth(0, 0), Some(0.1));
        assert_eq!(zb.get_depth(1, 0), Some(0.2));
    }
}
