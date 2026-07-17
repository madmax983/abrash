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

    /// Clear a specific sub-region of the z-buffer.
    ///
    /// Silently handles out-of-bounds coordinates by clipping the region.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::zbuffer::ZBuffer;
    ///
    /// let mut zb = ZBuffer::new(1920, 1080).unwrap();
    /// zb.clear_rect(100, 100, 500, 500); // Clears a 500x500 area
    /// ```
    pub fn clear_rect(&mut self, x: i32, y: i32, width: u32, height: u32) {
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

        let w = self.width as usize;
        let sx = start_x as usize;
        let ex = end_x as usize;

        let sy = start_y as usize;
        let ey = end_y as usize;

        let start_idx = sy * w;
        let end_idx = ey * w;

        if sx == 0 && ex == w {
            self.depths[start_idx..end_idx].fill(f32::INFINITY);
        } else {
            // Hot path optimization: process rows concurrently if large enough
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                // Only parallelize if the workload is large enough to overcome rayon's overhead
                let row_count = ey - sy;
                if row_count > 100 {
                    // ⚡ Bolt: Iterator-based chunking with unsafe inner-loop bounds elision improves SIMD alignment and performance.
                    self.depths[start_idx..end_idx]
                        .par_chunks_exact_mut(w)
                        .for_each(|row| unsafe {
                            row.get_unchecked_mut(sx..ex).fill(f32::INFINITY);
                        });
                    return;
                }
            }

            // ⚡ Bolt: Iterator-based chunking with unsafe inner-loop bounds elision improves SIMD alignment and performance.
            for row in self.depths[start_idx..end_idx].chunks_exact_mut(w) {
                unsafe {
                    row.get_unchecked_mut(sx..ex).fill(f32::INFINITY);
                }
            }
        }
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

    /// The width of the depth buffer in pixels.
    ///
    /// Useful for calculating normalized device coordinates (NDC) from screen space.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// The height of the depth buffer in pixels.
    ///
    /// Useful for calculating normalized device coordinates (NDC) from screen space.
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
    fn test_clear_rect() {
        let mut zb = ZBuffer::new(10, 10).unwrap();
        // Set all to 1.0
        for y in 0..10 {
            for x in 0..10 {
                zb.test_and_set(x, y, 1.0);
            }
        }

        // Clear a 5x5 area in the middle
        zb.clear_rect(2, 2, 5, 5);

        // Check bounds
        for y in 0..10 {
            for x in 0..10 {
                let depth = zb.get_depth(x, y).unwrap();
                if x >= 2 && x < 7 && y >= 2 && y < 7 {
                    assert!(depth.is_infinite());
                } else {
                    assert_eq!(depth, 1.0);
                }
            }
        }
    }

    #[test]
    fn test_clear_rect_oob() {
        let mut zb = ZBuffer::new(10, 10).unwrap();
        for y in 0..10 {
            for x in 0..10 {
                zb.test_and_set(x, y, 1.0);
            }
        }

        // Out of bounds start
        zb.clear_rect(-5, -5, 10, 10);
        // Only 0..5 should be cleared
        for y in 0..10 {
            for x in 0..10 {
                let depth = zb.get_depth(x, y).unwrap();
                if x < 5 && y < 5 {
                    assert!(depth.is_infinite());
                } else {
                    assert_eq!(depth, 1.0);
                }
            }
        }
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

    #[test]
    fn test_clear_rect_full_width() {
        let mut zb = ZBuffer::new(10, 10).unwrap();
        for y in 0..10 {
            for x in 0..10 {
                zb.test_and_set(x, y, 1.0);
            }
        }

        // Clear full width but only a few rows
        zb.clear_rect(0, 2, 10, 3);

        for y in 0..10 {
            for x in 0..10 {
                let depth = zb.get_depth(x, y).unwrap();
                if y >= 2 && y < 5 {
                    assert!(depth.is_infinite());
                } else {
                    assert_eq!(depth, 1.0);
                }
            }
        }
    }

    #[test]
    fn test_clear_rect_extreme_bounds() {
        let mut zb = ZBuffer::new(10, 10).unwrap();

        // Fill buffer to verify it clears correctly
        for y in 0..10 {
            for x in 0..10 {
                zb.test_and_set(x, y, 1.0);
            }
        }

        // This should safely clamp to the buffer dimensions without overflowing or panicking
        // due to the explicit integer casting protection inside clear_rect.
        zb.clear_rect(-500, -500, u32::MAX, u32::MAX);

        // The entire buffer should be cleared to infinity
        for y in 0..10 {
            for x in 0..10 {
                assert!(zb.get_depth(x, y).unwrap().is_infinite());
            }
        }
    }
}
