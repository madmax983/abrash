//! # Software Occluder Rasterization into a Tiled Hi-Z Buffer 🔱
//!
//! [`HiZBuffer`](crate::hiz_buffer::HiZBuffer) answers occlusion queries from a
//! depth buffer that has *already been rendered* at full resolution. This
//! module is the other half of the classic software-occlusion pipeline
//! (Greene 1996; Intel's masked software occlusion culling): rasterize a
//! handful of cheap, known-solid **occluders** — walls, buildings, terrain
//! slabs — directly into a coarse depth buffer *before* shading anything,
//! then test every other object's screen-space bounds against it. Objects
//! that fail the test are never binned, rasterized, or shaded at all.
//!
//! ## Layout
//!
//! The screen is partitioned into square **tiles** of
//! [`DEFAULT_TILE_SIZE`]×[`DEFAULT_TILE_SIZE`] pixels (configurable per
//! buffer). Each tile is one cell of the base level and stores a single
//! depth. Above the base level sits a pyramid whose levels are built by
//! 2×2 **max**-reduction (an AVX2 kernel handles the bulk on x86, with a
//! scalar fallback that produces identical results).
//!
//! ## The soundness invariant (why max, and why full coverage)
//!
//! Depth convention, as everywhere in this crate: **smaller is closer**.
//! Let `O(p)` be the nearest occluder depth at pixel `p`, or `INFINITY`
//! where no occluder covers `p`. A query box with nearest depth `z` is
//! occluded iff `z > O(p)` for *every* pixel `p` of its footprint —
//! equivalently `z > max O(footprint)`. Two rules keep the stored data a
//! sound over-estimate of that maximum, so a "culled" verdict is never
//! wrong (false *keeps* are fine; false culls are forbidden):
//!
//! 1. **Full coverage only.** An occluder writes a cell only if it covers
//!    the *whole* cell. A partially covered cell keeps `INFINITY`, and
//!    `INFINITY` wins every max-reduction above it, so one uncovered
//!    corner poisons the whole region above it — exactly as it should.
//! 2. **Farthest surface point.** The value written is the *maximum*
//!    occluder depth over the cell (for a slanted triangle, its depth at
//!    the cell's farthest corner). A box behind that point is behind the
//!    occluder everywhere in the cell. Where several occluders fully cover
//!    one cell, the stored value is the *minimum* of their candidates,
//!    which is still an upper bound on `max O(cell)` and the tightest
//!    such bound available per occluder.
//!
//! A consequence worth knowing: an occluder smaller than one tile can
//! never fully cover a cell and contributes nothing. Choose the tile size
//! for the occluders you actually have (32 px suits buildings and walls;
//! 8–16 px suits smaller props, at a larger base grid).
//!
//! ## Examples
//!
//! ```rust
//! use abrash_core::hiz_buffer::AABB3D;
//! use abrash_core::occluder_hiz::OccluderHiZ;
//!
//! let mut occluders = OccluderHiZ::new(640, 360);
//! // A solid wall filling the left half of the screen at depth 5.
//! occluders.rasterize_rect_occluder(0, 0, 319, 359, 5.0);
//! occluders.build_pyramid();
//!
//! // A crate behind the wall: culled without touching the rasterizer.
//! let crate_behind = AABB3D::new(32, 96, 32, 96, 10.0, 12.0);
//! assert!(occluders.is_occluded(crate_behind));
//!
//! // The same crate in front of the wall: must be drawn.
//! let crate_front = AABB3D::new(32, 96, 32, 96, 2.0, 4.0);
//! assert!(occluders.is_potentially_visible(crate_front));
//! ```

use crate::hiz_buffer::AABB3D;

/// Default occluder tile size in pixels (one base cell covers 32×32).
pub const DEFAULT_TILE_SIZE: u32 = 32;

/// A screen-space vertex with depth, for triangle occluders.
///
/// Coordinates are in pixels (the same space [`AABB3D`] uses); `depth`
/// follows the crate convention — smaller is closer to the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenVertex {
    /// X coordinate in pixels.
    pub x: f32,
    /// Y coordinate in pixels.
    pub y: f32,
    /// Depth at this vertex (smaller is closer).
    pub depth: f32,
}

impl ScreenVertex {
    /// Creates a new screen-space vertex.
    #[must_use]
    pub const fn new(x: f32, y: f32, depth: f32) -> Self {
        Self { x, y, depth }
    }
}

/// One level of the max-pyramid. Level 0 cells are the tiles themselves.
struct Level {
    width: u32,
    height: u32,
    depths: Vec<f32>,
}

impl Level {
    fn new(width: u32, height: u32) -> Self {
        let size = usize::try_from(u64::from(width) * u64::from(height))
            .expect("occluder Hi-Z dimensions overflow");
        Self {
            width,
            height,
            depths: vec![f32::INFINITY; size],
        }
    }

    fn get(&self, x: u32, y: u32) -> f32 {
        self.depths[(y * self.width + x) as usize]
    }
}

/// Tiled hierarchical depth buffer fed by software-rasterized occluders.
///
/// See the module documentation for the soundness invariant. The typical
/// per-frame flow is:
///
/// 1. [`clear`](Self::clear) (or construct) the buffer.
/// 2. Rasterize occluders with
///    [`rasterize_rect_occluder`](Self::rasterize_rect_occluder),
///    [`rasterize_aabb_occluder`](Self::rasterize_aabb_occluder), and
///    [`rasterize_triangle_occluder`](Self::rasterize_triangle_occluder).
/// 3. Call [`build_pyramid`](Self::build_pyramid) once.
/// 4. Test scene objects with
///    [`is_potentially_visible`](Self::is_potentially_visible),
///    [`is_occluded`](Self::is_occluded), or the batch
///    [`cull_aabbs_prealloc`](Self::cull_aabbs_prealloc).
pub struct OccluderHiZ {
    width: u32,
    height: u32,
    tile_size: u32,
    levels: Vec<Level>,
    valid: bool,
    covered_cells: usize,
}

impl OccluderHiZ {
    /// Creates a buffer for a `width`×`height` screen with the
    /// [`DEFAULT_TILE_SIZE`].
    ///
    /// # Panics
    ///
    /// Panics if a dimension is zero or the grid size overflows.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self::with_tile_size(width, height, DEFAULT_TILE_SIZE)
    }

    /// Creates a buffer with a custom tile size (in pixels).
    ///
    /// Smaller tiles cull more tightly and let smaller occluders
    /// contribute, at the cost of a larger base grid and pyramid.
    ///
    /// # Panics
    ///
    /// Panics if a dimension or the tile size is zero, the tile size is
    /// not a power of two, or the grid size overflows.
    #[must_use]
    pub fn with_tile_size(width: u32, height: u32, tile_size: u32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");
        assert!(
            tile_size > 0 && tile_size.is_power_of_two(),
            "Tile size must be a non-zero power of two"
        );

        let grid_w = width.div_ceil(tile_size);
        let grid_h = height.div_ceil(tile_size);

        // Pyramid over the tile grid: halve (rounding up) until 1×1.
        let mut levels = vec![Level::new(grid_w, grid_h)];
        let (mut w, mut h) = (grid_w, grid_h);
        while w > 1 || h > 1 {
            w = w.div_ceil(2);
            h = h.div_ceil(2);
            levels.push(Level::new(w, h));
        }

        Self {
            width,
            height,
            tile_size,
            levels,
            valid: false,
            covered_cells: 0,
        }
    }

    /// Screen width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Screen height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Tile size in pixels; one base cell covers `tile_size`² pixels.
    #[must_use]
    pub const fn tile_size(&self) -> u32 {
        self.tile_size
    }

    /// Base grid dimensions in cells: `(grid_width, grid_height)`.
    #[must_use]
    pub fn grid_dimensions(&self) -> (u32, u32) {
        (self.levels[0].width, self.levels[0].height)
    }

    /// Number of pyramid levels, including the base tile grid.
    #[must_use]
    pub const fn level_count(&self) -> u32 {
        // The level count is log2 of the grid size, so it always fits.
        self.levels.len() as u32
    }

    /// Dimensions of pyramid `level`, or `None` if out of range.
    #[must_use]
    pub fn level_dimensions(&self, level: u32) -> Option<(u32, u32)> {
        self.levels.get(level as usize).map(|l| (l.width, l.height))
    }

    /// Whether [`build_pyramid`](Self::build_pyramid) has run since the
    /// last occluder write. Queries on an invalid pyramid conservatively
    /// report everything as visible.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    /// Number of base cells at least one occluder fully covers.
    #[must_use]
    pub const fn covered_cell_count(&self) -> usize {
        self.covered_cells
    }

    /// Stored depth of base cell `(cell_x, cell_y)`, or `None` if the
    /// cell is out of range. Uncovered cells read `f32::INFINITY`.
    #[must_use]
    pub fn cell_depth(&self, cell_x: u32, cell_y: u32) -> Option<f32> {
        let base = &self.levels[0];
        if cell_x >= base.width || cell_y >= base.height {
            return None;
        }
        Some(base.get(cell_x, cell_y))
    }

    /// Removes all occluders and invalidates the pyramid.
    pub fn clear(&mut self) {
        for level in &mut self.levels {
            level.depths.fill(f32::INFINITY);
        }
        self.valid = false;
        self.covered_cells = 0;
    }

    /// Writes one candidate depth into a base cell, keeping the minimum
    /// of all candidates (see the module-level soundness rules).
    fn write_cell(&mut self, cell_x: u32, cell_y: u32, candidate: f32) {
        let base = &mut self.levels[0];
        let idx = (cell_y * base.width + cell_x) as usize;
        let old = base.depths[idx];
        if candidate < old {
            if old.is_infinite() {
                self.covered_cells += 1;
            }
            base.depths[idx] = candidate;
        }
        self.valid = false;
    }

    /// Rasterizes a solid axis-aligned rectangle occluder at constant
    /// `depth`, covering pixels `min_x..=max_x` × `min_y..=max_y`
    /// (inclusive, [`AABB3D`]-style).
    ///
    /// Only cells the rectangle *fully* covers are written. Non-finite
    /// depths are ignored: an occluder at `INFINITY` occludes nothing,
    /// and `NaN` cannot be ordered.
    pub fn rasterize_rect_occluder(
        &mut self,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
        depth: f32,
    ) {
        if !depth.is_finite() || min_x > max_x || min_y > max_y {
            return;
        }
        let t = i64::from(self.tile_size);
        // First cell whose left/top edge is at or after the rect edge,
        // and last cell whose right/bottom edge is at or before it.
        let cx0 = ceil_div(i64::from(min_x), t);
        let cy0 = ceil_div(i64::from(min_y), t);
        let cx1 = (i64::from(max_x) - t + 1).div_euclid(t);
        let cy1 = (i64::from(max_y) - t + 1).div_euclid(t);
        let (grid_w, grid_h) = self.grid_dimensions();
        let cx0 = cx0.clamp(0, i64::from(grid_w));
        let cy0 = cy0.clamp(0, i64::from(grid_h));
        let cx1 = cx1.clamp(-1, i64::from(grid_w) - 1);
        let cy1 = cy1.clamp(-1, i64::from(grid_h) - 1);
        if cx0 > cx1 || cy0 > cy1 {
            return;
        }
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                self.write_cell(cx as u32, cy as u32, depth);
            }
        }
    }

    /// Rasterizes a solid AABB occluder. The box's screen footprint is
    /// its `min_x..=max_x` × `min_y..=max_y` rectangle and its occluding
    /// surface is its front face, `min_depth`.
    ///
    /// The caller asserts the box is (approximately) solid — a wall, a
    /// building, a terrain block. Passing a loose bounds of a sparse
    /// object would over-occlude, which is a caller error, not a buffer
    /// error: the buffer's guarantee covers the occluders it is given.
    pub fn rasterize_aabb_occluder(&mut self, aabb: &AABB3D) {
        self.rasterize_rect_occluder(
            aabb.min_x,
            aabb.min_y,
            aabb.max_x,
            aabb.max_y,
            aabb.min_depth,
        );
    }

    /// Rasterizes a screen-space triangle occluder with per-vertex
    /// depths (depth varies affinely across the triangle).
    ///
    /// A cell is written only if all four of its corners lie inside the
    /// triangle (a triangle is convex, so corner coverage implies full
    /// cell coverage). The stored candidate is the triangle's depth at
    /// its farthest corner of the cell. Degenerate triangles and
    /// non-finite vertices are ignored.
    pub fn rasterize_triangle_occluder(
        &mut self,
        v0: ScreenVertex,
        v1: ScreenVertex,
        v2: ScreenVertex,
    ) {
        let verts = [v0, v1, v2];
        if verts
            .iter()
            .any(|v| !v.x.is_finite() || !v.y.is_finite() || !v.depth.is_finite())
        {
            return;
        }
        // Twice the signed area; also the edge-function normalizer.
        let area2 = edge(v0, v1, v2.x, v2.y);
        if area2 == 0.0 {
            return; // Degenerate: covers no area, occludes nothing.
        }
        // Barycentric weights are sub-area / total area; both share the
        // winding sign, so the weights come out >= 0 inside for either
        // winding. (Normalizing by the *sign* instead would flip the
        // weights for negatively-wound triangles and cull nothing.)
        let inv_area = 1.0 / area2;

        let t = self.tile_size as f32;
        let min_px = v0.x.min(v1.x).min(v2.x);
        let max_px = v0.x.max(v1.x).max(v2.x);
        let min_py = v0.y.min(v1.y).min(v2.y);
        let max_py = v0.y.max(v1.y).max(v2.y);
        let (grid_w, grid_h) = self.grid_dimensions();
        let cx0 = ((min_px / t).floor() as i32).clamp(0, grid_w as i32);
        let cy0 = ((min_py / t).floor() as i32).clamp(0, grid_h as i32);
        let cx1 = ((max_px / t).floor() as i32).clamp(0, grid_w as i32 - 1);
        let cy1 = ((max_py / t).floor() as i32).clamp(0, grid_h as i32 - 1);

        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let x0 = cx as f32 * t;
                let y0 = cy as f32 * t;
                let corners = [(x0, y0), (x0 + t, y0), (x0, y0 + t), (x0 + t, y0 + t)];
                let mut candidate = f32::NEG_INFINITY;
                let mut fully_covered = true;
                for (px, py) in corners {
                    // Barycentric coordinates of the corner.
                    let w0 = edge(v1, v2, px, py) * inv_area;
                    let w1 = edge(v2, v0, px, py) * inv_area;
                    let w2 = edge(v0, v1, px, py) * inv_area;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        fully_covered = false;
                        break;
                    }
                    let depth = w0 * v0.depth + w1 * v1.depth + w2 * v2.depth;
                    candidate = candidate.max(depth);
                }
                if fully_covered {
                    self.write_cell(cx as u32, cy as u32, candidate);
                }
            }
        }
    }

    /// Builds the max-pyramid over the rasterized base cells.
    ///
    /// Each parent cell stores the maximum of its (up to) four children,
    /// so an uncovered child (`INFINITY`) poisons its parent — a region
    /// can only cull when *every* cell inside it is covered. Runs in
    /// O(cells); the bulk reduction uses AVX2 where available.
    pub fn build_pyramid(&mut self) {
        for level_idx in 1..self.levels.len() {
            let (lower, upper) = self.levels.split_at_mut(level_idx);
            let src = &lower[level_idx - 1];
            let dst = &mut upper[0];
            max_reduce_2x2(
                &mut dst.depths,
                dst.width,
                dst.height,
                &src.depths,
                src.width,
                src.height,
            );
        }
        self.valid = true;
    }

    /// Exact maximum stored depth over the base-cell rectangle
    /// `cx0..=cx1` × `cy0..=cy1`, computed by descending the pyramid:
    /// nodes fully inside the rectangle contribute their stored value
    /// directly, boundary nodes are subdivided. Because every stored
    /// value over-estimates the true per-cell maximum (module docs),
    /// the result over-estimates `max O(footprint)`.
    fn region_max(&self, cx0: u32, cy0: u32, cx1: u32, cy1: u32) -> f32 {
        let top = self.levels.len() - 1;
        let mut result = f32::NEG_INFINITY;
        // Stack entries: (level, x, y). Seed with the top-level cells
        // that overlap the rectangle.
        let mut stack: Vec<(u32, u32, u32)> = Vec::new();
        let (tx0, ty0) = (cx0 >> top, cy0 >> top);
        let (tx1, ty1) = (cx1 >> top, cy1 >> top);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                stack.push((top as u32, tx, ty));
            }
        }
        while let Some((level, x, y)) = stack.pop() {
            let level_usize = level as usize;
            let node = &self.levels[level_usize];
            if x >= node.width || y >= node.height {
                continue; // Past the rounded-up edge: no cells here.
            }
            // The node's footprint in base cells.
            let span = 1u32 << level;
            let (nx0, ny0) = (x * span, y * span);
            let (nx1, ny1) = (nx0 + span - 1, ny0 + span - 1);
            if nx1 < cx0 || nx0 > cx1 || ny1 < cy0 || ny0 > cy1 {
                continue; // No overlap with the query rectangle.
            }
            if level == 0 || (nx0 >= cx0 && nx1 <= cx1 && ny0 >= cy0 && ny1 <= cy1) {
                result = result.max(node.get(x, y));
                if result.is_infinite() {
                    return result; // Cannot get more conservative.
                }
                continue;
            }
            for dy in 0..2 {
                for dx in 0..2 {
                    stack.push((level - 1, x * 2 + dx, y * 2 + dy));
                }
            }
        }
        result
    }

    /// Tests a screen-space AABB against the pyramid.
    ///
    /// Returns `true` if the box might be visible and must be drawn:
    /// the pyramid is invalid, any covered cell of its footprint is
    /// uncovered, or its nearest depth is in front of (or level with)
    /// the occluders somewhere in its footprint. Returns `false` only
    /// when the box is provably behind occluders at every pixel of its
    /// footprint — or entirely off-screen.
    #[must_use]
    pub fn is_potentially_visible(&self, aabb: AABB3D) -> bool {
        if !self.valid {
            return true;
        }
        if !self.overlaps_screen(&aabb) {
            return false; // Entirely off-screen: nothing to draw.
        }
        let min_x = aabb.min_x.max(0);
        let max_x = aabb.max_x.min(self.width as i32 - 1);
        let min_y = aabb.min_y.max(0);
        let max_y = aabb.max_y.min(self.height as i32 - 1);
        let t = self.tile_size as i32;
        let cx0 = (min_x / t) as u32;
        let cx1 = (max_x / t) as u32;
        let cy0 = (min_y / t) as u32;
        let cy1 = (max_y / t) as u32;
        let farthest_occluder = self.region_max(cx0, cy0, cx1, cy1);
        // Occluded iff the box's nearest point is strictly behind the
        // farthest occluder depth over its whole footprint. NaN depths
        // fail every comparison and stay visible — conservative.
        !(aabb.min_depth > farthest_occluder)
    }

    /// Whether the box is provably occluded by the rasterized
    /// occluders. Off-screen boxes are *not* occluded (they are simply
    /// off-screen), and an invalid pyramid occludes nothing.
    #[must_use]
    pub fn is_occluded(&self, aabb: AABB3D) -> bool {
        self.overlaps_screen(&aabb) && !self.is_potentially_visible(aabb)
    }

    /// Batch form of [`is_potentially_visible`](Self::is_potentially_visible):
    /// writes one visibility verdict per AABB into `results`
    /// (`true` = potentially visible, must be drawn).
    ///
    /// # Panics
    ///
    /// Panics if `aabbs.len() != results.len()`.
    pub fn cull_aabbs_prealloc(&self, aabbs: &[AABB3D], results: &mut [bool]) {
        assert_eq!(aabbs.len(), results.len());
        for (aabb, out) in aabbs.iter().zip(results.iter_mut()) {
            *out = self.is_potentially_visible(*aabb);
        }
    }

    /// Batch form of [`is_potentially_visible`](Self::is_potentially_visible)
    /// that allocates its result vector.
    #[must_use]
    pub fn cull_aabbs(&self, aabbs: &[AABB3D]) -> Vec<bool> {
        let mut results = vec![true; aabbs.len()];
        self.cull_aabbs_prealloc(aabbs, &mut results);
        results
    }

    const fn overlaps_screen(&self, aabb: &AABB3D) -> bool {
        aabb.max_x >= 0
            && aabb.min_x < self.width as i32
            && aabb.max_y >= 0
            && aabb.min_y < self.height as i32
    }
}

/// Ceiling division for a possibly-negative numerator and positive
/// denominator.
fn ceil_div(a: i64, b: i64) -> i64 {
    debug_assert!(b > 0);
    a.div_euclid(b) + i64::from(a.rem_euclid(b) != 0)
}

/// Twice the signed area of the triangle `(a, b, p)` — the standard
/// edge function. Positive when `p` is left of the directed edge `a→b`.
fn edge(a: ScreenVertex, b: ScreenVertex, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

/// 2×2 max-reduction of `src` into `dst`. Missing children at odd
/// edges simply do not contribute; an `INFINITY` child dominates, which
/// is the conservative behaviour the pyramid needs.
fn max_reduce_2x2(
    dst: &mut [f32],
    dst_width: u32,
    dst_height: u32,
    src: &[f32],
    src_width: u32,
    src_height: u32,
) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if is_x86_feature_detected!("avx2") {
        // SAFETY: guarded by the runtime feature check above.
        unsafe {
            max_reduce_2x2_avx2(dst, dst_width, dst_height, src, src_width, src_height);
        }
        return;
    }
    max_reduce_2x2_scalar(dst, dst_width, dst_height, src, src_width, src_height);
}

fn max_reduce_2x2_scalar(
    dst: &mut [f32],
    dst_width: u32,
    dst_height: u32,
    src: &[f32],
    src_width: u32,
    src_height: u32,
) {
    for dy in 0..dst_height {
        for dx in 0..dst_width {
            let mut m = f32::NEG_INFINITY;
            for sy in (dy * 2)..=((dy * 2 + 1).min(src_height - 1)) {
                for sx in (dx * 2)..=((dx * 2 + 1).min(src_width - 1)) {
                    m = m.max(src[(sy * src_width + sx) as usize]);
                }
            }
            dst[(dy * dst_width + dx) as usize] = m;
        }
    }
}

/// AVX2 bulk path for [`max_reduce_2x2`]: each output covers a 2×2
/// block, so one iteration consumes 16 consecutive floats from each of
/// the two source rows and produces 8 outputs. The tail (narrow rows,
/// odd widths, last row) falls back to the scalar code, so results are
/// identical on every machine.
///
/// # Safety
///
/// Caller must ensure AVX2 is available (runtime-detected).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn max_reduce_2x2_avx2(
    dst: &mut [f32],
    dst_width: u32,
    dst_height: u32,
    src: &[f32],
    src_width: u32,
    src_height: u32,
) {
    use std::arch::x86_64::{
        __m256, _mm256_loadu_ps, _mm256_max_ps, _mm256_permutevar8x32_ps, _mm256_setr_epi32,
        _mm256_shuffle_ps, _mm256_storeu_ps,
    };

    // Pairs within a vector: lane i holds max(in[2i], in[2i+1]) after a
    // swap-adjacent shuffle and a max.
    // SAFETY: all loads/stores below are bounds-checked by the loop
    // conditions; intrinsics are covered by the target_feature gate.
    unsafe {
        let pair_max = |v: __m256| -> __m256 {
            let swapped = _mm256_shuffle_ps(v, v, 0b1011_0001);
            _mm256_max_ps(v, swapped)
        };
        // Gather the even lanes of two pair-maxed vectors (which hold
        // results 0..8 and 8..16 of the block row) into one vector.
        let gather_evens = |a: __m256, b: __m256| -> __m256 {
            let mixed = _mm256_shuffle_ps(a, b, 0b1000_1000);
            let idx = _mm256_setr_epi32(0, 1, 4, 5, 2, 3, 6, 7);
            _mm256_permutevar8x32_ps(mixed, idx)
        };

        let mut dy = 0u32;
        while dy < dst_height {
            let sy = dy * 2;
            if sy + 1 >= src_height {
                break; // Last source row (or none): scalar tail below.
            }
            let row0 = (sy * src_width) as usize;
            let row1 = row0 + src_width as usize;
            let dst_row = (dy * dst_width) as usize;
            let mut dx = 0u32;
            // 8 outputs need 16 inputs per row, fully inside the row.
            while dx + 8 <= dst_width && (dx * 2 + 16) <= src_width {
                let sx = (dx * 2) as usize;
                let top = gather_evens(
                    pair_max(_mm256_loadu_ps(src.as_ptr().add(row0 + sx))),
                    pair_max(_mm256_loadu_ps(src.as_ptr().add(row0 + sx + 8))),
                );
                let bottom = gather_evens(
                    pair_max(_mm256_loadu_ps(src.as_ptr().add(row1 + sx))),
                    pair_max(_mm256_loadu_ps(src.as_ptr().add(row1 + sx + 8))),
                );
                let out = _mm256_max_ps(top, bottom);
                _mm256_storeu_ps(dst.as_mut_ptr().add(dst_row + dx as usize), out);
                dx += 8;
            }
            // Scalar finish for this row's remaining outputs.
            for dx in dx..dst_width {
                let mut m = f32::NEG_INFINITY;
                for yy in sy..=(sy + 1) {
                    for xx in (dx * 2)..=((dx * 2 + 1).min(src_width - 1)) {
                        m = m.max(src[(yy * src_width + xx) as usize]);
                    }
                }
                dst[dst_row + dx as usize] = m;
            }
            dy += 1;
        }
        // Scalar finish for any remaining destination rows.
        while dy < dst_height {
            for dx in 0..dst_width {
                let mut m = f32::NEG_INFINITY;
                for yy in (dy * 2)..=((dy * 2 + 1).min(src_height - 1)) {
                    for xx in (dx * 2)..=((dx * 2 + 1).min(src_width - 1)) {
                        m = m.max(src[(yy * src_width + xx) as usize]);
                    }
                }
                dst[(dy * dst_width + dx) as usize] = m;
            }
            dy += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_at(min_x: i32, max_x: i32, min_y: i32, max_y: i32, min_depth: f32) -> AABB3D {
        AABB3D::new(min_x, max_x, min_y, max_y, min_depth, min_depth + 2.0)
    }

    #[test]
    fn test_grid_and_level_dimensions() {
        let buf = OccluderHiZ::new(640, 360);
        assert_eq!(buf.tile_size(), DEFAULT_TILE_SIZE);
        assert_eq!(buf.grid_dimensions(), (20, 12)); // ceil(640/32), ceil(360/32)
        assert_eq!(buf.level_count(), 6); // 20x12 -> 10x6 -> 5x3 -> 3x2 -> 2x1 -> 1x1
        assert_eq!(buf.level_dimensions(0), Some((20, 12)));
        assert_eq!(buf.level_dimensions(5), Some((1, 1)));
        assert_eq!(buf.level_dimensions(6), None);
        assert!(!buf.is_valid());
        assert_eq!(buf.covered_cell_count(), 0);
    }

    #[test]
    #[should_panic(expected = "Dimensions must be positive")]
    fn test_zero_dimension_panics() {
        let _ = OccluderHiZ::new(0, 64);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn test_non_power_of_two_tile_panics() {
        let _ = OccluderHiZ::with_tile_size(64, 64, 24);
    }

    #[test]
    fn test_rect_covers_aligned_cells() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_rect_occluder(0, 0, 63, 63, 5.0);
        assert_eq!(buf.covered_cell_count(), 4);
        for cy in 0..2 {
            for cx in 0..2 {
                assert_eq!(buf.cell_depth(cx, cy), Some(5.0));
            }
        }
        assert_eq!(buf.cell_depth(2, 0), None);
    }

    #[test]
    fn test_rect_partial_coverage_writes_nothing() {
        let mut buf = OccluderHiZ::new(64, 64);
        // Straddles all four cells, fully covers none of them.
        buf.rasterize_rect_occluder(16, 16, 47, 47, 5.0);
        assert_eq!(buf.covered_cell_count(), 0);
        buf.build_pyramid();
        // A box right in the middle, well behind the rect, stays visible:
        // the rect never *provably* covers any whole cell.
        assert!(buf.is_potentially_visible(box_at(16, 47, 16, 47, 10.0)));
    }

    #[test]
    fn test_wall_culls_behind_keeps_front_and_side() {
        let mut buf = OccluderHiZ::new(640, 360);
        buf.rasterize_rect_occluder(0, 0, 319, 359, 5.0); // Left-half wall.
        buf.build_pyramid();
        assert!(buf.is_valid());

        assert!(buf.is_occluded(box_at(32, 96, 32, 96, 10.0))); // Behind wall.
        assert!(buf.is_potentially_visible(box_at(32, 96, 32, 96, 2.0))); // In front.
        assert!(buf.is_potentially_visible(box_at(352, 416, 32, 96, 10.0))); // Beside wall.
        // Straddling the wall's edge: the uncovered half keeps it visible.
        assert!(buf.is_potentially_visible(box_at(288, 352, 32, 96, 10.0)));
        // Level with the wall is not behind it.
        assert!(buf.is_potentially_visible(box_at(32, 96, 32, 96, 5.0)));
    }

    #[test]
    fn test_partially_occluded_box_is_not_culled() {
        // Regression guard for the soundness invariant: a closer
        // occluder over *part* of the footprint must never cull the box,
        // no matter how close that part is.
        let mut buf = OccluderHiZ::with_tile_size(128, 128, 16);
        buf.rasterize_rect_occluder(0, 0, 63, 127, 1.0); // Very close, left half only.
        buf.build_pyramid();
        assert!(buf.is_potentially_visible(box_at(32, 96, 32, 96, 50.0)));
        assert!(!buf.is_occluded(box_at(32, 96, 32, 96, 50.0)));
    }

    #[test]
    fn test_aabb_occluder_uses_front_face_depth() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_aabb_occluder(&AABB3D::new(0, 63, 0, 63, 5.0, 9.0));
        assert_eq!(buf.cell_depth(0, 0), Some(5.0));
        buf.build_pyramid();
        // Depth 6 is inside the solid box (front face at 5): occluded.
        assert!(buf.is_occluded(box_at(0, 31, 0, 31, 6.0)));
        assert!(buf.is_potentially_visible(box_at(0, 31, 0, 31, 4.0)));
    }

    #[test]
    fn test_overlapping_occluders_keep_minimum_candidate() {
        let mut a = OccluderHiZ::new(64, 64);
        a.rasterize_rect_occluder(0, 0, 63, 63, 8.0);
        a.rasterize_rect_occluder(0, 0, 63, 63, 5.0);
        assert_eq!(a.cell_depth(1, 1), Some(5.0));

        let mut b = OccluderHiZ::new(64, 64);
        b.rasterize_rect_occluder(0, 0, 63, 63, 5.0);
        b.rasterize_rect_occluder(0, 0, 63, 63, 8.0);
        assert_eq!(b.cell_depth(1, 1), Some(5.0));
    }

    #[test]
    fn test_slanted_triangle_uses_farthest_corner() {
        // Depth plane: 2 at y=0 rising to 10 at y=128.
        let mut buf = OccluderHiZ::new(128, 128);
        buf.rasterize_triangle_occluder(
            ScreenVertex::new(0.0, 0.0, 2.0),
            ScreenVertex::new(128.0, 0.0, 2.0),
            ScreenVertex::new(0.0, 128.0, 10.0),
        );
        // Cell (0,0) spans y in [0,32]: farthest corner depth is
        // 2 + (8/128)*32 = 4.
        let d = buf.cell_depth(0, 0).expect("cell exists");
        assert!((d - 4.0).abs() < 1e-4, "cell depth {d}");
        // Cell (3,0)'s corner (128,32) is outside the triangle
        // (x + y = 160 > 128): not covered.
        assert_eq!(buf.cell_depth(3, 0), Some(f32::INFINITY));

        buf.build_pyramid();
        // Behind the far corner everywhere in cell (0,0): culled.
        assert!(buf.is_occluded(box_at(0, 31, 0, 31, 4.5)));
        // In front of the far corner (but behind the near edge): the
        // slanted surface pokes in front of the box somewhere — visible.
        assert!(buf.is_potentially_visible(box_at(0, 31, 0, 31, 3.5)));
    }

    #[test]
    fn test_triangle_winding_does_not_matter() {
        // Regression: barycentric weights must be sub-area / total area.
        // Dividing by the sign-normalized area flipped the weights of
        // negatively-wound triangles, so they covered nothing at all.
        let tri = [
            ScreenVertex::new(0.0, 0.0, 2.0),
            ScreenVertex::new(128.0, 0.0, 2.0),
            ScreenVertex::new(0.0, 128.0, 10.0),
        ];
        let mut ccw = OccluderHiZ::new(128, 128);
        ccw.rasterize_triangle_occluder(tri[0], tri[1], tri[2]);
        let mut cw = OccluderHiZ::new(128, 128);
        cw.rasterize_triangle_occluder(tri[0], tri[2], tri[1]);
        assert_eq!(ccw.covered_cell_count(), cw.covered_cell_count());
        assert!(cw.covered_cell_count() > 0);
        for cy in 0..4 {
            for cx in 0..4 {
                assert_eq!(
                    ccw.cell_depth(cx, cy).map(f32::to_bits),
                    cw.cell_depth(cx, cy).map(f32::to_bits),
                    "cell ({cx},{cy})"
                );
            }
        }
    }

    #[test]
    fn test_small_triangle_contributes_nothing() {
        let mut buf = OccluderHiZ::new(128, 128);
        buf.rasterize_triangle_occluder(
            ScreenVertex::new(40.0, 40.0, 1.0),
            ScreenVertex::new(56.0, 40.0, 1.0),
            ScreenVertex::new(40.0, 56.0, 1.0),
        );
        assert_eq!(buf.covered_cell_count(), 0);
    }

    #[test]
    fn test_degenerate_and_nonfinite_occluders_ignored() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_triangle_occluder(
            ScreenVertex::new(0.0, 0.0, 1.0),
            ScreenVertex::new(32.0, 0.0, 1.0),
            ScreenVertex::new(64.0, 0.0, 1.0), // Collinear.
        );
        buf.rasterize_rect_occluder(0, 0, 63, 63, f32::INFINITY);
        buf.rasterize_rect_occluder(0, 0, 63, 63, f32::NAN);
        assert_eq!(buf.covered_cell_count(), 0);
    }

    #[test]
    fn test_invalid_pyramid_until_built_and_after_writes() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_rect_occluder(0, 0, 63, 63, 5.0);
        let behind = box_at(0, 31, 0, 31, 10.0);
        assert!(buf.is_potentially_visible(behind)); // Not built yet.
        buf.build_pyramid();
        assert!(buf.is_occluded(behind));
        buf.rasterize_rect_occluder(0, 0, 31, 31, 1.0); // Any write invalidates.
        assert!(buf.is_potentially_visible(behind));
        buf.build_pyramid();
        buf.clear();
        assert_eq!(buf.covered_cell_count(), 0);
        assert!(buf.is_potentially_visible(behind));
    }

    #[test]
    fn test_offscreen_and_partially_offscreen() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_rect_occluder(-32, -32, 31, 31, 4.0);
        assert_eq!(buf.cell_depth(0, 0), Some(4.0)); // Cell (0,0) fully covered.
        buf.build_pyramid();

        let offscreen = box_at(-100, -10, -100, -10, 10.0);
        assert!(!buf.is_potentially_visible(offscreen)); // Nothing to draw…
        assert!(!buf.is_occluded(offscreen)); // …but not *occluded*.

        // Half off-screen, half behind the occluder: occluded.
        assert!(buf.is_occluded(box_at(-16, 31, 0, 31, 10.0)));
    }

    #[test]
    fn test_odd_dimensions_partial_edge_cells() {
        let mut buf = OccluderHiZ::new(100, 50); // Grid 4x2, edge cells partial.
        assert_eq!(buf.grid_dimensions(), (4, 2));
        buf.rasterize_rect_occluder(0, 0, 99, 49, 3.0);
        // Only cells fully inside the rect: columns 0..=2 of row 0.
        // Column 3 spans pixels 96..=127 (rect ends at 99) and row 1
        // spans 32..=63 (rect ends at 49), so neither is fully covered.
        assert_eq!(buf.covered_cell_count(), 3);
        buf.build_pyramid();
        assert!(buf.is_occluded(box_at(0, 31, 0, 31, 5.0)));
        assert!(buf.is_potentially_visible(box_at(96, 99, 0, 31, 5.0)));
    }

    #[test]
    fn test_pyramid_top_is_max_and_inf_poisoned() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_rect_occluder(0, 0, 31, 63, 7.0); // Left column of cells.
        buf.rasterize_rect_occluder(32, 0, 63, 31, 3.0); // Top-right cell.
        buf.build_pyramid();
        // Bottom-right cell uncovered -> top of pyramid is INFINITY.
        let top = buf.levels.last().expect("top level");
        assert!(top.depths[0].is_infinite());

        buf.rasterize_rect_occluder(32, 32, 63, 63, 9.0);
        buf.build_pyramid();
        let top = buf.levels.last().expect("top level");
        assert!((top.depths[0] - 9.0).abs() < f32::EPSILON); // Max over all cells.
    }

    #[test]
    fn test_batch_cull_matches_scalar_queries() {
        let mut buf = OccluderHiZ::new(128, 128);
        buf.rasterize_rect_occluder(0, 0, 63, 127, 5.0);
        buf.build_pyramid();
        let boxes = [
            box_at(0, 31, 0, 31, 10.0),    // Occluded.
            box_at(0, 31, 0, 31, 1.0),     // In front.
            box_at(64, 95, 0, 31, 10.0),   // No occluder there.
            box_at(32, 95, 0, 31, 10.0),   // Straddles edge.
            box_at(-50, -1, -50, -1, 1.0), // Off-screen.
        ];
        let batch = buf.cull_aabbs(&boxes);
        for (b, visible) in boxes.iter().zip(batch.iter()) {
            assert_eq!(*visible, buf.is_potentially_visible(*b));
        }
        assert_eq!(batch, vec![false, true, true, true, false]);

        let mut out = [true; 5];
        buf.cull_aabbs_prealloc(&boxes, &mut out);
        assert_eq!(out, batch.as_slice());
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn test_batch_length_mismatch_panics() {
        let buf = OccluderHiZ::new(64, 64);
        let mut out = [true; 1];
        buf.cull_aabbs_prealloc(
            &[box_at(0, 1, 0, 1, 1.0), box_at(0, 1, 0, 1, 1.0)],
            &mut out,
        );
    }

    #[test]
    fn test_nan_query_depth_stays_visible() {
        let mut buf = OccluderHiZ::new(64, 64);
        buf.rasterize_rect_occluder(0, 0, 63, 63, 5.0);
        buf.build_pyramid();
        assert!(buf.is_potentially_visible(box_at(0, 31, 0, 31, f32::NAN)));
    }

    /// Deterministic PRNG (same LCG style as the culling tests).
    struct Lcg(u32);
    impl Lcg {
        fn next_f32(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (self.0 as f32) / (u32::MAX as f32)
        }
        fn range(&mut self, lo: i32, hi: i32) -> i32 {
            lo + (self.next_f32() * ((hi - lo) as f32)) as i32
        }
    }

    #[test]
    fn test_pyramid_matches_naive_max_reduction() {
        // Random base grid, then every pyramid level must equal the
        // naive 2x2 max-reduction of the level below — this exercises
        // whichever reduction kernel (AVX2 or scalar) the host picked.
        let mut rng = Lcg(0x1234_5678);
        let mut buf = OccluderHiZ::with_tile_size(256, 192, 16);
        for _ in 0..40 {
            let x = rng.range(0, 15) * 16;
            let y = rng.range(0, 11) * 16;
            let w = rng.range(1, 4) * 16 - 1;
            let depth = 1.0 + rng.next_f32() * 20.0;
            buf.rasterize_rect_occluder(x, y, x + w, y + w, depth);
        }
        buf.build_pyramid();
        for level_idx in 1..buf.levels.len() {
            let (src, dst) = (&buf.levels[level_idx - 1], &buf.levels[level_idx]);
            for dy in 0..dst.height {
                for dx in 0..dst.width {
                    let mut expect = f32::NEG_INFINITY;
                    for sy in (dy * 2)..=((dy * 2 + 1).min(src.height - 1)) {
                        for sx in (dx * 2)..=((dx * 2 + 1).min(src.width - 1)) {
                            expect = expect.max(src.get(sx, sy));
                        }
                    }
                    // Max-reduction copies values, so equality must
                    // hold bit-for-bit.
                    assert_eq!(
                        dst.get(dx, dy).to_bits(),
                        expect.to_bits(),
                        "level {level_idx} cell ({dx},{dy})"
                    );
                }
            }
            // region_max over the full grid equals the level's own max.
            let (gw, gh) = buf.grid_dimensions();
            let full = buf.region_max(0, 0, gw - 1, gh - 1);
            let level_max = buf.levels[0]
                .depths
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(full.to_bits(), level_max.to_bits());
        }
    }

    #[test]
    fn test_region_max_matches_brute_force() {
        let mut rng = Lcg(0x0bad_f00d);
        let mut buf = OccluderHiZ::with_tile_size(256, 192, 16);
        for _ in 0..30 {
            let x = rng.range(0, 15) * 16;
            let y = rng.range(0, 11) * 16;
            let depth = 1.0 + rng.next_f32() * 20.0;
            buf.rasterize_rect_occluder(x, y, x + 31, y + 31, depth);
        }
        buf.build_pyramid();
        let (gw, gh) = buf.grid_dimensions();
        for _ in 0..200 {
            let cx0 = rng.range(0, gw as i32 - 1);
            let cy0 = rng.range(0, gh as i32 - 1);
            let cx1 = cx0 + rng.range(0, 4);
            let cy1 = cy0 + rng.range(0, 4);
            let (cx1, cy1) = (cx1.min(gw as i32 - 1), cy1.min(gh as i32 - 1));
            let got = buf.region_max(cx0 as u32, cy0 as u32, cx1 as u32, cy1 as u32);
            let mut expect = f32::NEG_INFINITY;
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    expect = expect.max(buf.cell_depth(cx as u32, cy as u32).unwrap());
                }
            }
            assert_eq!(
                got.to_bits(),
                expect.to_bits(),
                "rect ({cx0},{cy0})-({cx1},{cy1})"
            );
        }
    }

    #[test]
    fn test_soundness_against_per_pixel_truth() {
        // The core guarantee, checked against a per-pixel model: if
        // the buffer culls a box, then at *every* pixel of the box an
        // occluder rect must sit strictly in front of the box.
        let mut rng = Lcg(0xfeed_face);
        let mut buf = OccluderHiZ::with_tile_size(128, 128, 16);
        let mut rects: Vec<(i32, i32, i32, i32, f32)> = Vec::new();
        for _ in 0..24 {
            let x = rng.range(0, 7) * 16;
            let y = rng.range(0, 7) * 16;
            let w = rng.range(1, 3) * 16 - 1;
            let depth = 1.0 + rng.next_f32() * 10.0;
            buf.rasterize_rect_occluder(x, y, x + w, y + w, depth);
            rects.push((x, y, x + w, y + w, depth));
        }
        buf.build_pyramid();

        let mut culled = 0usize;
        for _ in 0..500 {
            let x = rng.range(0, 97);
            let y = rng.range(0, 97);
            let query = box_at(
                x,
                x + rng.range(1, 32),
                y,
                y + rng.range(1, 32),
                0.5 + rng.next_f32() * 14.0,
            );
            if !buf.is_occluded(query) {
                continue;
            }
            culled += 1;
            for py in query.min_y..=query.max_y {
                for px in query.min_x..=query.max_x {
                    let covered_in_front = rects.iter().any(|&(rx0, ry0, rx1, ry1, d)| {
                        px >= rx0 && px <= rx1 && py >= ry0 && py <= ry1 && d < query.min_depth
                    });
                    assert!(
                        covered_in_front,
                        "box {query:?} culled but pixel ({px},{py}) is not behind an occluder"
                    );
                }
            }
        }
        assert!(culled > 0, "property test must actually cull something");
    }
}
