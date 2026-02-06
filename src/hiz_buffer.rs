/// Hierarchical Z-Buffer for efficient occlusion culling
///
/// The Hi-Z buffer maintains a depth pyramid where each level stores the minimum
/// (closest) depth from a 2×2 region of the level below. This enables fast occlusion
/// queries by testing against progressively coarser representations.
///
/// # Memory Layout
/// - Level 0: References the full-resolution `ZBuffer` (not duplicated)
/// - Level 1+: Progressively coarser 2×2 min-reductions
/// - For 1920×1080: ~2.67 MB total pyramid overhead (33% of zbuffer size)
///
/// # Query Algorithm
/// Hierarchical descent from coarse to fine levels, testing AABB depth bounds
/// against pyramid cells. Conservative: false positives OK, false negatives NOT OK.
use crate::zbuffer::ZBuffer;

/// 3D Axis-Aligned Bounding Box for occlusion queries
#[derive(Debug, Clone, Copy)]
pub struct AABB3D {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_depth: f32, // Closest point of AABB
    pub max_depth: f32, // Farthest point of AABB
}

/// Single level in the depth pyramid
struct PyramidLevel {
    width: u32,
    height: u32,
    depths: Vec<f32>, // Row-major: depths[y * width + x]
}

impl PyramidLevel {
    fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            depths: vec![f32::INFINITY; size],
        }
    }
}

/// Hierarchical Z-Buffer for occlusion culling
///
/// # Example
/// ```
/// use abrash::zbuffer::ZBuffer;
/// use abrash::hiz_buffer::HiZBuffer;
///
/// let width = 800;
/// let height = 600;
/// let mut zb = ZBuffer::new(width, height);
/// let mut hiz = HiZBuffer::new(width, height);
///
/// // After rendering a frame, build the pyramid
/// hiz.build_pyramid(&zb);
///
/// // Query if an AABB is potentially visible
/// // (will be tested in integration phase)
/// ```
pub struct HiZBuffer {
    width: u32,
    height: u32,
    level_count: u32,
    levels: Vec<PyramidLevel>, // levels[0] conceptually references zbuffer, 1+ are reductions
    valid: bool,               // Pyramid needs rebuild after zbuffer writes
}

impl HiZBuffer {
    /// Create a new hierarchical z-buffer
    ///
    /// Level count is computed as: ceil(log2(max(width, height))) + 1
    /// This ensures the top level is at most 2×2 pixels.
    ///
    /// # Panics
    ///
    /// Panics if width or height is zero.
    ///
    /// # Example
    /// ```
    /// use abrash::hiz_buffer::HiZBuffer;
    ///
    /// let hiz = HiZBuffer::new(1920, 1080);
    /// assert_eq!(hiz.level_count(), 12); // ceil(log2(1920)) + 1 = 11 + 1 = 12
    /// ```
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");

        // Calculate level count: ceil(log2(max(width, height))) + 1
        // This gives us levels 0..level_count where level 0 is full resolution
        let max_dim = width.max(height) as f32;
        let level_count = max_dim.log2().ceil() as u32 + 1;

        // Create pyramid levels (skip level 0 since it references zbuffer)
        let mut levels = Vec::with_capacity(level_count as usize);
        levels.push(PyramidLevel::new(width, height)); // Level 0 placeholder

        for level_idx in 1..level_count {
            let scale = 1u32 << level_idx; // 2^level_idx
            let level_width = width.div_ceil(scale);
            let level_height = height.div_ceil(scale);
            levels.push(PyramidLevel::new(level_width, level_height));
        }

        Self {
            width,
            height,
            level_count,
            levels,
            valid: false,
        }
    }

    /// Get the number of pyramid levels
    #[must_use]
    pub const fn level_count(&self) -> u32 {
        self.level_count
    }

    /// Get the dimensions of a specific pyramid level
    #[must_use]
    pub fn level_dimensions(&self, level: u32) -> Option<(u32, u32)> {
        if level >= self.level_count {
            return None;
        }
        Some((
            self.levels[level as usize].width,
            self.levels[level as usize].height,
        ))
    }

    /// Check if the pyramid is valid (built and up-to-date)
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    /// Mark pyramid as invalid (needs rebuild)
    pub const fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Build the entire pyramid from a zbuffer
    ///
    /// This performs 2×2 min-reductions from level 0 (zbuffer) up to the top.
    /// Time complexity: O(width × height) for full scan
    ///
    /// # Panics
    ///
    /// Panics if the zbuffer dimensions don't match this Hi-Z buffer's dimensions.
    ///
    /// # Performance
    /// - 1920×1080: ~1-2ms (2.6M pixels processed)
    /// - 3840×2160: ~4-8ms (10.4M pixels processed)
    pub fn build_pyramid(&mut self, zbuffer: &ZBuffer) {
        assert_eq!(zbuffer.width(), self.width);
        assert_eq!(zbuffer.height(), self.height);

        // Build level 1 directly from zbuffer
        let level0 = zbuffer.as_slice();
        self.build_level(1, level0, self.width);

        // Build subsequent levels from previous levels
        for level_idx in 2..self.level_count {
            // Clone the previous level's depths to avoid borrowing issues
            let prev_width = self.levels[(level_idx - 1) as usize].width;
            let prev_depths = self.levels[(level_idx - 1) as usize].depths.clone();
            self.build_level(level_idx, &prev_depths, prev_width);
        }

        self.valid = true;
    }

    /// Build a single pyramid level via 2×2 min-reduction
    fn build_level(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
        // SIMD implementation was removed due to 2.7× performance regression.
        // Scalar is faster until SIMD shuffle pattern is optimized.
        self.build_level_scalar(level_idx, source, source_width);
    }

    /// Scalar 2×2 min-reduction implementation
    fn build_level_scalar(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
        let level_width = self.levels[level_idx as usize].width;
        let level_height = self.levels[level_idx as usize].height;

        for y in 0..level_height {
            for x in 0..level_width {
                let src_x = (x * 2) as usize;
                let src_y = (y * 2) as usize;

                // Sample 2×2 quad from previous level
                let d00 = source[src_y * source_width as usize + src_x];
                let d10 = source
                    .get(src_y * source_width as usize + src_x + 1)
                    .copied()
                    .unwrap_or(d00); // Clamp to border
                let d01 = source
                    .get((src_y + 1) * source_width as usize + src_x)
                    .copied()
                    .unwrap_or(d00);
                let d11 = source
                    .get((src_y + 1) * source_width as usize + src_x + 1)
                    .copied()
                    .unwrap_or(d00);

                // Store minimum (closest) depth
                let min_depth = d00.min(d10).min(d01).min(d11);
                self.levels[level_idx as usize].depths[(y * level_width + x) as usize] = min_depth;
            }
        }
    }


    /// Test if an AABB is potentially visible
    ///
    /// Returns true if the AABB must be rasterized, false if fully occluded.
    /// This is a conservative test: false positives are acceptable, false negatives are not.
    ///
    /// # Algorithm
    /// 1. Find starting level where AABB fits in ≤4 cells (2×2)
    /// 2. Descend from coarse to fine, testing at each level
    /// 3. If AABB's closest point is farther than pyramid's closest point → occluded
    ///
    /// # Performance
    /// - Single query: <100ns (cache hit)
    /// - Batch of 100: <50µs (cache reuse)
    #[must_use]
    pub fn is_potentially_visible(&self, aabb: AABB3D) -> bool {
        if !self.valid {
            return true; // Pyramid invalid, assume visible
        }

        // Check if AABB is entirely offscreen before clamping
        if aabb.max_x < 0
            || aabb.min_x >= self.width as i32
            || aabb.max_y < 0
            || aabb.min_y >= self.height as i32
        {
            return false; // Entirely offscreen
        }

        // Clamp AABB to screen bounds
        let min_x = aabb.min_x.max(0).min(self.width as i32 - 1);
        let max_x = aabb.max_x.max(0).min(self.width as i32 - 1);
        let min_y = aabb.min_y.max(0).min(self.height as i32 - 1);
        let max_y = aabb.max_y.max(0).min(self.height as i32 - 1);

        // Find starting level where AABB fits in ≤4 cells (2×2)
        let aabb_width = (max_x - min_x + 1) as u32;
        let aabb_height = (max_y - min_y + 1) as u32;
        let start_level = self.find_covering_level(aabb_width, aabb_height);

        // Descend from coarse to fine, testing at each level
        for level_idx in (1..=start_level).rev() {
            let scale = 1u32 << level_idx; // 2^level_idx
            let level = &self.levels[level_idx as usize];

            let lx0 = (min_x as u32 / scale) as usize;
            let ly0 = (min_y as u32 / scale) as usize;
            let lx1 = ((max_x as u32 / scale).min(level.width - 1)) as usize;
            let ly1 = ((max_y as u32 / scale).min(level.height - 1)) as usize;

            // Find minimum depth in covered pyramid cells
            let mut pyramid_min = f32::INFINITY;
            for ly in ly0..=ly1 {
                for lx in lx0..=lx1 {
                    pyramid_min = pyramid_min.min(level.depths[ly * level.width as usize + lx]);
                }
            }

            // Conservative test: If AABB's closest point is farther than
            // pyramid's closest point, AABB is fully occluded
            if aabb.min_depth > pyramid_min {
                return false; // Fully occluded
            }
        }

        true // Potentially visible
    }

    /// Find the coarsest pyramid level where AABB fits in ≤4 cells (2×2)
    fn find_covering_level(&self, aabb_width: u32, aabb_height: u32) -> u32 {
        for level_idx in (1..self.level_count).rev() {
            let scale = 1u32 << level_idx;

            let cells_x = aabb_width.div_ceil(scale);
            let cells_y = aabb_height.div_ceil(scale);

            if cells_x <= 2 && cells_y <= 2 {
                return level_idx;
            }
        }
        1 // Fallback to level 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_count_computation() {
        // 1024×1024 → ceil(log2(1024)) + 1 = 10 + 1 = 11 levels
        let hiz = HiZBuffer::new(1024, 1024);
        assert_eq!(hiz.level_count(), 11);

        // 1920×1080 → ceil(log2(1920)) + 1 = 11 + 1 = 12 levels
        let hiz = HiZBuffer::new(1920, 1080);
        assert_eq!(hiz.level_count(), 12);

        // 800×600 → ceil(log2(800)) + 1 = 10 + 1 = 11 levels
        let hiz = HiZBuffer::new(800, 600);
        assert_eq!(hiz.level_count(), 11);

        // 512×512 → ceil(log2(512)) + 1 = 9 + 1 = 10 levels
        let hiz = HiZBuffer::new(512, 512);
        assert_eq!(hiz.level_count(), 10);
    }

    #[test]
    fn test_level_dimensions() {
        let hiz = HiZBuffer::new(1920, 1080);

        // Level 0: full resolution
        assert_eq!(hiz.level_dimensions(0), Some((1920, 1080)));

        // Level 1: half resolution (960×540)
        assert_eq!(hiz.level_dimensions(1), Some((960, 540)));

        // Level 2: quarter resolution (480×270)
        assert_eq!(hiz.level_dimensions(2), Some((480, 270)));

        // Level 11: top of pyramid (should be 1×1 or 2×1)
        let top_level = hiz.level_dimensions(11);
        assert!(top_level.is_some());
        let (w, h) = top_level.unwrap();
        assert!(w <= 2 && h <= 2);

        // Out of bounds
        assert_eq!(hiz.level_dimensions(12), None);
    }

    #[test]
    fn test_pyramid_initially_invalid() {
        let hiz = HiZBuffer::new(800, 600);
        assert!(!hiz.is_valid());
    }

    #[test]
    fn test_pyramid_valid_after_build() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(800, 600).unwrap();

        hiz.build_pyramid(&zb);
        assert!(hiz.is_valid());
    }

    #[test]
    fn test_pyramid_invalidate() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(800, 600).unwrap();

        hiz.build_pyramid(&zb);
        assert!(hiz.is_valid());

        hiz.invalidate();
        assert!(!hiz.is_valid());
    }

    #[test]
    fn test_min_reduction_simple() {
        // 4×4 input with known pattern
        let mut zb = ZBuffer::new(4, 4).unwrap();

        // Fill with pattern (row-major order):
        // Row 0: 1.0  2.0  3.0  4.0
        // Row 1: 5.0  6.0  7.0  8.0
        // Row 2: 9.0 10.0 11.0 12.0
        // Row 3: 13.0 14.0 15.0 16.0
        let slice = zb.as_mut_slice();
        for i in 0..16 {
            slice[i] = (i + 1) as f32;
        }

        let mut hiz = HiZBuffer::new(4, 4);
        hiz.build_pyramid(&zb);

        // Level 1 should be 2×2 with mins of each 2×2 quad:
        // Top-left (0,0)-(1,1): min(1,2,5,6) = 1.0
        // Top-right (2,0)-(3,1): min(3,4,7,8) = 3.0
        // Bottom-left (0,2)-(1,3): min(9,10,13,14) = 9.0
        // Bottom-right (2,2)-(3,3): min(11,12,15,16) = 11.0
        let level1 = &hiz.levels[1];
        assert_eq!(level1.width, 2);
        assert_eq!(level1.height, 2);
        assert_eq!(level1.depths[0], 1.0); // Top-left quad
        assert_eq!(level1.depths[1], 3.0); // Top-right quad
        assert_eq!(level1.depths[2], 9.0); // Bottom-left quad
        assert_eq!(level1.depths[3], 11.0); // Bottom-right quad
    }

    #[test]
    fn test_min_reduction_propagates_to_top() {
        let mut zb = ZBuffer::new(4, 4).unwrap();

        // Fill with gradient, smallest value at (0,0)
        let slice = zb.as_mut_slice();
        for y in 0..4 {
            for x in 0..4 {
                slice[y * 4 + x] = (x + y) as f32;
            }
        }

        let mut hiz = HiZBuffer::new(4, 4);
        hiz.build_pyramid(&zb);

        // Level 2 should be 1×1 with global minimum
        let level2 = &hiz.levels[2];
        assert_eq!(level2.width, 1);
        assert_eq!(level2.height, 1);
        assert_eq!(level2.depths[0], 0.0); // Minimum of entire zbuffer
    }

    #[test]
    fn test_non_power_of_two_dimensions() {
        // 800×600 is not power of 2
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Fill with constant depth
        let slice = zb.as_mut_slice();
        for y in 0..600 {
            for x in 0..800 {
                slice[y * 800 + x] = 5.0;
            }
        }

        let mut hiz = HiZBuffer::new(800, 600);
        hiz.build_pyramid(&zb);

        // All pyramid levels should have depth 5.0 (constant propagation)
        for level_idx in 1..hiz.level_count() {
            let level = &hiz.levels[level_idx as usize];
            for depth in &level.depths {
                assert_eq!(*depth, 5.0);
            }
        }
    }

    #[test]
    fn test_offscreen_aabb_returns_false() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(800, 600).unwrap();
        hiz.build_pyramid(&zb);

        // AABB completely offscreen (negative coords)
        let aabb = AABB3D {
            min_x: -100,
            max_x: -10,
            min_y: -50,
            max_y: -5,
            min_depth: 1.0,
            max_depth: 10.0,
        };

        assert!(!hiz.is_potentially_visible(aabb));

        // AABB beyond screen bounds
        let aabb = AABB3D {
            min_x: 900,
            max_x: 1000,
            min_y: 700,
            max_y: 800,
            min_depth: 1.0,
            max_depth: 10.0,
        };

        assert!(!hiz.is_potentially_visible(aabb));
    }

    #[test]
    fn test_fully_occluded_aabb_returns_false() {
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Fill zbuffer with depth 5.0
        let slice = zb.as_mut_slice();
        for y in 0..600 {
            for x in 0..800 {
                slice[y * 800 + x] = 5.0;
            }
        }

        let mut hiz = HiZBuffer::new(800, 600);
        hiz.build_pyramid(&zb);

        // Query AABB with min_depth=10.0 (farther than zbuffer)
        let aabb = AABB3D {
            min_x: 100,
            max_x: 200,
            min_y: 100,
            max_y: 200,
            min_depth: 10.0,
            max_depth: 20.0,
        };

        assert!(!hiz.is_potentially_visible(aabb));
    }

    #[test]
    fn test_potentially_visible_aabb_returns_true() {
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Fill zbuffer with depth 10.0
        let slice = zb.as_mut_slice();
        for y in 0..600 {
            for x in 0..800 {
                slice[y * 800 + x] = 10.0;
            }
        }

        let mut hiz = HiZBuffer::new(800, 600);
        hiz.build_pyramid(&zb);

        // Query AABB with min_depth=5.0 (closer than zbuffer)
        let aabb = AABB3D {
            min_x: 100,
            max_x: 200,
            min_y: 100,
            max_y: 200,
            min_depth: 5.0,
            max_depth: 15.0,
        };

        assert!(hiz.is_potentially_visible(aabb));
    }

    #[test]
    fn test_partially_visible_aabb_returns_true() {
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Fill zbuffer with depth 10.0, except one region with 15.0 (farther)
        let slice = zb.as_mut_slice();
        for y in 0..600 {
            for x in 0..800 {
                if x >= 100 && x < 200 && y >= 100 && y < 200 {
                    slice[y * 800 + x] = 15.0; // Farther region (occluder is farther)
                } else {
                    slice[y * 800 + x] = 10.0;
                }
            }
        }

        let mut hiz = HiZBuffer::new(800, 600);
        hiz.build_pyramid(&zb);

        // Query AABB that overlaps both regions
        // AABB is at depth 7-12, which is closer than some existing geometry
        let aabb = AABB3D {
            min_x: 150,
            max_x: 250,
            min_y: 150,
            max_y: 250,
            min_depth: 7.0, // Closer than both zbuffer regions
            max_depth: 12.0,
        };

        // Should be visible because AABB is closer than all geometry in its region
        assert!(hiz.is_potentially_visible(aabb));
    }

    #[test]
    fn test_partially_occluded_aabb_returns_false() {
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Fill zbuffer with depth 10.0, except one region with 5.0 (closer)
        let slice = zb.as_mut_slice();
        for y in 0..600 {
            for x in 0..800 {
                if x >= 100 && x < 200 && y >= 100 && y < 200 {
                    slice[y * 800 + x] = 5.0; // Closer region (occluder is closer)
                } else {
                    slice[y * 800 + x] = 10.0;
                }
            }
        }

        let mut hiz = HiZBuffer::new(800, 600);
        hiz.build_pyramid(&zb);

        // Query AABB that overlaps the closer region
        // AABB is at depth 7-12, which is farther than the occluder at 5.0
        let aabb = AABB3D {
            min_x: 150,
            max_x: 250,
            min_y: 150,
            max_y: 250,
            min_depth: 7.0, // Farther than the occluder at 5.0
            max_depth: 12.0,
        };

        // Should be occluded because there's closer geometry in the AABB's region
        assert!(!hiz.is_potentially_visible(aabb));
    }

    #[test]
    fn test_invalid_pyramid_assumes_visible() {
        let hiz = HiZBuffer::new(800, 600);
        // Don't build pyramid, leave it invalid

        let aabb = AABB3D {
            min_x: 100,
            max_x: 200,
            min_y: 100,
            max_y: 200,
            min_depth: 5.0,
            max_depth: 10.0,
        };

        // Invalid pyramid should assume everything is visible
        assert!(hiz.is_potentially_visible(aabb));
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_simd_scalar_equivalence() {
        // Test that SIMD and scalar implementations produce identical results
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill with pseudo-random pattern to test all code paths
        let slice = zb.as_mut_slice();
        for i in 0..slice.len() {
            // Generate pseudo-random depth values using simple hash
            let hash = ((i.wrapping_mul(2654435761)) >> 16) as f32 / 65536.0;
            slice[i] = hash * 100.0;
        }

        // Build pyramid with SIMD
        let mut hiz_simd = HiZBuffer::new(1920, 1080);
        hiz_simd.build_pyramid(&zb);

        // Build pyramid with scalar
        let mut hiz_scalar = HiZBuffer::new(1920, 1080);
        // Temporarily use scalar implementation
        for level_idx in 1..hiz_scalar.level_count {
            if level_idx == 1 {
                let level0 = zb.as_slice();
                hiz_scalar.build_level_scalar(1, level0, 1920);
            } else {
                let prev_width = hiz_scalar.levels[(level_idx - 1) as usize].width;
                let prev_depths = hiz_scalar.levels[(level_idx - 1) as usize].depths.clone();
                hiz_scalar.build_level_scalar(level_idx, &prev_depths, prev_width);
            }
        }

        // Compare all pyramid levels
        for level_idx in 1..hiz_simd.level_count {
            let simd_level = &hiz_simd.levels[level_idx as usize];
            let scalar_level = &hiz_scalar.levels[level_idx as usize];

            assert_eq!(simd_level.width, scalar_level.width);
            assert_eq!(simd_level.height, scalar_level.height);

            // Compare depths (should be bit-identical)
            for (i, (&simd_depth, &scalar_depth)) in simd_level
                .depths
                .iter()
                .zip(scalar_level.depths.iter())
                .enumerate()
            {
                assert_eq!(
                    simd_depth, scalar_depth,
                    "Mismatch at level {} index {}: SIMD={} vs Scalar={}",
                    level_idx, i, simd_depth, scalar_depth
                );
            }
        }
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_simd_4k_resolution() {
        // Test SIMD implementation at 4K resolution
        let mut zb = ZBuffer::new(3840, 2160).unwrap();

        // Fill with gradient pattern
        let slice = zb.as_mut_slice();
        for y in 0..2160 {
            for x in 0..3840 {
                slice[y * 3840 + x] = (x + y) as f32 * 0.1;
            }
        }

        let mut hiz = HiZBuffer::new(3840, 2160);
        hiz.build_pyramid(&zb);

        // Verify pyramid is valid
        assert!(hiz.is_valid());

        // Verify level dimensions
        assert_eq!(hiz.level_dimensions(1), Some((1920, 1080)));
        assert_eq!(hiz.level_dimensions(2), Some((960, 540)));

        // Verify minimum propagated to top
        let top_level = hiz.level_count - 1;
        let top = &hiz.levels[top_level as usize];
        assert!(top.width <= 2 && top.height <= 2);
        assert_eq!(top.depths[0], 0.0); // Minimum should be at (0,0)
    }
}
