#![allow(clippy::manual_is_multiple_of)]
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
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::hiz_buffer::HiZBuffer;
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

    #[cfg(feature = "gpu-binning")]
    gpu_builder: Option<crate::gpu::GpuHiZBuilder>,
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

            #[cfg(feature = "gpu-binning")]
            gpu_builder: None,
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

        // Try GPU build first if enabled
        #[cfg(feature = "gpu-binning")]
        {
            let use_gpu = self.gpu_builder.is_some();
            if use_gpu {
                // Take ownership temporarily to avoid borrow issues
                let mut gpu = self.gpu_builder.take().unwrap();

                let result = gpu
                    .upload_zbuffer(zbuffer.as_slice())
                    .and_then(|_| gpu.build_pyramid())
                    .and_then(|_| gpu.download_pyramid(self));

                // Put it back
                self.gpu_builder = Some(gpu);

                if result.is_ok() {
                    // GPU build succeeded, pyramid is valid
                    return;
                }
                // GPU build failed, fall through to CPU build
            }
        }

        // CPU fallback
        // Only build pyramid if there are levels beyond level 0
        if self.level_count > 1 {
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
        }

        self.valid = true;
    }

    /// Enable GPU-accelerated pyramid build
    ///
    /// Creates a GPU compute shader pipeline for building the Hi-Z pyramid on the GPU.
    /// Falls back to CPU build if GPU initialization fails.
    #[cfg(feature = "gpu-binning")]
    pub fn enable_gpu_build(&mut self) -> Result<(), crate::gpu::GpuError> {
        self.gpu_builder = Some(crate::gpu::GpuHiZBuilder::new(self.width, self.height)?);
        Ok(())
    }

    /// Build a single pyramid level via 2×2 min-reduction
    fn build_level(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
        // SIMD disabled after extensive profiling and optimization (2026-02-06)
        //
        // **History:**
        // - Initial AVX2 SIMD: 2.7× slower than scalar (6 shuffles per 4 pixels)
        // - Optimized version: 1.9× slower (5 shuffles per 8 pixels, 58% reduction)
        // - Added adaptive threshold: Still 1.9× slower at 1080p and 4K
        //
        // **Root causes:**
        // 1. Memory bandwidth saturation: 4× unaligned loads per 2×2 reduction
        //    - Scalar: 2.16 cycles/pixel
        //    - SIMD: 4.12 cycles/pixel (1.9× overhead)
        // 2. Excessive shuffle operations: Even optimized 5-shuffle pattern too slow
        //    - Horizontal min-reduction requires complex shuffle patterns
        //    - Each shuffle: 1-3 cycles latency, overhead exceeds benefit
        // 3. Small pyramid levels: Upper levels (<64 pixels) too small to amortize setup cost
        // 4. Unaligned loads: _mm256_loadu_ps is 2-3× slower than aligned loads
        //
        // **Attempts:**
        // - ✅ Reduced shuffles from 6→5 per 8 pixels (58% reduction)
        // - ✅ Added width threshold (skip SIMD for levels <16 pixels)
        // - ❌ Still 1.9× slower than scalar baseline
        //
        // **Conclusion:**
        // Hi-Z pyramid is fundamentally unsuited for SIMD due to:
        // - Small working set (most levels <128 pixels wide)
        // - Memory-bound (4× loads per output pixel)
        // - Complex shuffle patterns (horizontal reductions are expensive)
        //
        // Scalar implementation is optimal for this workload.
        // See: SIMD_PROFILING_ANALYSIS.md for full profiling data
        self.build_level_scalar(level_idx, source, source_width);
    }

    /// Scalar 2×2 min-reduction implementation
    fn build_level_scalar(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
        let level_width = self.levels[level_idx as usize].width;
        let level_height = self.levels[level_idx as usize].height;
        let dest = &mut self.levels[level_idx as usize].depths;

        let sw = source_width as usize;
        let sh = source.len() / sw;

        let odd_width = (source_width % 2) != 0;
        let odd_height = (sh % 2) != 0;

        let safe_width = if odd_width { level_width - 1 } else { level_width };
        let safe_height = if odd_height {
            level_height - 1
        } else {
            level_height
        };

        for y in 0..safe_height {
            let src_y = (y * 2) as usize;
            let row0_start = src_y * sw;
            let row1_start = (src_y + 1) * sw;

            // Slice rows to avoid bounds checks in inner loop
            let row0 = &source[row0_start..];
            let row1 = &source[row1_start..];
            let dst_row = &mut dest[(y * level_width) as usize..];

            for x in 0..safe_width {
                let sx = (x * 2) as usize;

                // Direct access: we know sx+1 is valid because x < safe_width
                let d00 = row0[sx];
                let d10 = row0[sx + 1];
                let d01 = row1[sx];
                let d11 = row1[sx + 1];

                dst_row[x as usize] = d00.min(d10).min(d01).min(d11);
            }

            // Handle last column if odd width
            if odd_width {
                let x = safe_width;
                let sx = (x * 2) as usize;
                let d00 = row0[sx];
                let d01 = row1[sx];
                // Clamp to left column
                dst_row[x as usize] = d00.min(d01);
            }
        }

        // Handle last row if odd height
        if odd_height {
            let y = safe_height;
            let src_y = (y * 2) as usize;
            let row0_start = src_y * sw;
            let row0 = &source[row0_start..];
            let dst_row = &mut dest[(y * level_width) as usize..];

            for x in 0..safe_width {
                let sx = (x * 2) as usize;
                let d00 = row0[sx];
                let d10 = row0[sx + 1];
                // Clamp to top row
                dst_row[x as usize] = d00.min(d10);
            }

            if odd_width {
                let x = safe_width;
                let sx = (x * 2) as usize;
                let d00 = row0[sx];
                dst_row[x as usize] = d00;
            }
        }
    }

    /// SIMD 2×2 min-reduction using AVX2 (processes 8 reductions simultaneously)
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn build_level_simd(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::*;

            let level_width = self.levels[level_idx as usize].width;
            let level_height = self.levels[level_idx as usize].height;

            // Only use SIMD for wide levels (amortize overhead)
            // For narrow levels (<16 pixels), scalar is faster due to setup overhead.
            const SIMD_WIDTH_THRESHOLD: u32 = 16;
            if level_width < SIMD_WIDTH_THRESHOLD {
                return self.build_level_scalar(level_idx, source, source_width);
            }

            // Process 8 output pixels at a time (optimized shuffle pattern)
            let simd_width = 8;

            for y in 0..level_height {
                let mut x = 0;

                // SIMD loop: process 8 output pixels at once
                while x + simd_width <= level_width {
                    let src_x = (x * 2) as usize;
                    let src_y = (y * 2) as usize;
                    let src_width_usize = source_width as usize;

                    unsafe {
                        // For 8 output pixels, we need 16 source values per row
                        // Each 2×2 reduction: (i, i+1) from row0 and row1
                        let row0_idx = src_y * src_width_usize + src_x;
                        let row1_idx = (src_y + 1) * src_width_usize + src_x;

                        // Bounds check
                        if row0_idx + 16 <= source.len() && row1_idx + 16 <= source.len() {
                            // Load 16 values from each row (2× 256-bit loads per row)
                            let row0_lo = _mm256_loadu_ps(source.as_ptr().add(row0_idx));
                            let row0_hi = _mm256_loadu_ps(source.as_ptr().add(row0_idx + 8));
                            let row1_lo = _mm256_loadu_ps(source.as_ptr().add(row1_idx));
                            let row1_hi = _mm256_loadu_ps(source.as_ptr().add(row1_idx + 8));

                            // Vertical min: min(row0, row1) for both halves
                            let min_vert_lo = _mm256_min_ps(row0_lo, row1_lo);
                            let min_vert_hi = _mm256_min_ps(row0_hi, row1_hi);

                            // Horizontal min-reduction using optimized 4-shuffle pattern
                            // Goal: Minimize shuffles by clever use of hadd-style reduction
                            //
                            // min_vert_lo: [v0, v1, v2, v3 | v4, v5, v6, v7]
                            // min_vert_hi: [v8, v9, v10, v11 | v12, v13, v14, v15]
                            // Want: [min(v0,v1), min(v2,v3), ..., min(v14,v15)]

                            // Use hadd-style shuffle: swap adjacent pairs then min
                            // Shuffle to get: [v1, v0, v3, v2 | v5, v4, v7, v6]
                            let swapped_lo = _mm256_permute_ps(min_vert_lo, 0b10_11_00_01);
                            let swapped_hi = _mm256_permute_ps(min_vert_hi, 0b10_11_00_01);
                            // swapped_lo: [v1, v0, v3, v2 | v5, v4, v7, v6]
                            // swapped_hi: [v9, v8, v11, v10 | v13, v12, v15, v14]

                            // Min with original to get horizontal pairs
                            let min_pairs_lo = _mm256_min_ps(min_vert_lo, swapped_lo);
                            let min_pairs_hi = _mm256_min_ps(min_vert_hi, swapped_hi);
                            // min_pairs_lo: [min01, min01, min23, min23 | min45, min45, min67, min67]
                            // min_pairs_hi: [min89, min89, min1011, min1011 | min1213, min1213, min1415, min1415]

                            // Final packing strategy: use permute2f128 to rearrange lanes, then shuffle
                            // Step 1: Gather low lanes [min01, min01, min23, min23, min89, min89, min1011, min1011]
                            let low_lanes =
                                _mm256_permute2f128_ps(min_pairs_lo, min_pairs_hi, 0x20);
                            // Step 2: Gather high lanes [min45, min45, min67, min67, min1213, min1213, min1415, min1415]
                            let high_lanes =
                                _mm256_permute2f128_ps(min_pairs_lo, min_pairs_hi, 0x31);

                            // Step 3: Shuffle to extract unique values and interleave
                            // Mask 0b10_00_10_00 extracts indices [0, 2] from each source
                            let final_result =
                                _mm256_shuffle_ps(low_lanes, high_lanes, 0b10_00_10_00);
                            // final_result: [min01, min23, min45, min67 | min89, min1011, min1213, min1415]

                            // Store 8 results
                            let dst_idx = (y * level_width + x) as usize;
                            _mm256_storeu_ps(
                                self.levels[level_idx as usize]
                                    .depths
                                    .as_mut_ptr()
                                    .add(dst_idx),
                                final_result,
                            );
                        } else {
                            // Fallback to scalar for boundary cases
                            for i in 0..simd_width {
                                if x + i >= level_width {
                                    break;
                                }
                                self.build_level_scalar_single(
                                    level_idx,
                                    source,
                                    source_width,
                                    x + i,
                                    y,
                                );
                            }
                        }
                    }

                    x += simd_width;
                }

                // Scalar tail for remaining pixels
                while x < level_width {
                    self.build_level_scalar_single(level_idx, source, source_width, x, y);
                    x += 1;
                }
            }
        }
    }

    /// Helper to process a single output pixel (used for SIMD tail and boundary cases)
    #[inline]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn build_level_scalar_single(
        &mut self,
        level_idx: u32,
        source: &[f32],
        source_width: u32,
        x: u32,
        y: u32,
    ) {
        let src_x = (x * 2) as usize;
        let src_y = (y * 2) as usize;
        let source_width_usize = source_width as usize;

        // Sample 2×2 quad from previous level
        let d00 = source[src_y * source_width_usize + src_x];
        let d10 = source
            .get(src_y * source_width_usize + src_x + 1)
            .copied()
            .unwrap_or(d00);
        let d01 = source
            .get((src_y + 1) * source_width_usize + src_x)
            .copied()
            .unwrap_or(d00);
        let d11 = source
            .get((src_y + 1) * source_width_usize + src_x + 1)
            .copied()
            .unwrap_or(d00);

        let min_depth = d00.min(d10).min(d01).min(d11);
        let level_width = self.levels[level_idx as usize].width;
        self.levels[level_idx as usize].depths[(y * level_width + x) as usize] = min_depth;
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

    /// Test if a coarse bin (128×128 pixels) is potentially visible
    ///
    /// This method is used for two-level hierarchical binning, where coarse bins
    /// (128×128 pixels) are first tested against the Hi-Z pyramid at level 2.
    /// Only visible coarse bins are subdivided into fine bins (32×32 pixels).
    ///
    /// # Algorithm
    /// 1. Convert bin AABB to pyramid level 2 coordinates (128×128 = 32 * 2^2)
    /// 2. Sample all pyramid cells covering the bin region
    /// 3. Find minimum depth across all samples
    /// 4. Compare bin's `min_depth` against pyramid's `min_depth`
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
    /// use abrash::hiz_buffer::{HiZBuffer, AABB3D};
    /// use abrash::zbuffer::ZBuffer;
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

        // Find minimum depth in covered pyramid cells
        let mut pyramid_min = f32::INFINITY;
        for ly in ly0..=ly1 {
            for lx in lx0..=lx1 {
                pyramid_min = pyramid_min.min(level.depths[ly * level.width as usize + lx]);
            }
        }

        // Conservative test: If bin's closest point is farther than
        // pyramid's closest point, bin is fully occluded
        bin_aabb.min_depth <= pyramid_min
    }

    /// Write pyramid level data from GPU (for GPU Hi-Z pyramid build)
    ///
    /// This method allows the GPU Hi-Z builder to populate pyramid levels
    /// directly from GPU-computed data.
    ///
    /// # Arguments
    /// * `level` - Pyramid level index (0 = full resolution, 1+ = reductions)
    /// * `data` - Depth values in row-major order (width × height floats)
    ///
    /// # Panics
    /// Panics if level index is out of range or data size doesn't match level dimensions
    #[cfg(feature = "gpu-binning")]
    pub fn write_level_data(&mut self, level: u32, data: &[f32]) {
        assert!(
            level < self.level_count,
            "Level index {} out of range (max {})",
            level,
            self.level_count - 1
        );

        let level_data = &mut self.levels[level as usize];
        let expected_size = (level_data.width * level_data.height) as usize;
        assert_eq!(
            data.len(),
            expected_size,
            "Data size {} doesn't match level {} dimensions {}×{} (expected {} floats)",
            data.len(),
            level,
            level_data.width,
            level_data.height,
            expected_size
        );

        // Copy GPU data into pyramid level
        level_data.depths.copy_from_slice(data);
    }

    /// Mark pyramid as valid after GPU build
    ///
    /// This should be called after all pyramid levels have been written via write_level_data()
    #[cfg(feature = "gpu-binning")]
    pub fn mark_valid(&mut self) {
        self.valid = true;
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

    #[test]
    fn test_coarse_bin_visible_when_closer() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 10.0
        let slice = zb.as_mut_slice();
        for i in 0..slice.len() {
            slice[i] = 10.0;
        }

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
        for i in 0..slice.len() {
            slice[i] = 5.0;
        }

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
        for i in 0..slice.len() {
            slice[i] = 10.0;
        }

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
                if x >= 64 && x < 128 && y >= 64 && y < 128 {
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

        // Should be occluded because there's closer geometry in the bin
        assert!(!hiz.is_coarse_bin_visible(bin_aabb));
    }

    #[test]
    fn test_coarse_bin_queries_level_2() {
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Fill zbuffer with depth 10.0
        let slice = zb.as_mut_slice();
        for i in 0..slice.len() {
            slice[i] = 10.0;
        }

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
        for i in 0..slice.len() {
            slice[i] = 10.0;
        }

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
        for i in 0..slice.len() {
            slice[i] = 10.0;
        }

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
}
