//! # The Hierarchical Z-Buffer (Hi-Z) 🏔️
//!
//! Welcome to the heights of occlusion culling! When rendering complex scenes, drawing geometry
//! that is eventually hidden behind other objects (overdraw) is a massive performance killer.
//! The `HiZBuffer` acts as a sentinel, rapidly determining if a piece of geometry is completely hidden
//! before the rasterizer wastes time drawing it.
//!
//! Think of it as a mipmapped pyramid of depth values. Instead of checking every single pixel of a bounding box
//! against the full-resolution depth buffer, we can test it against a coarse summary. If the closest point of
//! the bounding box is *farther* away than the farthest geometry in a coarse region, we know the entire box
//! (and the geometry inside it) is completely occluded.
//!
//! ## The Algorithm
//! 1. **Build:** After rendering a base pass (or using depth from the previous frame), we build a pyramid
//!    where each level stores the maximum (farthest) depth from a 2×2 region of the level below it.
//!    The maximum is the only sound aggregate for a whole-box cull: the box is hidden only if its
//!    closest point is farther than *every* stored depth under its footprint, i.e. farther than the
//!    maximum. (An earlier version reduced with the minimum and could false-cull boxes whose
//!    footprint covered cells with differing depths.)
//! 2. **Query:** When testing an `AABB3D`, we find the pyramid level where the box covers roughly 2×2 pixels.
//!    We compare the box's closest depth against the maximum of those cells.
//! 3. **Result:** If the box is behind the known depth, it is discarded. This process is conservative:
//!    false positives (drawing something that ends up hidden) are acceptable, but false negatives are forbidden.
//!
//! ## Examples
//!
//! ```rust
//! use abrash_core::zbuffer::ZBuffer;
//! use abrash_core::hiz_buffer::{HiZBuffer, AABB3D};
//!
//! // 1. Initialize buffers for our viewport
//! let width = 1920;
//! let height = 1080;
//! let mut zb = ZBuffer::new(width, height).unwrap();
//! let mut hiz = HiZBuffer::new(width, height);
//!
//! // 2. Build the pyramid (usually done after a pre-pass or using previous frame data)
//! hiz.build_pyramid(&zb);
//!
//! // 3. Create a bounding box for an object we want to draw
//! let my_object_aabb = AABB3D::new(100, 200, 100, 200, 50.0, 60.0);
//!
//! // 4. Ask the sentinel!
//! if hiz.is_potentially_visible(my_object_aabb) {
//!     // Draw the object, it might be seen!
//! } else {
//!     // Skip drawing, it's definitely hidden behind a wall.
//! }
//! ```
//!
//! ## Performance Characteristics
//! * **Memory:** ~33% overhead of the base `ZBuffer` size (e.g., ~2.67 MB for 1080p).
//! * **Build Time:** ~1-2ms on CPU at 1080p.
//! * **Query Time:** <100ns per object.
//!

use crate::zbuffer::ZBuffer;

/// A 3D Axis-Aligned Bounding Box (AABB) in screen space.
///
/// This structure defines a volume in the screen's coordinate system, used primarily for
/// occlusion queries against the [`HiZBuffer`].
///
/// By knowing the bounding volume of an object *before* we draw it, we can ask the `HiZBuffer`
/// if this entire volume is hidden behind existing geometry.
///
/// ## Examples
/// ```
/// use abrash_core::hiz_buffer::AABB3D;
///
/// // A box spanning pixels (10, 10) to (50, 50) at a depth range of 10.0 to 20.0
/// let bounds = AABB3D::new(10, 50, 10, 50, 10.0, 20.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AABB3D {
    /// Minimum X coordinate in screen space
    pub min_x: i32,
    /// Maximum X coordinate in screen space
    pub max_x: i32,
    /// Minimum Y coordinate in screen space
    pub min_y: i32,
    /// Maximum Y coordinate in screen space
    pub max_y: i32,
    /// Minimum depth value (closest point to camera)
    pub min_depth: f32,
    /// Maximum depth value (farthest point from camera)
    pub max_depth: f32,
}

impl AABB3D {
    /// Creates a new AABB3D.
    ///
    /// The bounding box is defined by its screen space coordinates (`min_x`, `max_x`, `min_y`, `max_y`)
    /// and its depth bounds (`min_depth`, `max_depth`).
    #[must_use]
    pub const fn new(
        min_x: i32,
        max_x: i32,
        min_y: i32,
        max_y: i32,
        min_depth: f32,
        max_depth: f32,
    ) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            min_depth,
            max_depth,
        }
    }
}

/// Single level in the depth pyramid
struct PyramidLevel {
    width: u32,
    height: u32,
    depths: Vec<f32>, // Row-major: depths[y * width + x]
}

impl PyramidLevel {
    fn new(width: u32, height: u32) -> Self {
        // Prevent capacity overflow when allocating vec![f32::INFINITY; size]
        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .expect("Hi-Z dimensions overflow") as usize;
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
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::zbuffer::ZBuffer;
/// use abrash_core::hiz_buffer::HiZBuffer;
///
/// let width = 800;
/// let height = 600;
/// let mut zb = ZBuffer::new(width, height).unwrap();
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
    /// use abrash_core::hiz_buffer::HiZBuffer;
    ///
    /// let hiz = HiZBuffer::new(1920, 1080);
    /// assert_eq!(hiz.level_count(), 12); // ceil(log2(1920)) + 1 = 11 + 1 = 12
    /// ```
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");

        // Prevent huge allocations up front
        let _ = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .expect("capacity overflow");

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
    /// Gets the width and height of a specific pyramid level.
    ///
    /// Level 0 is the base full-resolution dimension. Each subsequent level is half the
    /// dimensions of the previous level, rounded up.
    ///
    /// Returns `None` if the requested level exceeds the [`level_count`](Self::level_count).
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
    /// This performs 2×2 max-reductions from level 0 (zbuffer) up to the top.
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
        self.build_pyramid_from_level0(zbuffer.as_slice());
    }

    /// Build the entire pyramid directly from a caller-owned depth slice.
    ///
    /// This avoids wrapping the depths in a temporary [`ZBuffer`] when the caller
    /// already owns the backing storage.
    ///
    /// # Panics
    ///
    /// Panics if the provided dimensions do not match this Hi-Z buffer or if the
    /// depth slice is smaller than `width * height`.
    pub fn build_pyramid_from_depths(&mut self, width: u32, height: u32, depths: &[f32]) {
        assert_eq!(width, self.width);
        assert_eq!(height, self.height);
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .expect("Hi-Z dimensions overflow");

        // WARDEN DEFENSE: Prevent capacity overflow panics
        assert!(
            expected_len <= (isize::MAX as usize) / 4,
            "Hi-Z dimensions overflow: capacity exceeded"
        );
        assert!(
            depths.len() >= expected_len,
            "Depth slice too small for Hi-Z pyramid build"
        );
        self.build_pyramid_from_level0(&depths[..expected_len]);
    }

    fn build_pyramid_from_level0(&mut self, level0: &[f32]) {
        if self.level_count > 1 {
            let dest_width = self.levels[1].width;
            let dest_height = self.levels[1].height;
            let dest = &mut self.levels[1].depths;
            Self::max_reduce_2x2(dest, dest_width, dest_height, level0, self.width);

            for level_idx in 2..self.level_count as usize {
                let (lower_levels, higher_levels) = self.levels.split_at_mut(level_idx);
                let prev_level = &lower_levels[level_idx - 1];
                let curr_level = &mut higher_levels[0];

                Self::max_reduce_2x2(
                    &mut curr_level.depths,
                    curr_level.width,
                    curr_level.height,
                    &prev_level.depths,
                    prev_level.width,
                );
            }
        }

        self.valid = true;
    }

    /// Optimized scalar 2×2 max-reduction implementation
    ///
    /// # SIMD Note
    /// SIMD was attempted but found to be slower due to memory bandwidth saturation,
    /// excessive shuffle operations for horizontal reduction, and small working sets.
    /// See `SIMD_PROFILING_ANALYSIS.md` for details.
    fn max_reduce_2x2(
        dest: &mut [f32],
        dest_width: u32,
        dest_height: u32,
        source: &[f32],
        source_width: u32,
    ) {
        let sw = source_width as usize;
        let sh = source.len() / sw;

        let odd_width = !source_width.is_multiple_of(2);
        let odd_height = !sh.is_multiple_of(2);

        let safe_width = if odd_width {
            dest_width - 1
        } else {
            dest_width
        };
        let safe_height = if odd_height {
            dest_height - 1
        } else {
            dest_height
        };

        for y in 0..safe_height {
            let src_y = (y * 2) as usize;
            let row0_start = src_y * sw;
            let row1_start = (src_y + 1) * sw;

            // Slice rows to avoid bounds checks in inner loop
            let row0 = &source[row0_start..];
            let row1 = &source[row1_start..];
            let dst_row = &mut dest[(y * dest_width) as usize..];

            for x in 0..safe_width {
                let sx = (x * 2) as usize;

                // Direct access: we know sx+1 is valid because x < safe_width
                let d00 = row0[sx];
                let d10 = row0[sx + 1];
                let d01 = row1[sx];
                let d11 = row1[sx + 1];

                dst_row[x as usize] = d00.max(d10).max(d01).max(d11);
            }

            // Handle last column if odd width
            if odd_width {
                let x = safe_width;
                let sx = (x * 2) as usize;
                let d00 = row0[sx];
                let d01 = row1[sx];
                // Clamp to left column
                dst_row[x as usize] = d00.max(d01);
            }
        }

        // Handle last row if odd height
        if odd_height {
            let y = safe_height;
            let src_y = (y * 2) as usize;
            let row0_start = src_y * sw;
            let row0 = &source[row0_start..];
            let dst_row = &mut dest[(y * dest_width) as usize..];

            for x in 0..safe_width {
                let sx = (x * 2) as usize;
                let d00 = row0[sx];
                let d10 = row0[sx + 1];
                // Clamp to top row
                dst_row[x as usize] = d00.max(d10);
            }

            if odd_width {
                let x = safe_width;
                let sx = (x * 2) as usize;
                let d00 = row0[sx];
                dst_row[x as usize] = d00;
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
    /// 3. If AABB's closest point is farther than the pyramid's farthest point
    ///    under the footprint → occluded
    ///
    /// # Performance
    /// - Single query: <100ns (cache hit)
    /// - Batch of 100: <50µs (cache reuse)
    /// Tests if a given 3D bounding box ([`AABB3D`]) might be visible on screen.
    ///
    /// This function performs a conservative occlusion query against the hierarchical depth pyramid.
    /// It determines the appropriate pyramid level based on the AABB size on-screen, and compares the
    /// closest depth of the AABB with the farthest depth known in that coarse region.
    ///
    /// - Returns `true` if the object **might** be visible (or if the pyramid is invalid).
    /// - Returns `false` if the object is **definitely** hidden behind existing geometry.
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

            // Find maximum depth in covered pyramid cells. The maximum is
            // what makes the cull sound: covered cells may extend past the
            // footprint, which can only inflate the maximum (fewer culls,
            // never false culls). Empty pixels store INFINITY, so any
            // footprint touching one has maximum INFINITY and is kept.
            let mut pyramid_max = f32::NEG_INFINITY;
            for ly in ly0..=ly1 {
                for lx in lx0..=lx1 {
                    pyramid_max = pyramid_max.max(level.depths[ly * level.width as usize + lx]);
                }
            }

            // Conservative test: the AABB is fully occluded only if its
            // closest point is farther than the farthest stored depth under
            // its footprint — every pixel in the footprint is in front of it.
            if aabb.min_depth > pyramid_max {
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

    /// Test if a coarse bin (128×128 pixels) is potentially visible
    ///
    /// This method is used for two-level hierarchical binning, where coarse bins
    /// (128×128 pixels) are first tested against the Hi-Z pyramid at level 2.
    /// Only visible coarse bins are subdivided into fine bins (32×32 pixels).
    ///
    /// # Algorithm
    /// 1. Convert bin AABB to pyramid level 2 coordinates (128×128 = 32 * 2^2)
    /// 2. Sample all pyramid cells covering the bin region
    /// 3. Find maximum depth across all samples
    /// 4. Compare bin's `min_depth` against the samples' maximum depth
    ///
    /// # Returns
    /// - `true` if the bin is potentially visible (must be subdivided)
    /// - `false` if the bin is fully occluded (skip subdivision)
    ///
    /// # Performance
    /// - Single query: <100ns (cache hit)
    /// - Typical scene: 90-95% coarse bins culled at high depth complexity
    ///
    /// # Example
    /// ```
    /// use abrash_core::hiz_buffer::{HiZBuffer, AABB3D};
    /// use abrash_core::zbuffer::ZBuffer;
    ///
    /// let mut hiz = HiZBuffer::new(1920, 1080);
    /// let zb = ZBuffer::new(1920, 1080).unwrap();
    /// hiz.build_pyramid(&zb);
    ///
    /// let bin_aabb = AABB3D {
    ///     min_x: 0,
    ///     max_x: 127,
    ///     min_y: 0,
    ///     max_y: 127,
    ///     min_depth: 10.0,
    ///     max_depth: 20.0,
    /// };
    ///
    /// if hiz.is_coarse_bin_visible(bin_aabb) {
    ///     // Subdivide into fine bins (32×32) and process
    /// }
    /// ```
    /// Tests if a coarse screen bin (128x128 pixels) is potentially visible.
    ///
    /// This is an optimization for two-level hierarchical binning. It quickly checks the
    /// Hi-Z pyramid at Level 2 to see if an entire coarse tile of the screen is completely occluded.
    /// If it is, the rasterizer can entirely skip processing fine bins (32x32 pixels) inside it.
    #[must_use]
    pub fn is_coarse_bin_visible(&self, bin_aabb: AABB3D) -> bool {
        const COARSE_BIN_LEVEL: u32 = 2;

        if !self.valid {
            return true; // Pyramid invalid, assume visible
        }

        // Check if bin is entirely offscreen before clamping
        if bin_aabb.max_x < 0
            || bin_aabb.min_x >= self.width as i32
            || bin_aabb.max_y < 0
            || bin_aabb.min_y >= self.height as i32
        {
            return false; // Entirely offscreen
        }

        // Clamp bin to screen bounds
        let min_x = bin_aabb.min_x.max(0).min(self.width as i32 - 1);
        let max_x = bin_aabb.max_x.max(0).min(self.width as i32 - 1);
        let min_y = bin_aabb.min_y.max(0).min(self.height as i32 - 1);
        let max_y = bin_aabb.max_y.max(0).min(self.height as i32 - 1);

        // Query pyramid at level 2 (128×128 = 32 * 2^2)
        // For 1920×1080: level 2 is 480×270 (each cell covers 4×4 pixels)
        let scale = 1u32 << COARSE_BIN_LEVEL; // 2^2 = 4

        // Ensure level 2 exists
        if COARSE_BIN_LEVEL >= self.level_count {
            // Pyramid not deep enough, fall back to is_potentially_visible
            return self.is_potentially_visible(bin_aabb);
        }

        let level = &self.levels[COARSE_BIN_LEVEL as usize];

        // Convert bin coordinates to pyramid level 2 coordinates
        let lx0 = (min_x as u32 / scale) as usize;
        let ly0 = (min_y as u32 / scale) as usize;
        let lx1 = ((max_x as u32 / scale).min(level.width - 1)) as usize;
        let ly1 = ((max_y as u32 / scale).min(level.height - 1)) as usize;

        // Find maximum depth in covered pyramid cells (see
        // `is_potentially_visible` for why the maximum is the sound
        // aggregate for a whole-region cull)
        let mut pyramid_max = f32::NEG_INFINITY;
        for ly in ly0..=ly1 {
            for lx in lx0..=lx1 {
                pyramid_max = pyramid_max.max(level.depths[ly * level.width as usize + lx]);
            }
        }

        // Conservative test: the bin is fully occluded only if its closest
        // point is farther than the farthest stored depth in its footprint
        bin_aabb.min_depth <= pyramid_max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Dimensions must be positive")]
    fn test_hiz_new_zero_width_panic() {
        let _ = HiZBuffer::new(0, 10);
    }

    #[test]
    #[should_panic(expected = "Dimensions must be positive")]
    fn test_hiz_new_zero_height_panic() {
        let _ = HiZBuffer::new(10, 0);
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn test_hiz_dimensions_overflow() {
        let _ = HiZBuffer::new(u32::MAX, u32::MAX);
    }

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
    #[should_panic(expected = "assertion `left == right` failed")]
    fn test_build_pyramid_dimensions_mismatch_width_panic() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(400, 600).unwrap();
        hiz.build_pyramid(&zb);
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn test_build_pyramid_dimensions_mismatch_height_panic() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(800, 300).unwrap();
        hiz.build_pyramid(&zb);
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn test_build_pyramid_from_depths_dimensions_mismatch_width_panic() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(400, 600).unwrap();
        hiz.build_pyramid_from_depths(400, 600, zb.as_slice());
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn test_build_pyramid_from_depths_dimensions_mismatch_height_panic() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(800, 300).unwrap();
        hiz.build_pyramid_from_depths(800, 300, zb.as_slice());
    }

    #[test]
    #[should_panic(expected = "Depth slice too small for Hi-Z pyramid build")]
    fn test_build_pyramid_from_depths_buffer_too_small_panic() {
        let mut hiz = HiZBuffer::new(800, 600);
        let zb = ZBuffer::new(800, 300).unwrap();
        // Passing 800x600 but a slice of 800x300 length
        hiz.build_pyramid_from_depths(800, 600, zb.as_slice());
    }

    #[test]
    fn test_build_pyramid_from_depths_matches_zbuffer_build() {
        let mut zb = ZBuffer::new(8, 8).unwrap();
        for (idx, depth) in zb.as_mut_slice().iter_mut().enumerate() {
            *depth = (idx as f32) * 0.25;
        }

        let mut from_zbuffer = HiZBuffer::new(8, 8);
        from_zbuffer.build_pyramid(&zb);

        let mut from_depths = HiZBuffer::new(8, 8);
        from_depths.build_pyramid_from_depths(8, 8, zb.as_slice());

        assert_eq!(from_depths.level_count(), from_zbuffer.level_count());
        for level_idx in 1..from_zbuffer.level_count() as usize {
            assert_eq!(
                from_depths.levels[level_idx].depths,
                from_zbuffer.levels[level_idx].depths
            );
        }
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
    fn test_max_reduction_simple() {
        // 4×4 input with known pattern
        let mut zb = ZBuffer::new(4, 4).unwrap();

        // Fill with pattern (row-major order):
        // Row 0: 1.0  2.0  3.0  4.0
        // Row 1: 5.0  6.0  7.0  8.0
        // Row 2: 9.0 10.0 11.0 12.0
        // Row 3: 13.0 14.0 15.0 16.0
        let slice = zb.as_mut_slice();
        for (i, d) in slice.iter_mut().enumerate().take(16) {
            *d = (i + 1) as f32;
        }

        let mut hiz = HiZBuffer::new(4, 4);
        hiz.build_pyramid(&zb);

        // Level 1 should be 2×2 with maxes of each 2×2 quad. (The pyramid
        // stores the farthest depth per region: that is the aggregate a
        // sound whole-box occlusion cull compares against. It used to store
        // minimums, which made the cull unsound over mixed-depth footprints.)
        // Top-left (0,0)-(1,1): max(1,2,5,6) = 6.0
        // Top-right (2,0)-(3,1): max(3,4,7,8) = 8.0
        // Bottom-left (0,2)-(1,3): max(9,10,13,14) = 14.0
        // Bottom-right (2,2)-(3,3): max(11,12,15,16) = 16.0
        let level1 = &hiz.levels[1];
        assert_eq!(level1.width, 2);
        assert_eq!(level1.height, 2);
        assert!((level1.depths[0] - 6.0).abs() < f32::EPSILON); // Top-left quad
        assert!((level1.depths[1] - 8.0).abs() < f32::EPSILON); // Top-right quad
        assert!((level1.depths[2] - 14.0).abs() < f32::EPSILON); // Bottom-left quad
        assert!((level1.depths[3] - 16.0).abs() < f32::EPSILON); // Bottom-right quad
    }

    #[test]
    fn test_max_reduction_propagates_to_top() {
        let mut zb = ZBuffer::new(4, 4).unwrap();

        // Fill with gradient, largest value at (3,3)
        let slice = zb.as_mut_slice();
        for y in 0..4 {
            for x in 0..4 {
                slice[y * 4 + x] = (x + y) as f32;
            }
        }

        let mut hiz = HiZBuffer::new(4, 4);
        hiz.build_pyramid(&zb);

        // Level 2 should be 1×1 with global maximum
        let level2 = &hiz.levels[2];
        assert_eq!(level2.width, 1);
        assert_eq!(level2.height, 1);
        assert!((level2.depths[0] - 6.0).abs() < f32::EPSILON); // Maximum of entire zbuffer
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
                assert!((*depth - 5.0).abs() < f32::EPSILON);
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
                if (100..200).contains(&x) && (100..200).contains(&y) {
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
    fn test_partially_occluded_aabb_returns_true() {
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Fill zbuffer with depth 10.0, except one region with 5.0 (closer)
        let slice = zb.as_mut_slice();
        for y in 0..600 {
            for x in 0..800 {
                if (100..200).contains(&x) && (100..200).contains(&y) {
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

        // CORRECTED EXPECTATION (was: assert occluded). The old test codified
        // an unsound cull: only the part of the footprint overlapping the 5.0
        // patch is occluded. Over the rest of the footprint the stored depth
        // is 10.0, which is *behind* the box's front face (7.0), so the box
        // is visible there. A whole-box cull requires the box's front to be
        // behind the MAXIMUM depth over the footprint (10.0), not the minimum
        // (5.0). Comparing against the minimum false-culls this box.
        assert!(hiz.is_potentially_visible(aabb));
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
    fn test_coarse_bin_visible_when_closer() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 10.0
        let slice = zb.as_mut_slice();
        slice.fill(10.0);

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Test coarse bin (128×128) with min_depth=5.0 (closer than zbuffer)
        let bin_aabb = AABB3D {
            min_x: 0,
            max_x: 127,
            min_y: 0,
            max_y: 127,
            min_depth: 5.0,
            max_depth: 15.0,
        };

        // Should be visible because bin is closer than existing geometry
        assert!(hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_occluded_when_farther() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 5.0
        let slice = zb.as_mut_slice();
        slice.fill(5.0);

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Test coarse bin (128×128) with min_depth=10.0 (farther than zbuffer)
        let bin_aabb = AABB3D {
            min_x: 0,
            max_x: 127,
            min_y: 0,
            max_y: 127,
            min_depth: 10.0,
            max_depth: 20.0,
        };

        // Should be occluded because bin is farther than existing geometry
        assert!(!hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_offscreen_returns_false() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();
        let slice = zb.as_mut_slice();
        slice.fill(10.0);

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Bin completely offscreen (negative coords)
        let bin_aabb = AABB3D {
            min_x: -200,
            max_x: -72,
            min_y: -200,
            max_y: -72,
            min_depth: 5.0,
            max_depth: 15.0,
        };

        assert!(!hiz.is_coarse_bin_visible(bin_aabb));

        // Bin beyond screen bounds
        let bin_aabb = AABB3D {
            min_x: 2000,
            max_x: 2127,
            min_y: 1200,
            max_y: 1327,
            min_depth: 5.0,
            max_depth: 15.0,
        };

        assert!(!hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_partial_occlusion() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 10.0, except one region with 5.0 (closer)
        let slice = zb.as_mut_slice();
        for y in 0..1080 {
            for x in 0..1920 {
                if (64..128).contains(&x) && (64..128).contains(&y) {
                    slice[y * 1920 + x] = 5.0; // Closer occluder
                } else {
                    slice[y * 1920 + x] = 10.0;
                }
            }
        }

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Test coarse bin that overlaps the closer region
        let bin_aabb = AABB3D {
            min_x: 0,
            max_x: 127,
            min_y: 0,
            max_y: 127,
            min_depth: 7.0, // Farther than the occluder at 5.0
            max_depth: 12.0,
        };

        // CORRECTED EXPECTATION (was: assert occluded). Same unsound cull as
        // the AABB test: only the part of the bin overlapping the 5.0 patch
        // is occluded; over the rest of the bin the stored depth is 10.0,
        // behind the bin's front face (7.0), so the bin is visible there.
        // A whole-bin cull needs the MAXIMUM depth over the footprint.
        assert!(hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_queries_level_2() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 10.0
        let slice = zb.as_mut_slice();
        slice.fill(10.0);
        slice.fill(10.0);
        slice.fill(10.0);
        slice.fill(10.0);

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Verify level 2 exists and has correct dimensions
        // For 1920×1080: level 2 should be 480×270 (div by 4)
        assert_eq!(hiz.level_dimensions(2), Some((480, 270)));

        // Test that coarse bin uses level 2 (implicitly tested via correct results)
        let bin_aabb = AABB3D {
            min_x: 128,
            max_x: 255,
            min_y: 128,
            max_y: 255,
            min_depth: 5.0,
            max_depth: 15.0,
        };

        // Should be visible (bin is closer)
        assert!(hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_invalid_pyramid_assumes_visible() {
        let hiz = HiZBuffer::new(1920, 1080);
        // Don't build pyramid, leave it invalid

        let bin_aabb = AABB3D {
            min_x: 0,
            max_x: 127,
            min_y: 0,
            max_y: 127,
            min_depth: 5.0,
            max_depth: 10.0,
        };

        // Invalid pyramid should assume everything is visible
        assert!(hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_multiple_regions() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Create a checkerboard pattern with different depths
        let slice = zb.as_mut_slice();
        for y in 0..1080 {
            for x in 0..1920 {
                // 128×128 tile pattern
                let tile_x = x / 128;
                let tile_y = y / 128;
                let is_even = (tile_x + tile_y) % 2 == 0;
                slice[y * 1920 + x] = if is_even { 5.0 } else { 15.0 };
            }
        }

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Test bin in an even tile (depth 5.0)
        let bin_even = AABB3D {
            min_x: 0,
            max_x: 127,
            min_y: 0,
            max_y: 127,
            min_depth: 10.0, // Farther than 5.0
            max_depth: 20.0,
        };
        assert!(!hiz.is_coarse_bin_visible(bin_even)); // Should be occluded

        // Test bin in an odd tile (depth 15.0)
        let bin_odd = AABB3D {
            min_x: 128,
            max_x: 255,
            min_y: 0,
            max_y: 127,
            min_depth: 10.0, // Closer than 15.0
            max_depth: 20.0,
        };
        assert!(hiz.is_coarse_bin_visible(bin_odd)); // Should be visible
    }

    #[test]
    fn test_coarse_bin_boundary_conditions() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();
        let slice = zb.as_mut_slice();
        slice.fill(10.0);

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Test bin at screen edges (clamping behavior)
        let bin_partial = AABB3D {
            min_x: 1850,
            max_x: 1977, // Extends beyond screen width (1920)
            min_y: 950,
            max_y: 1077, // Extends beyond screen height (1080)
            min_depth: 5.0,
            max_depth: 15.0,
        };

        // Should be visible (bin is closer, clamping should work correctly)
        assert!(hiz.is_coarse_bin_visible(bin_partial));
    }

    #[test]
    fn test_coarse_bin_equal_depth_returns_true() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 10.0
        let slice = zb.as_mut_slice();
        slice.fill(10.0);
        slice.fill(10.0);
        slice.fill(10.0);

        let mut hiz = HiZBuffer::new(1920, 1080);
        hiz.build_pyramid(&zb);

        // Test bin with min_depth exactly equal to zbuffer depth
        let bin_aabb = AABB3D {
            min_x: 0,
            max_x: 127,
            min_y: 0,
            max_y: 127,
            min_depth: 10.0, // Equal to zbuffer
            max_depth: 20.0,
        };

        // Should be visible (conservative: equal depth treated as visible)
        assert!(hiz.is_coarse_bin_visible(bin_aabb));
    }

    /// Brute-force reference: a box is occluded only if *every* pixel of its
    /// (clamped) footprint holds a stored depth strictly closer than the
    /// box's front face. Smaller depth = closer; equal depth is visible.
    fn reference_occluded(depths: &[f32], width: u32, aabb: AABB3D) -> bool {
        let min_x = aabb.min_x.max(0);
        let max_x = aabb.max_x.min(width as i32 - 1);
        let min_y = aabb.min_y.max(0);
        let max_y = aabb.max_y.min((depths.len() as u32 / width) as i32 - 1);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if depths[y as usize * width as usize + x as usize] >= aabb.min_depth {
                    return false; // stored surface at/behind box front: visible here
                }
            }
        }
        true
    }

    #[test]
    fn test_mixed_depth_footprint_is_not_false_culled() {
        // Regression scene for the min-pyramid bug: the left half of the
        // footprint is covered by a close occluder (5.0), the right half by
        // a far one (15.0). A box whose front (10.0) sits between the two is
        // visible over the right half. The old query compared the box front
        // against the footprint MINIMUM (5.0) and culled it anyway.
        let mut zb = ZBuffer::new(8, 8).unwrap();
        let slice = zb.as_mut_slice();
        for y in 0..8 {
            for x in 0..8 {
                slice[y * 8 + x] = if x < 4 { 5.0 } else { 15.0 };
            }
        }

        let mut hiz = HiZBuffer::new(8, 8);
        hiz.build_pyramid(&zb);

        let aabb = AABB3D::new(0, 7, 0, 7, 10.0, 12.0);
        assert!(hiz.is_potentially_visible(aabb));

        // Same scene through the coarse-bin path.
        assert!(hiz.is_coarse_bin_visible(aabb));
    }

    #[test]
    fn test_heterogeneous_occluder_still_culls_when_box_behind_farthest() {
        // Depths vary across the footprint (5.0..8.0) but every pixel is in
        // front of the box front (10.0): the cull must still fire. Guards
        // against "fixing" the false cull by never culling at all.
        let mut zb = ZBuffer::new(8, 8).unwrap();
        let slice = zb.as_mut_slice();
        for y in 0..8 {
            for x in 0..8 {
                slice[y * 8 + x] = 5.0 + ((x + y) % 4) as f32;
            }
        }

        let mut hiz = HiZBuffer::new(8, 8);
        hiz.build_pyramid(&zb);

        let aabb = AABB3D::new(0, 7, 0, 7, 10.0, 12.0);
        assert!(!hiz.is_potentially_visible(aabb));
        assert!(!hiz.is_coarse_bin_visible(aabb));
    }

    #[test]
    fn test_aabb_verdict_never_false_culls_randomized() {
        // Property: for randomized depth buffers and boxes, a "culled"
        // verdict from either query must agree with the brute-force
        // per-pixel reference. False "visible" verdicts are acceptable
        // (conservative); false culls are not.
        // Deterministic xorshift64* so failures reproduce exactly.
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut next_u32 = move || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
        };

        let (w, h) = (24u32, 18u32); // non-power-of-two on purpose
        for case in 0..500u32 {
            let mut zb = ZBuffer::new(w, h).unwrap();
            {
                let slice = zb.as_mut_slice();
                for d in slice.iter_mut() {
                    // Depths 1..=20 in 0.5 steps, plus empty (INFINITY) cells.
                    let r = next_u32() % 45;
                    *d = if r == 44 {
                        f32::INFINITY
                    } else {
                        1.0 + (r % 40) as f32 * 0.5
                    };
                }
            }
            let depths_snapshot = zb.as_slice().to_vec();

            let mut hiz = HiZBuffer::new(w, h);
            hiz.build_pyramid(&zb);

            for _ in 0..20 {
                let x0 = (next_u32() % w) as i32;
                let y0 = (next_u32() % h) as i32;
                let x1 = x0 + (next_u32() % 12) as i32;
                let y1 = y0 + (next_u32() % 12) as i32;
                let front = 0.5 + (next_u32() % 44) as f32 * 0.5;
                let aabb = AABB3D::new(
                    x0,
                    x1.min(w as i32 - 1),
                    y0,
                    y1.min(h as i32 - 1),
                    front,
                    front + 5.0,
                );

                let occluded_ref = reference_occluded(&depths_snapshot, w, aabb);
                assert!(
                    hiz.is_potentially_visible(aabb) || occluded_ref,
                    "case {case}: is_potentially_visible false-culled {aabb:?}"
                );
                assert!(
                    hiz.is_coarse_bin_visible(aabb) || occluded_ref,
                    "case {case}: is_coarse_bin_visible false-culled {aabb:?}"
                );
            }
        }
    }
}
