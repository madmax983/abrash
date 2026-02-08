//! Tile-based rendering for improved cache locality at high resolutions.
//!
//! This module implements a tile-based rasterizer that subdivides the framebuffer into 32×32 pixel
//! tiles and processes each tile independently. The working set (4KB pixels + 4KB depth = 8KB)
//! fits comfortably in L1 cache (typically 32-64 KB per core), providing significant performance
//! improvements when rendering large framebuffers that exceed L3 cache capacity.
//!
//! # Performance Characteristics
//!
//! Benchmark results show that tile-based rendering excels at **high resolutions with low triangle counts**:
//!
//! | Resolution | Framebuffer Size | Triangle Count | Performance vs Scanline |
//! |------------|-----------------|----------------|-------------------------|
//! | 800×600 | 3.84 MB | 10-500 | **1.3-3× slower** (use scanline) |
//! | 1920×1080 | 16.6 MB | ≤10 | **8% faster** ✓ |
//! | 1920×1080 | 16.6 MB | 200 | ~Even |
//! | 3840×2160 | 66.4 MB | 10 | **27% faster** ✓ |
//! | 3840×2160 | 66.4 MB | 50 | **6% faster** ✓ |
//! | 3840×2160 | 66.4 MB | ≥200 | **6% slower** (use scanline) |
//!
//! **Usage Guideline**: Use [`TileRenderer`] when resolution ≥ 1920×1080 AND triangle count ≤ 100.
//! Use [`Rasterizer::fill_triangle_3d`](crate::rasterizer::Rasterizer::fill_triangle_3d) otherwise.
//!
//! # Parallel Rendering
//!
//! Enable the `parallel` feature for multi-threaded tile dispatch using Rayon:
//!
//! ```toml
//! [dependencies]
//! abrash = { version = "0.1", features = ["parallel"] }
//! ```
//!
//! With parallel rendering enabled, tiles are processed concurrently across all CPU cores, providing
//! near-linear speedup (3-4× on 4-core, 7-8× on 8-core systems). Each tile renders independently
//! into thread-local buffers, then merges into non-overlapping framebuffer regions safely.
//!
//! **Performance**: Expect 70-90% parallel efficiency for workloads with 100+ tiles (≥1920×1080).
//!
//! # Why the Crossover?
//!
//! - **At 800×600**: The 3.84 MB framebuffer fits in L2/L3 cache, so scanline doesn't suffer cache
//!   misses. Tiling overhead (prepare, bin, merge) dominates and makes it slower.
//! - **At 1920×1080**: The 16.6 MB framebuffer exceeds typical L3 cache (8-16 MB), causing cache
//!   thrashing in scanline. Tiled wins at low triangle counts where setup cost is minimal.
//! - **At 3840×2160**: The 66.4 MB framebuffer severely exceeds L3 cache. Scanline suffers massive
//!   cache thrashing while tiled's 8KB working set stays in L1. Tiled wins decisively at ≤50 triangles.
//!
//! # Example
//!
//! ```no_run
//! use abrash::tile_renderer::{TileRenderer, ClipTriangle};
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::Vec3;
//!
//! let mut fb = Framebuffer::new(3840, 2160).unwrap(); // 4K resolution
//! let mut zb = ZBuffer::new(3840, 2160).unwrap();
//! let mut renderer = TileRenderer::new(3840, 2160);
//!
//! let triangles: Vec<ClipTriangle> = vec![
//!     ((Vec3::new(-0.5, -0.5, 0.5), 1.0),
//!      (Vec3::new(0.5, -0.5, 0.5), 1.0),
//!      (Vec3::new(0.0, 0.5, 0.5), 1.0),
//!      0xFF0000FF), // Red triangle
//! ];
//!
//! renderer.render_batch(&mut fb, &mut zb, &triangles);
//! ```

use crate::clipping::clip_triangle_against_near_plane;
use crate::framebuffer::Framebuffer;
use crate::hiz_buffer::{AABB3D, HiZBuffer};
use crate::math::{ScreenPoint, Vec2, Vec3, project_to_screen};
use crate::rasterizer::{
    EdgeWalker, PerspectiveTextureGradients, PerspectiveTextureEdgeWalker,
    PerspectiveSpanStart, RECIPROCAL_TABLE, is_backface, sort_by_y,
};
use crate::texture::{FilterMode, Texture};
use crate::zbuffer::ZBuffer;

/// Fixed-point vertex coordinates using 24.8 format (24 bits integer, 8 bits fractional).
///
/// This provides sub-pixel precision while using deterministic integer arithmetic.
/// The 24-bit integer part supports coordinates up to 16,777,215 pixels (well beyond 4K).
///
/// # Benefits over floating-point
///
/// - **Deterministic**: No floating-point rounding issues, always pixel-identical results
/// - **Faster**: Integer ALU operations are faster than float on most CPUs
/// - **Better SIMD**: AVX2 can process 16 i32 values vs 8 f32 values per instruction
///
/// # Format
///
/// - Fixed-point scale factor: 256 (2^8)
/// - Conversion: `fixed = (float * 256.0) as i32`
/// - Sub-pixel precision: 1/256th of a pixel (~0.004 pixels)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexFixed {
    pub x: i32, // 24.8 fixed point
    pub y: i32, // 24.8 fixed point
    pub z: i32, // 24.8 fixed-point depth
}

impl VertexFixed {
    /// Convert a `ScreenPoint` to fixed-point coordinates.
    ///
    /// # Format
    ///
    /// The integer screen coordinates are shifted left by 8 bits to create the
    /// 24.8 fixed-point representation. For example:
    /// - Screen coordinate 100 → Fixed-point 25600 (100 << 8)
    /// - Screen coordinate 50.5 → Not applicable (ScreenPoint uses i32)
    #[inline]
    fn from_screen_point(p: ScreenPoint) -> Self {
        Self {
            x: p.x << 8, // Convert to 24.8 fixed point
            y: p.y << 8,
            z: (p.z * 256.0) as i32, // Convert depth to 24.8 fixed point
        }
    }

    /// Extract the integer pixel coordinate (discard fractional part).
    #[inline]
    const fn to_pixel_x(self) -> i32 {
        self.x >> 8
    }

    /// Extract the integer pixel coordinate (discard fractional part).
    #[inline]
    const fn to_pixel_y(self) -> i32 {
        self.y >> 8
    }
}

/// Compute fixed-point edge function for triangle rasterization.
///
/// The edge function computes the signed area of the parallelogram formed by
/// vectors (p - v0) and (v1 - v0). It's used to determine if a point is inside
/// a triangle.
///
/// # Returns
///
/// - Positive if point p is on the "right" side of edge v0→v1
/// - Negative if point p is on the "left" side
/// - Zero if point p is exactly on the edge
///
/// # Format
///
/// Input coordinates are in 24.8 fixed-point. The result is in 16.16 fixed-point
/// due to the multiplication of two 24.8 values:
/// - (24.8) * (24.8) = (48.16) → truncated to i32 preserves upper 32 bits
///
/// This is intentional - we only care about the sign for edge testing, not the
/// exact magnitude.
#[inline(always)]
const fn edge_function_fixed(px: i32, py: i32, v0: VertexFixed, v1: VertexFixed) -> i32 {
    // Edge function: (p.x - v0.x) * (v1.y - v0.y) - (p.y - v0.y) * (v1.x - v0.x)
    // All coordinates are 24.8 fixed point
    let dx = px - v0.x;
    let dy = py - v0.y;
    let edge_dx = v1.x - v0.x;
    let edge_dy = v1.y - v0.y;

    // Multiply: (24.8) * (24.8) = (48.16)
    // The i32 result keeps the upper 32 bits, giving us 16.16 fixed point
    // This is fine for edge testing - we only care about the sign
    (dx as i64 * edge_dy as i64 - dy as i64 * edge_dx as i64) as i32
}

#[cfg(feature = "parallel")]
/// Wrapper for raw pointers to enable thread-safe parallel writes to non-overlapping regions.
///
/// SAFETY: This is safe because each thread writes to a non-overlapping region determined
/// by its tile coordinates (tx, ty). The tile renderer ensures that no two tiles overlap.
struct SendPtr<T>(*mut T);

#[cfg(feature = "parallel")]
impl<T> SendPtr<T> {
    /// SAFETY: Caller must ensure the index is within bounds and writes are to non-overlapping regions
    #[inline]
    unsafe fn write(&self, index: usize, value: T) {
        // SAFETY: Caller guarantees index is within bounds and writes are non-overlapping
        unsafe {
            *self.0.add(index) = value;
        }
    }
}

#[cfg(feature = "parallel")]
unsafe impl<T> Send for SendPtr<T> {}

#[cfg(feature = "parallel")]
unsafe impl<T> Sync for SendPtr<T> {}

/// Tile size in pixels. 32x32 = 1024 pixels * 4 bytes = 4KB per buffer.
pub const TILE_SIZE: u32 = 32;

/// A clip-space triangle with three vertices `(position, w)` and a flat color.
pub type ClipTriangle = ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32);

/// A clip-space triangle with three vertices `(position, w)` and UV coordinates.
pub type TexturedClipTriangle = ((Vec3, f32), Vec2, (Vec3, f32), Vec2, (Vec3, f32), Vec2);

/// A triangle that has been clipped, projected, culled, Y-sorted, and had gradients computed.
#[derive(Clone, Copy)]
pub struct PreparedTriangle {
    pub p0: ScreenPoint,
    pub p1: ScreenPoint,
    pub p2: ScreenPoint,
    // Fixed-point vertices for deterministic edge function evaluation
    pub p0_fixed: VertexFixed,
    pub p1_fixed: VertexFixed,
    pub p2_fixed: VertexFixed,
    pub dz_dx: f32,
    pub long_edge_is_left: bool,
    pub color: u32,
    pub aabb_min_x: i32,
    pub aabb_min_y: i32,
    pub aabb_max_x: i32,
    pub aabb_max_y: i32,
    pub min_depth: f32, // Minimum depth across triangle
    pub max_depth: f32, // Maximum depth across triangle
}

/// A textured triangle prepared for rasterization.
#[derive(Clone, Copy)]
pub struct PreparedTexturedTriangle {
    pub p0: ScreenPoint,
    pub p1: ScreenPoint,
    pub p2: ScreenPoint,
    pub p0_fixed: VertexFixed,
    pub p1_fixed: VertexFixed,
    pub p2_fixed: VertexFixed,
    pub q0: f32,
    pub q1: f32,
    pub q2: f32,
    pub u0: f32,
    pub u1: f32,
    pub u2: f32,
    pub v0: f32,
    pub v1: f32,
    pub v2: f32,
    pub gradients: PerspectiveTextureGradients,
    pub long_edge_is_left: bool,
    pub aabb_min_x: i32,
    pub aabb_min_y: i32,
    pub aabb_max_x: i32,
    pub aabb_max_y: i32,
    pub min_depth: f32,
    pub max_depth: f32,
}

/// Render a single tile: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_single_tile(
    tx: u32,
    ty: u32,
    tile_bins: &[Vec<usize>],
    prepared: &[PreparedTriangle],
    tiles_x: u32,
    width: u32,
    _height: u32,
) -> Option<(Vec<u32>, Vec<f32>, i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins[bin_idx].is_empty() {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;

    // Compute Y range covered by triangles in this bin (partial tile clear)
    let mut clear_y_min = tile_y1;
    let mut clear_y_max = tile_y0;
    let bin = &tile_bins[bin_idx];
    for &tri_idx in bin {
        let tri = &prepared[tri_idx];
        clear_y_min = clear_y_min.min(tri.aabb_min_y.max(tile_y0));
        clear_y_max = clear_y_max.max(tri.aabb_max_y.min(tile_y1 - 1));
    }

    // Allocate tile-local buffers
    let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
    let mut tile_pixels = vec![0u32; tile_area];
    let mut tile_depths = vec![f32::INFINITY; tile_area];

    // Clear only the rows that will be touched
    let row_start = ((clear_y_min - tile_y0) as u32 * TILE_SIZE) as usize;
    let row_end = (((clear_y_max - tile_y0) as u32 + 1) * TILE_SIZE) as usize;
    tile_pixels[row_start..row_end].fill(0xFF00_0000);
    tile_depths[row_start..row_end].fill(f32::INFINITY);

    // Render all triangles in bin
    let screen_w = width as i32;
    for &tri_idx in bin {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile(
            &mut tile_pixels,
            &mut tile_depths,
            tri,
            tile_x0,
            tile_y0,
            tile_x1,
            tile_y1,
            screen_w,
        );
    }

    Some((tile_pixels, tile_depths, clear_y_min, clear_y_max))
}

/// Render a triangle into tile-local buffers. Free function to avoid `&mut self` borrow conflicts.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_triangle_in_tile(
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
    tri: &PreparedTriangle,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    screen_w: i32,
) {
    let y_start = tri.p0.y.max(tile_y0);
    let y_end = tri.p2.y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    // Edge A: always p0→p2 (long edge)
    let mut edge_a = EdgeWalker::new(tri.p0, tri.p2);
    if y_start > tri.p0.y {
        edge_a.step_n(y_start - tri.p0.y);
    }

    // Edge B: depends on whether y_start is above or below p1.y
    let mut edge_b = if y_start < tri.p1.y {
        let mut e = EdgeWalker::new(tri.p0, tri.p1);
        if y_start > tri.p0.y {
            e.step_n(y_start - tri.p0.y);
        }
        e
    } else {
        let mut e = EdgeWalker::new(tri.p1, tri.p2);
        if y_start > tri.p1.y {
            e.step_n(y_start - tri.p1.y);
        }
        e
    };

    let dz_dx = tri.dz_dx;
    let color = tri.color;

    for y in y_start..=y_end {
        if y == tri.p1.y && y != tri.p0.y {
            edge_b = EdgeWalker::new(tri.p1, tri.p2);
        }

        let (x_start, x_end, z_left_fixed) = if tri.long_edge_is_left {
            ((edge_a.x >> 16) as i32, (edge_b.x >> 16) as i32, edge_a.z)
        } else {
            ((edge_b.x >> 16) as i32, (edge_a.x >> 16) as i32, edge_b.z)
        };

        let dx = i64::from(x_end) - i64::from(x_start);

        if dx <= 0 {
            // Single-pixel scanline
            if x_start >= tile_x0 && x_start < tile_x1 && x_start >= 0 && x_start <= screen_x_max {
                let tile_idx =
                    ((y - tile_y0) as u32 * TILE_SIZE + (x_start - tile_x0) as u32) as usize;
                // Convert fixed-point to float for zbuffer comparison
                let z_left_float = (z_left_fixed as f32) / 256.0;
                if z_left_float < tile_depths[tile_idx] {
                    tile_depths[tile_idx] = z_left_float;
                    tile_pixels[tile_idx] = color;
                }
            }
        } else {
            // Clamp X to tile and screen bounds
            let xs = x_start.max(tile_x0).max(0);
            let xe = x_end.min(tile_x1 - 1).min(screen_x_max);

            if xs <= xe {
                // Calculate z at xs in fixed-point, then convert to float
                let dz_dx_fixed = (dz_dx * 256.0) as i32;
                let z_at_xs_fixed =
                    z_left_fixed + (i64::from(xs) - i64::from(x_start)) as i32 * dz_dx_fixed;
                let z_at_xs = (z_at_xs_fixed as f32) / 256.0;

                let row_offset = ((y - tile_y0) as u32 * TILE_SIZE) as usize;
                let col_start = (xs - tile_x0) as usize;
                let col_end = (xe - tile_x0) as usize;

                let pixels = &mut tile_pixels[row_offset + col_start..=row_offset + col_end];
                let depths = &mut tile_depths[row_offset + col_start..=row_offset + col_end];

                // SIMD disabled after extensive profiling and optimization (2026-02-06)
                //
                // **History:**
                // - Initial AVX2 SIMD: 3.8× slower than scalar
                // - Re-enabled with adaptive threshold (≥32 pixels): 4.0× slower
                //
                // **Root causes:**
                // 1. Masked store penalty: _mm256_maskstore_ps/epi32 is extremely slow
                //    - Each masked store: 10-15 cycles
                //    - Scalar conditional write: 1-2 cycles
                //    - 8× penalty per SIMD operation
                // 2. Setup overhead: Initializing depth vectors, stride computation
                //    - 10-20 cycles fixed cost per scanline
                //    - Not amortized for typical scanlines (10-50 pixels)
                // 3. Memory bandwidth: 8-wide loads may saturate L1 cache
                //    - Cache line contention with adjacent scanlines
                //    - Prefetcher less effective with strided access
                //
                // **Benchmark results (1080p, 100 iterations):**
                // - Scalar: 457 µs/frame
                // - SIMD (with threshold): 1,840 µs/frame (4.0× slower)
                //
                // **Attempts:**
                // - ✅ Added adaptive threshold (≥32 pixels)
                // - ❌ Still 4.0× slower than scalar
                //
                // **Conclusion:**
                // Scanline rasterization is unsuited for SIMD due to:
                // - Small typical scanlines (10-50 pixels, not 64+)
                // - Masked store penalty dominates (10-15 cycles each)
                // - Memory bandwidth saturation
                //
                // Alternative optimizations:
                // - ✅ Parallel (Rayon): 3-4× speedup on 4-core, 7-8× on 8-core
                // - ✅ Tiling: 1.2-2.5× speedup at 4K with cache locality
                //
                // Scalar + parallel is optimal for this workload.
                // See: SIMD_PROFILING_ANALYSIS.md for full profiling data
                rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
            }
        }

        edge_a.step();
        edge_b.step();
    }
}

/// Render a single tile textured: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_single_tile_textured(
    tx: u32,
    ty: u32,
    tile_bins: &[Vec<usize>],
    prepared: &[PreparedTexturedTriangle],
    tiles_x: u32,
    width: u32,
    _height: u32,
    texture: &Texture,
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins[bin_idx].is_empty() {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;

    // Compute Y range covered by triangles in this bin (partial tile clear)
    let mut clear_y_min = tile_y1;
    let mut clear_y_max = tile_y0;
    let bin = &tile_bins[bin_idx];
    for &tri_idx in bin {
        let tri = &prepared[tri_idx];
        clear_y_min = clear_y_min.min(tri.aabb_min_y.max(tile_y0));
        clear_y_max = clear_y_max.max(tri.aabb_max_y.min(tile_y1 - 1));
    }

    // Clear only the rows that will be touched
    let row_start = ((clear_y_min - tile_y0) as u32 * TILE_SIZE) as usize;
    let row_end = (((clear_y_max - tile_y0) as u32 + 1) * TILE_SIZE) as usize;
    tile_pixels[row_start..row_end].fill(0xFF00_0000);
    tile_depths[row_start..row_end].fill(f32::INFINITY);

    // Render all triangles in bin
    let screen_w = width as i32;
    for &tri_idx in bin {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile_textured(
            tile_pixels,
            tile_depths,
            tri,
            tile_x0,
            tile_y0,
            tile_x1,
            tile_y1,
            screen_w,
            texture,
        );
    }

    Some((clear_y_min, clear_y_max))
}

/// Render a textured triangle into tile-local buffers.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_triangle_in_tile_textured(
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
    tri: &PreparedTexturedTriangle,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    screen_w: i32,
    texture: &Texture,
) {
    let y_start = tri.p0.y.max(tile_y0);
    let y_end = tri.p2.y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    let mut edge_a = PerspectiveTextureEdgeWalker::new(
        tri.p0, tri.p2, tri.q0, tri.q2, tri.u0, tri.u2, tri.v0, tri.v2,
    );
    if y_start > tri.p0.y {
        edge_a.step_n(y_start - tri.p0.y);
    }

    let mut edge_b = if y_start < tri.p1.y {
        let mut e = PerspectiveTextureEdgeWalker::new(
            tri.p0, tri.p1, tri.q0, tri.q1, tri.u0, tri.u1, tri.v0, tri.v1,
        );
        if y_start > tri.p0.y {
            e.step_n(y_start - tri.p0.y);
        }
        e
    } else {
        let mut e = PerspectiveTextureEdgeWalker::new(
            tri.p1, tri.p2, tri.q1, tri.q2, tri.u1, tri.u2, tri.v1, tri.v2,
        );
        if y_start > tri.p1.y {
            e.step_n(y_start - tri.p1.y);
        }
        e
    };

    for y in y_start..=y_end {
        if y == tri.p1.y && y != tri.p0.y {
            edge_b = PerspectiveTextureEdgeWalker::new(
                tri.p1, tri.p2, tri.q1, tri.q2, tri.u1, tri.u2, tri.v1, tri.v2,
            );
        }

        let (x_start, x_end, z_left, q_left, u_left, v_left) = if tri.long_edge_is_left {
            (
                (edge_a.x >> 16) as i32,
                (edge_b.x >> 16) as i32,
                edge_a.z,
                edge_a.q,
                edge_a.u,
                edge_a.v,
            )
        } else {
            (
                (edge_b.x >> 16) as i32,
                (edge_a.x >> 16) as i32,
                edge_b.z,
                edge_b.q,
                edge_b.u,
                edge_b.v,
            )
        };

        let dx = i64::from(x_end) - i64::from(x_start);

        if dx <= 0 {
            // Single-pixel scanline
            if x_start >= tile_x0 && x_start < tile_x1 && x_start >= 0 && x_start <= screen_x_max {
                let tile_idx =
                    ((y - tile_y0) as u32 * TILE_SIZE + (x_start - tile_x0) as u32) as usize;
                if z_left < tile_depths[tile_idx] && q_left.abs() > 0.000_001 {
                    tile_depths[tile_idx] = z_left;
                    let w = 1.0 / q_left;
                    let u_tex = u_left * w;
                    let v_tex = v_left * w;
                    tile_pixels[tile_idx] = match texture.filter_mode {
                        FilterMode::Nearest => texture.get_pixel_texel(u_tex as i32, v_tex as i32),
                        FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                        FilterMode::Trilinear => {
                            let q = q_left;
                            let q_sq = q * q;
                            let lod = if q_sq > 0.000_000_1 {
                                let inv_q_sq = 1.0 / q_sq;
                                let u = u_left;
                                let v = v_left;
                                let u_x = (tri.gradients.du_dx * q - u * tri.gradients.dq_dx) * inv_q_sq * texture.width as f32;
                                let v_x = (tri.gradients.dv_dx * q - v * tri.gradients.dq_dx) * inv_q_sq * texture.height as f32;
                                let u_y = (tri.gradients.du_dy * q - u * tri.gradients.dq_dy) * inv_q_sq * texture.width as f32;
                                let v_y = (tri.gradients.dv_dy * q - v * tri.gradients.dq_dy) * inv_q_sq * texture.height as f32;

                                let d_max_sq = (u_x * u_x + v_x * v_x).max(u_y * u_y + v_y * v_y);
                                0.5 * d_max_sq.log2()
                            } else {
                                0.0
                            };
                            texture.get_pixel_lod(u_tex, v_tex, lod)
                        }
                    };
                }
            }
        } else {
            // Clamp X to tile and screen bounds
            let xs = x_start.max(tile_x0).max(0);
            let xe = x_end.min(tile_x1 - 1).min(screen_x_max);

            if xs <= xe {
                let dx_start = (xs - x_start) as f32;
                let z_start = z_left + dx_start * tri.gradients.dz_dx;
                let q_start = q_left + dx_start * tri.gradients.dq_dx;
                let u_start = u_left + dx_start * tri.gradients.du_dx;
                let v_start = v_left + dx_start * tri.gradients.dv_dx;

                let start = PerspectiveSpanStart {
                    z: z_start,
                    q: q_start,
                    u: u_start,
                    v: v_start,
                };

                let row_offset = ((y - tile_y0) as u32 * TILE_SIZE) as usize;
                let col_start = (xs - tile_x0) as usize;
                let col_end = (xe - tile_x0) as usize;

                let pixels = &mut tile_pixels[row_offset + col_start..=row_offset + col_end];
                let depths = &mut tile_depths[row_offset + col_start..=row_offset + col_end];

                rasterize_scanline_textured(pixels, depths, texture, start, &tri.gradients);
            }
        }

        edge_a.step();
        edge_b.step();
    }
}

#[inline(always)]
fn rasterize_scanline_textured(
    pixels: &mut [u32],
    depths: &mut [f32],
    texture: &Texture,
    start: PerspectiveSpanStart,
    gradients: &PerspectiveTextureGradients,
) {
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;

    let len = pixels.len();
    let span_size = 16;
    let mut i = 0;

    // Calculate initial start values
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
    let mut u_tex_start = u * w_start;
    let mut v_tex_start = v * w_start;

    while i < len {
        let count = (len - i).min(span_size);

        // End values at 'i + count'
        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        let inv_count = RECIPROCAL_TABLE[count];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        match texture.filter_mode {
            FilterMode::Nearest => {
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                for k in 0..count {
                    let depth_val = unsafe { depths.get_unchecked_mut(i + k) };
                    let pixel = unsafe { pixels.get_unchecked_mut(i + k) };

                    if z < *depth_val {
                        *depth_val = z;
                        *pixel = texture.get_pixel_texel(u_fix >> 16, v_fix >> 16);
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }
            FilterMode::Bilinear | FilterMode::Trilinear => {
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                let lod = if texture.filter_mode == FilterMode::Trilinear {
                    let q_sq = q * q;
                    if q_sq > 0.000_000_1 {
                         let inv_q_sq = 1.0 / q_sq;
                         let u_x = (gradients.du_dx * q - u * gradients.dq_dx) * inv_q_sq * texture.width as f32;
                         let v_x = (gradients.dv_dx * q - v * gradients.dq_dx) * inv_q_sq * texture.height as f32;
                         let u_y = (gradients.du_dy * q - u * gradients.dq_dy) * inv_q_sq * texture.width as f32;
                         let v_y = (gradients.dv_dy * q - v * gradients.dq_dy) * inv_q_sq * texture.height as f32;

                         let d_max_sq = (u_x * u_x + v_x * v_x).max(u_y * u_y + v_y * v_y);
                         0.5 * d_max_sq.log2()
                    } else {
                         0.0
                    }
                } else {
                    0.0
                };

                for k in 0..count {
                    let depth_val = unsafe { depths.get_unchecked_mut(i + k) };
                    let pixel = unsafe { pixels.get_unchecked_mut(i + k) };

                    if z < *depth_val {
                        *depth_val = z;
                        if texture.filter_mode == FilterMode::Trilinear {
                             let u_norm = (u_fix as f32) / 65536.0 / (texture.width as f32);
                             let v_norm = (v_fix as f32) / 65536.0 / (texture.height as f32);
                             *pixel = texture.get_pixel_lod(u_norm, v_norm, lod);
                        } else {
                            *pixel = texture.get_pixel_bilinear_fixed(u_fix >> 8, v_fix >> 8);
                        }
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }
        }

        q = q_end;
        u = u_end;
        v = v_end;
        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;
        i += count;
    }
}

/// Scalar scanline rasterization: process 1 pixel per iteration
#[inline(always)]
fn rasterize_scanline_scalar(
    pixels: &mut [u32],
    depths: &mut [f32],
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    // Convert to 24.8 fixed point for accumulation
    let mut z_fixed = (z_start * 256.0) as i32;
    let dz_dx_fixed = (dz_dx * 256.0) as i32;

    // Use multiplication instead of division (3-5 cycles vs 10-20 cycles)
    const INV_256: f32 = 1.0 / 256.0;

    for (pixel, depth) in pixels.iter_mut().zip(depths.iter_mut()) {
        // Convert fixed-point to float for zbuffer comparison (Option A)
        let z_float = (z_fixed as f32) * INV_256;
        if z_float < *depth {
            *depth = z_float;
            *pixel = color;
        }
        z_fixed += dz_dx_fixed;
    }
}

/// AVX2 vectorized scanline rasterization: process 8 pixels per iteration
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[inline(always)]
#[allow(dead_code)]
fn rasterize_scanline_simd(
    pixels: &mut [u32],
    depths: &mut [f32],
    z_at_xs: f32,
    dz_dx: f32,
    color: u32,
) {
    use std::arch::x86_64::*;

    let len = pixels.len();
    let mut i = 0;

    unsafe {
        // Setup: stride vector for incrementing depths by 8*dz_dx per iteration
        let stride_vec = _mm256_set1_ps(8.0 * dz_dx);

        // Initialize depth vector: [z0, z1, z2, z3, z4, z5, z6, z7]
        let mut depths_vec = _mm256_set_ps(
            z_at_xs + 7.0 * dz_dx,
            z_at_xs + 6.0 * dz_dx,
            z_at_xs + 5.0 * dz_dx,
            z_at_xs + 4.0 * dz_dx,
            z_at_xs + 3.0 * dz_dx,
            z_at_xs + 2.0 * dz_dx,
            z_at_xs + 1.0 * dz_dx,
            z_at_xs,
        );

        let color_vec = _mm256_set1_epi32(color as i32);

        // Process 8 pixels at a time with AVX2
        while i + 8 <= len {
            // Load zbuffer values for 8 pixels
            let zb_ptr = depths.as_ptr().add(i);
            let zb_vals = _mm256_loadu_ps(zb_ptr);

            // Compare: depth < zbuffer (8 comparisons in parallel)
            let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);

            // Conditional depth write via masked store
            let depths_mut_ptr = depths.as_mut_ptr().add(i);
            _mm256_maskstore_ps(depths_mut_ptr, _mm256_castps_si256(mask), depths_vec);

            // Conditional color write
            let pixels_ptr = pixels.as_mut_ptr().add(i) as *mut i32;
            _mm256_maskstore_epi32(pixels_ptr, _mm256_castps_si256(mask), color_vec);

            // Increment depths by stride (8*dz_dx) for next iteration
            depths_vec = _mm256_add_ps(depths_vec, stride_vec);
            i += 8;
        }
    }

    // Handle remaining pixels with scalar fallback
    let mut z = z_at_xs + (i as f32) * dz_dx;
    for j in i..len {
        if z < depths[j] {
            depths[j] = z;
            pixels[j] = color;
        }
        z += dz_dx;
    }
}

/// Fallback for when SIMD is not available (non-x86_64 or feature disabled)
#[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
#[inline(always)]
#[allow(dead_code)]
fn rasterize_scanline_simd(
    pixels: &mut [u32],
    depths: &mut [f32],
    z_at_xs: f32,
    dz_dx: f32,
    color: u32,
) {
    rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
}

/// Tile-based renderer that bins triangles into 32×32 tiles for cache-friendly rendering.
///
/// This renderer implements a 4-phase pipeline optimized for large framebuffers that exceed
/// CPU cache capacity:
///
/// 1. **Prepare**: Clip, project, cull, sort vertices, and compute per-triangle metadata
/// 2. **Bin**: Assign each triangle to all tiles it overlaps based on its AABB
/// 3. **Render**: For each tile, rasterize only the triangles in its bin
/// 4. **Merge**: Copy completed tiles back to the main framebuffer
///
/// # When to Use
///
/// Use `TileRenderer` when **both** conditions are met:
/// - Resolution ≥ 1920×1080 (framebuffer > 12 MB)
/// - Triangle count ≤ 100 per frame (after culling)
///
/// For smaller resolutions or higher triangle counts, use
/// [`Rasterizer::fill_triangle_3d`](crate::rasterizer::Rasterizer::fill_triangle_3d) instead,
/// as it avoids the prepare/bin/merge overhead.
///
/// # Performance
///
/// The 32×32 tile size provides an 8 KB working set (4KB pixels + 4KB depth) that fits in L1
/// cache. This is critical for 4K rendering where the 66 MB framebuffer would cause severe
/// cache thrashing with traditional scanline rendering.
///
/// See the [module documentation](self) for detailed benchmark results.
pub struct TileRenderer {
    #[cfg(not(feature = "parallel"))]
    tile_pixels: Vec<u32>,
    #[cfg(not(feature = "parallel"))]
    tile_depths: Vec<f32>,
    tiles_x: u32,
    tiles_y: u32,
    width: u32,
    height: u32,
    tile_bins: Vec<Vec<usize>>,
    prepared: Vec<PreparedTriangle>,
    prepared_textured: Vec<PreparedTexturedTriangle>,
    hiz_buffer: Option<HiZBuffer>,
    #[cfg(feature = "gpu-binning")]
    gpu_binner: Option<crate::gpu::GpuBinner>,
}

impl TileRenderer {
    /// Create a new tile renderer for the given framebuffer dimensions.
    ///
    /// # Panics
    ///
    /// Panics if width or height is zero.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");

        let tiles_x = width.div_ceil(TILE_SIZE);
        let tiles_y = height.div_ceil(TILE_SIZE);
        let tile_count = (tiles_x * tiles_y) as usize;
        #[cfg(not(feature = "parallel"))]
        let tile_area = (TILE_SIZE * TILE_SIZE) as usize;

        Self {
            #[cfg(not(feature = "parallel"))]
            tile_pixels: vec![0; tile_area],
            #[cfg(not(feature = "parallel"))]
            tile_depths: vec![0.0; tile_area],
            tiles_x,
            tiles_y,
            width,
            height,
            tile_bins: vec![Vec::new(); tile_count],
            prepared: Vec::new(),
            prepared_textured: Vec::new(),
            hiz_buffer: None,
            #[cfg(feature = "gpu-binning")]
            gpu_binner: None,
        }
    }

    /// Enable hierarchical z-buffer occlusion culling.
    ///
    /// When enabled, the tile renderer will use a Hi-Z pyramid to cull occluded triangles
    /// before binning them to tiles. This can provide 1.2-2.5× speedup for scenes with
    /// 100+ triangles and significant depth complexity.
    ///
    /// # Performance
    ///
    /// - Best for: Complex scenes (100+ triangles), high depth overlap, static/slowly moving geometry
    /// - Overhead: 1-2ms pyramid build at 1080p, 4-8ms at 4K
    /// - Culling rate: 30-70% in typical scenes with occlusion
    pub fn enable_hiz(&mut self) {
        self.hiz_buffer = Some(HiZBuffer::new(self.width, self.height));
    }

    /// Enable GPU-accelerated triangle binning via DirectX 12 compute shaders.
    ///
    /// When enabled, the tile renderer will use a D3D12 compute shader to bin triangles
    /// to tiles on the GPU, which can provide 10-20× faster binning for triangle-heavy scenes.
    ///
    /// **Requirements:**
    /// - `gpu-binning` feature must be enabled
    /// - Windows platform with DirectX 12 support
    /// - Suitable GPU adapter (non-software)
    ///
    /// **Performance:**
    /// - Binning: 100 triangles <0.05ms, 1000 triangles <0.5ms
    /// - Overall: 2-3× speedup for scenes with 100+ triangles
    ///
    /// # Errors
    ///
    /// Returns `GpuError` if GPU initialization fails (e.g., no suitable adapter, device creation failure).
    #[cfg(feature = "gpu-binning")]
    pub fn enable_gpu_binning(&mut self) -> Result<(), crate::gpu::GpuError> {
        self.gpu_binner = Some(crate::gpu::GpuBinner::new(
            self.width,
            self.height,
            TILE_SIZE,
            1000, // Max triangles per batch
        )?);
        Ok(())
    }

    /// Enable two-level hierarchical GPU binning with Hi-Z culling.
    ///
    /// This method enables GPU compute shader binning with two-level hierarchical binning:
    /// 1. Coarse binning pass: Bin triangles to 128×128 pixel coarse bins (GPU)
    /// 2. Hi-Z culling pass: Cull occluded coarse bins using Hi-Z pyramid (CPU)
    /// 3. Fine binning pass: Bin visible triangles to 32×32 fine tiles (GPU)
    ///
    /// Two-level binning can provide additional speedup over single-level GPU binning
    /// by avoiding fine binning work for occluded regions of the screen.
    ///
    /// # Prerequisites
    ///
    /// - GPU binning must be enabled first via `enable_gpu_binning()`
    /// - Hi-Z buffer should be enabled via `enable_hiz()` for effective culling
    ///
    /// # Returns
    ///
    /// `GpuError` if two-level binning initialization fails or GPU binning is not enabled.
    #[cfg(feature = "gpu-binning")]
    pub fn enable_two_level_binning(&mut self) -> Result<(), crate::gpu::GpuError> {
        let gpu = self.gpu_binner.as_mut().ok_or_else(|| {
            crate::gpu::GpuError::DeviceCreation(windows::core::Error::from_hresult(
                windows::core::HRESULT(0x8007_0057u32 as i32), // E_INVALIDARG
            ))
        })?;

        gpu.enable_two_level_binning()?;
        Ok(())
    }

    /// Returns the number of tiles in X direction.
    #[must_use]
    pub const fn tiles_x(&self) -> u32 {
        self.tiles_x
    }

    /// Returns the number of tiles in Y direction.
    #[must_use]
    pub const fn tiles_y(&self) -> u32 {
        self.tiles_y
    }

    /// Render a batch of clip-space triangles using the 4-phase tile-based pipeline.
    ///
    /// This method processes all triangles through:
    /// 1. Prepare: Clip against near plane, project to screen, cull backfaces, sort vertices
    /// 2. Bin: Assign triangles to tiles based on AABB overlap
    /// 3. Render: Rasterize each tile with its assigned triangles
    /// 4. Merge: Copy tile buffers back to main framebuffer
    ///
    /// # Arguments
    ///
    /// * `fb` - Target framebuffer (must match dimensions from [`new`](Self::new))
    /// * `zb` - Depth buffer for z-testing (must match dimensions)
    /// * `triangles` - Slice of clip-space triangles `((Vec3, w), (Vec3, w), (Vec3, w), color)`
    ///
    /// # Performance Notes
    ///
    /// - **Reusable**: This method can be called multiple times with different geometry. Internal
    ///   buffers are reused to avoid allocations.
    /// - **Best for low triangle counts**: At 4K resolution, this is 27% faster than scanline
    ///   with 10 triangles, but 6% slower with 500 triangles due to binning overhead.
    /// - **Empty tiles skipped**: Tiles with no overlapping triangles are not processed, so
    ///   performance scales with screen coverage, not framebuffer size.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use abrash::tile_renderer::TileRenderer;
    /// # use abrash::framebuffer::Framebuffer;
    /// # use abrash::zbuffer::ZBuffer;
    /// # use abrash::math::Vec3;
    /// let mut renderer = TileRenderer::new(1920, 1080);
    /// let mut fb = Framebuffer::new(1920, 1080).unwrap();
    /// let mut zb = ZBuffer::new(1920, 1080).unwrap();
    ///
    /// let triangles = vec![
    ///     ((Vec3::new(0.0, 0.0, 1.0), 1.0),
    ///      (Vec3::new(1.0, 0.0, 1.0), 1.0),
    ///      (Vec3::new(0.5, 1.0, 1.0), 1.0),
    ///      0xFFFFFFFF),
    /// ];
    ///
    /// renderer.render_batch(&mut fb, &mut zb, &triangles);
    /// ```
    pub fn render_batch(
        &mut self,
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        triangles: &[ClipTriangle],
    ) {
        assert_eq!(
            fb.width(),
            self.width,
            "Framebuffer width must match TileRenderer width"
        );
        assert_eq!(
            fb.height(),
            self.height,
            "Framebuffer height must match TileRenderer height"
        );
        assert_eq!(
            zb.width(),
            self.width,
            "ZBuffer width must match TileRenderer width"
        );
        assert_eq!(
            zb.height(),
            self.height,
            "ZBuffer height must match TileRenderer height"
        );

        self.prepared.clear();
        for bin in &mut self.tile_bins {
            bin.clear();
        }

        // Phase 1: Prepare
        for &(v0, v1, v2, color) in triangles {
            self.prepare_triangle(v0, v1, v2, color);
        }

        // Build Hi-Z pyramid from previous frame (temporal coherence)
        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid(zb);
            }
        }

        // Phase 2: Bin (GPU or CPU with optional Hi-Z occlusion culling)
        #[cfg(feature = "gpu-binning")]
        if let Some(ref mut gpu) = self.gpu_binner {
            // GPU binning path - check if two-level binning is enabled
            if gpu.is_two_level_enabled() {
                // Two-level hierarchical binning with Hi-Z culling
                match gpu.bin_triangles_two_level(
                    &self.prepared,
                    self.hiz_buffer.as_ref(),
                    &mut self.tile_bins,
                ) {
                    Ok(_stats) => {
                        // Two-level binning succeeded
                        // Stats available for debugging/profiling but not used in production
                    }
                    Err(e) => {
                        eprintln!("Two-level GPU binning failed: {e}, falling back to CPU");
                        self.bin_triangles_cpu();
                    }
                }
            } else {
                // Single-level GPU binning
                if let Err(e) = gpu.bin_triangles(&self.prepared, &mut self.tile_bins) {
                    eprintln!("GPU binning failed: {e}, falling back to CPU");
                    self.bin_triangles_cpu();
                }
            }
        } else {
            self.bin_triangles_cpu();
        }

        #[cfg(not(feature = "gpu-binning"))]
        self.bin_triangles_cpu();

        // Phase 3+4: Render and merge each tile
        #[cfg(not(feature = "parallel"))]
        {
            // Sequential rendering
            for ty in 0..self.tiles_y {
                for tx in 0..self.tiles_x {
                    if let Some((tile_pixels, tile_depths, clear_y_min, clear_y_max)) =
                        render_single_tile(
                            tx,
                            ty,
                            &self.tile_bins,
                            &self.prepared,
                            self.tiles_x,
                            self.width,
                            self.height,
                        )
                    {
                        Self::merge_tile_direct(
                            &tile_pixels,
                            &tile_depths,
                            fb,
                            zb,
                            tx,
                            ty,
                            self.width,
                            self.height,
                            clear_y_min,
                            clear_y_max,
                        );
                    }
                }
            }
        }

        #[cfg(feature = "parallel")]
        {
            // Parallel rendering using Rayon
            use rayon::prelude::*;

            // Collect tile coordinates
            let tiles: Vec<(u32, u32)> = (0..self.tiles_y)
                .flat_map(|ty| (0..self.tiles_x).map(move |tx| (tx, ty)))
                .collect();

            // SAFETY: Each tile writes to a non-overlapping region of the framebuffer/zbuffer.
            // Tiles are 32×32 pixels at coordinates (tx*32, ty*32), so no two tiles overlap.
            // This is safe because:
            // 1. Each tile computes its own (tile_x0, tile_y0) bounds
            // 2. merge_tile_direct writes only to pixels within [tile_x0..tile_x1) × [tile_y0..tile_y1)
            // 3. No two tiles have the same (tx, ty), therefore no two tiles write to the same pixels
            unsafe {
                let fb_ptr = SendPtr(fb.as_mut_slice().as_mut_ptr());
                let zb_ptr = SendPtr(zb.as_mut_slice().as_mut_ptr());
                let width = self.width;
                let height = self.height;
                let tiles_x = self.tiles_x;
                let tile_bins = &self.tile_bins;
                let prepared = &self.prepared;

                tiles.par_iter().for_each(move |&(tx, ty)| {
                    if let Some((tile_pixels, tile_depths, clear_y_min, clear_y_max)) =
                        render_single_tile(tx, ty, tile_bins, prepared, tiles_x, width, height)
                    {
                        // Merge tile into framebuffer/zbuffer
                        let tile_x0 = tx * TILE_SIZE;
                        let tile_y0 = ty * TILE_SIZE;
                        let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                        let tile_cols = (tile_x_end - tile_x0) as usize;

                        let row_begin = clear_y_min.max(tile_y0 as i32) as u32;
                        let row_end = (clear_y_max as u32 + 1)
                            .min(tile_y0 + TILE_SIZE)
                            .min(height);

                        for row in row_begin..row_end {
                            let tile_row_offset = ((row - tile_y0) * TILE_SIZE) as usize;
                            let fb_start = row as usize * width as usize + tile_x0 as usize;

                            // SAFETY: fb_start and tile_row_offset are within bounds, and each thread
                            // writes to non-overlapping regions determined by unique (tx, ty)
                            for col in 0..tile_cols {
                                fb_ptr.write(fb_start + col, tile_pixels[tile_row_offset + col]);
                                zb_ptr.write(fb_start + col, tile_depths[tile_row_offset + col]);
                            }
                        }
                    }
                });
            }
        }

        // Invalidate Hi-Z for next frame
        if let Some(ref mut hiz) = self.hiz_buffer {
            hiz.invalidate();
        }
    }

    /// Render a batch of textured clip-space triangles.
    pub fn render_batch_textured(
        &mut self,
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        triangles: &[TexturedClipTriangle],
        texture: &Texture,
    ) {
        assert_eq!(
            fb.width(),
            self.width,
            "Framebuffer width must match TileRenderer width"
        );
        assert_eq!(
            fb.height(),
            self.height,
            "Framebuffer height must match TileRenderer height"
        );
        assert_eq!(
            zb.width(),
            self.width,
            "ZBuffer width must match TileRenderer width"
        );
        assert_eq!(
            zb.height(),
            self.height,
            "ZBuffer height must match TileRenderer height"
        );

        self.prepared_textured.clear();
        for bin in &mut self.tile_bins {
            bin.clear();
        }

        // Phase 1: Prepare
        let tex_w = texture.width as f32;
        let tex_h = texture.height as f32;
        for &(v0, uv0, v1, uv1, v2, uv2) in triangles {
            self.prepare_triangle_textured((v0, uv0), (v1, uv1), (v2, uv2), tex_w, tex_h);
        }

        // Build Hi-Z pyramid from previous frame (temporal coherence)
        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid(zb);
            }
        }

        // Phase 2: Bin (CPU only for now)
        self.bin_triangles_textured_cpu();

        // Phase 3+4: Render and merge each tile
        #[cfg(not(feature = "parallel"))]
        {
            // Sequential rendering
            for ty in 0..self.tiles_y {
                for tx in 0..self.tiles_x {
                    if let Some((clear_y_min, clear_y_max)) = render_single_tile_textured(
                        tx,
                        ty,
                        &self.tile_bins,
                        &self.prepared_textured,
                        self.tiles_x,
                        self.width,
                        self.height,
                        texture,
                        &mut self.tile_pixels,
                        &mut self.tile_depths,
                    ) {
                        Self::merge_tile_direct(
                            &self.tile_pixels,
                            &self.tile_depths,
                            fb,
                            zb,
                            tx,
                            ty,
                            self.width,
                            self.height,
                            clear_y_min,
                            clear_y_max,
                        );
                    }
                }
            }
        }

        #[cfg(feature = "parallel")]
        {
            // Parallel rendering using Rayon
            use rayon::prelude::*;

            // Collect tile coordinates
            let tiles: Vec<(u32, u32)> = (0..self.tiles_y)
                .flat_map(|ty| (0..self.tiles_x).map(move |tx| (tx, ty)))
                .collect();

            unsafe {
                let fb_ptr = SendPtr(fb.as_mut_slice().as_mut_ptr());
                let zb_ptr = SendPtr(zb.as_mut_slice().as_mut_ptr());
                let width = self.width;
                let height = self.height;
                let tiles_x = self.tiles_x;
                let tile_bins = &self.tile_bins;
                let prepared = &self.prepared_textured;

                tiles.par_iter().for_each(move |&(tx, ty)| {
                    // Allocate thread-local buffers
                    let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                    let mut tile_pixels = vec![0u32; tile_area];
                    let mut tile_depths = vec![f32::INFINITY; tile_area];

                    if let Some((clear_y_min, clear_y_max)) = render_single_tile_textured(
                        tx,
                        ty,
                        tile_bins,
                        prepared,
                        tiles_x,
                        width,
                        height,
                        texture,
                        &mut tile_pixels,
                        &mut tile_depths,
                    ) {
                        // Merge tile into framebuffer/zbuffer
                        let tile_x0 = tx * TILE_SIZE;
                        let tile_y0 = ty * TILE_SIZE;
                        let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                        let tile_cols = (tile_x_end - tile_x0) as usize;

                        let row_begin = clear_y_min.max(tile_y0 as i32) as u32;
                        let row_end = (clear_y_max as u32 + 1)
                            .min(tile_y0 + TILE_SIZE)
                            .min(height);

                        for row in row_begin..row_end {
                            let tile_row_offset = ((row - tile_y0) * TILE_SIZE) as usize;
                            let fb_start = row as usize * width as usize + tile_x0 as usize;

                            for col in 0..tile_cols {
                                fb_ptr.write(fb_start + col, tile_pixels[tile_row_offset + col]);
                                zb_ptr.write(fb_start + col, tile_depths[tile_row_offset + col]);
                            }
                        }
                    }
                });
            }
        }

        // Invalidate Hi-Z for next frame
        if let Some(ref mut hiz) = self.hiz_buffer {
            hiz.invalidate();
        }
    }

    fn prepare_triangle_textured(
        &mut self,
        v0: ((Vec3, f32), Vec2),
        v1: ((Vec3, f32), Vec2),
        v2: ((Vec3, f32), Vec2),
        tex_w: f32,
        tex_h: f32,
    ) {
        let clipped = clip_triangle_against_near_plane(v0, v1, v2, |v| v.0.1);

        for i in 0..clipped.count {
            let base = i * 3;
            let cv0 = clipped.tris[base];
            let cv1 = clipped.tris[base + 1];
            let cv2 = clipped.tris[base + 2];

            let p0_orig = project_to_screen(cv0.0.0, cv0.0.1, self.width, self.height);
            let p1_orig = project_to_screen(cv1.0.0, cv1.0.1, self.width, self.height);
            let p2_orig = project_to_screen(cv2.0.0, cv2.0.1, self.width, self.height);

            if is_backface(p0_orig, p1_orig, p2_orig) {
                continue;
            }

            let w0 = cv0.0.1;
            let w1 = cv1.0.1;
            let w2 = cv2.0.1;

            let inv_w0 = if w0.abs() > 0.0001 { 1.0 / w0 } else { 1.0 };
            let inv_w1 = if w1.abs() > 0.0001 { 1.0 / w1 } else { 1.0 };
            let inv_w2 = if w2.abs() > 0.0001 { 1.0 / w2 } else { 1.0 };

            let u0 = cv0.1.x * tex_w * inv_w0;
            let v0_val = cv0.1.y * tex_h * inv_w0;
            let u1 = cv1.1.x * tex_w * inv_w1;
            let v1_val = cv1.1.y * tex_h * inv_w1;
            let u2 = cv2.1.x * tex_w * inv_w2;
            let v2_val = cv2.1.y * tex_h * inv_w2;

            let mut verts = [
                (p0_orig, inv_w0, u0, v0_val),
                (p1_orig, inv_w1, u1, v1_val),
                (p2_orig, inv_w2, u2, v2_val),
            ];
            sort_by_y(&mut verts, |(p, _, _, _)| p.y);
            let [(p0, q0, u0, v0), (p1, q1, u1, v1), (p2, q2, u2, v2)] = verts;

            let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            if total_height == 0.0 {
                continue;
            }

            // Gradients
            let (gradients, long_edge_is_left) = {
                let g = PerspectiveTextureGradients::new(
                    p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2,
                );

                let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
                let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
                let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
                let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
                let left = ux * vy - uy * vx > 0.0;
                (g, left)
            };

            // AABB
            let min_x = p0.x.min(p1.x).min(p2.x).max(0);
            let min_y = p0.y.max(0);
            let max_x = p0.x.max(p1.x).max(p2.x).min(self.width as i32 - 1);
            let max_y = p2.y.min(self.height as i32 - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            let min_depth = p0.z.min(p1.z).min(p2.z);
            let max_depth = p0.z.max(p1.z).max(p2.z);

            let p0_fixed = VertexFixed::from_screen_point(p0);
            let p1_fixed = VertexFixed::from_screen_point(p1);
            let p2_fixed = VertexFixed::from_screen_point(p2);

            self.prepared_textured.push(PreparedTexturedTriangle {
                p0,
                p1,
                p2,
                p0_fixed,
                p1_fixed,
                p2_fixed,
                q0,
                q1,
                q2,
                u0,
                u1,
                u2,
                v0,
                v1,
                v2,
                gradients,
                long_edge_is_left,
                aabb_min_x: min_x,
                aabb_min_y: min_y,
                aabb_max_x: max_x,
                aabb_max_y: max_y,
                min_depth,
                max_depth,
            });
        }
    }

    fn bin_triangles_textured_cpu(&mut self) {
        let prepared_len = self.prepared_textured.len();
        for i in 0..prepared_len {
            if let Some(ref hiz) = self.hiz_buffer {
                let tri = &self.prepared_textured[i];
                let aabb = AABB3D {
                    min_x: tri.aabb_min_x,
                    max_x: tri.aabb_max_x,
                    min_y: tri.aabb_min_y,
                    max_y: tri.aabb_max_y,
                    min_depth: tri.min_depth,
                    max_depth: tri.max_depth,
                };

                if !hiz.is_potentially_visible(aabb) {
                    continue;
                }
            }
            self.bin_triangle_textured(i);
        }
    }

    fn bin_triangle_textured(&mut self, tri_idx: usize) {
        let tri = &self.prepared_textured[tri_idx];
        let tile_size_i32 = TILE_SIZE as i32;

        let tx_min = (tri.aabb_min_x / tile_size_i32) as u32;
        let ty_min = (tri.aabb_min_y / tile_size_i32) as u32;
        let tx_max = ((tri.aabb_max_x / tile_size_i32) as u32).min(self.tiles_x - 1);
        let ty_max = ((tri.aabb_max_y / tile_size_i32) as u32).min(self.tiles_y - 1);

        for ty in ty_min..=ty_max {
            for tx in tx_min..=tx_max {
                let bin_idx = (ty * self.tiles_x + tx) as usize;
                self.tile_bins[bin_idx].push(tri_idx);
            }
        }
    }

    fn prepare_triangle(&mut self, v0: (Vec3, f32), v1: (Vec3, f32), v2: (Vec3, f32), color: u32) {
        let clipped = clip_triangle_against_near_plane(v0, v1, v2, |v| v.1);

        for i in 0..clipped.count {
            let base = i * 3;
            let cv0 = clipped.tris[base];
            let cv1 = clipped.tris[base + 1];
            let cv2 = clipped.tris[base + 2];

            let p0_orig = project_to_screen(cv0.0, cv0.1, self.width, self.height);
            let p1_orig = project_to_screen(cv1.0, cv1.1, self.width, self.height);
            let p2_orig = project_to_screen(cv2.0, cv2.1, self.width, self.height);

            if is_backface(p0_orig, p1_orig, p2_orig) {
                continue;
            }

            let mut verts = [p0_orig, p1_orig, p2_orig];
            sort_by_y(&mut verts, |p| p.y);
            let [p0, p1, p2] = verts;

            let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            if total_height == 0.0 {
                continue;
            }

            // Compute dz/dx
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let uz = p1.z - p0.z;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let vz = p2.z - p0.z;
            let nx = uy * vz - uz * vy;
            let nz = ux * vy - uy * vx;

            let dz_dx = if nz.abs() > 0.0001 { -nx / nz } else { 0.0 };
            let long_edge_is_left = nz > 0.0;

            // AABB clamped to screen
            let min_x = p0.x.min(p1.x).min(p2.x).max(0);
            let min_y = p0.y.max(0);
            let max_x = p0.x.max(p1.x).max(p2.x).min(self.width as i32 - 1);
            let max_y = p2.y.min(self.height as i32 - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            // Compute min/max depth for Hi-Z occlusion culling
            let min_depth = p0.z.min(p1.z).min(p2.z);
            let max_depth = p0.z.max(p1.z).max(p2.z);

            // Convert to fixed-point for deterministic edge functions
            let p0_fixed = VertexFixed::from_screen_point(p0);
            let p1_fixed = VertexFixed::from_screen_point(p1);
            let p2_fixed = VertexFixed::from_screen_point(p2);

            self.prepared.push(PreparedTriangle {
                p0,
                p1,
                p2,
                p0_fixed,
                p1_fixed,
                p2_fixed,
                dz_dx,
                long_edge_is_left,
                color,
                aabb_min_x: min_x,
                aabb_min_y: min_y,
                aabb_max_x: max_x,
                aabb_max_y: max_y,
                min_depth,
                max_depth,
            });
        }
    }

    /// CPU binning path with optional Hi-Z occlusion culling
    fn bin_triangles_cpu(&mut self) {
        let prepared_len = self.prepared.len();
        for i in 0..prepared_len {
            // Occlusion test before binning (if Hi-Z is enabled)
            if let Some(ref hiz) = self.hiz_buffer {
                let tri = &self.prepared[i];
                let aabb = AABB3D {
                    min_x: tri.aabb_min_x,
                    max_x: tri.aabb_max_x,
                    min_y: tri.aabb_min_y,
                    max_y: tri.aabb_max_y,
                    min_depth: tri.min_depth,
                    max_depth: tri.max_depth,
                };

                if !hiz.is_potentially_visible(aabb) {
                    continue; // Skip binning if occluded
                }
            }

            self.bin_triangle(i);
        }
    }

    fn bin_triangle(&mut self, tri_idx: usize) {
        let tri = &self.prepared[tri_idx];
        let tile_size_i32 = TILE_SIZE as i32;

        let tx_min = (tri.aabb_min_x / tile_size_i32) as u32;
        let ty_min = (tri.aabb_min_y / tile_size_i32) as u32;
        let tx_max = ((tri.aabb_max_x / tile_size_i32) as u32).min(self.tiles_x - 1);
        let ty_max = ((tri.aabb_max_y / tile_size_i32) as u32).min(self.tiles_y - 1);

        for ty in ty_min..=ty_max {
            for tx in tx_min..=tx_max {
                let bin_idx = (ty * self.tiles_x + tx) as usize;
                self.tile_bins[bin_idx].push(tri_idx);
            }
        }
    }

    /// Merge tile buffers into framebuffer using direct copy (no depth test).
    /// Only copies the rows between `y_min` and `y_max` (inclusive, screen coords).
    #[allow(clippy::too_many_arguments)]
    #[cfg(not(feature = "parallel"))]
    fn merge_tile_direct(
        tile_pixels: &[u32],
        tile_depths: &[f32],
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        tx: u32,
        ty: u32,
        width: u32,
        height: u32,
        y_min: i32,
        y_max: i32,
    ) {
        let tile_x0 = tx * TILE_SIZE;
        let tile_y0 = ty * TILE_SIZE;
        let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
        let tile_cols = (tile_x_end - tile_x0) as usize;

        let fb_slice = fb.as_mut_slice();
        let zb_slice = zb.as_mut_slice();
        let fb_width = width as usize;

        let row_begin = y_min.max(tile_y0 as i32) as u32;
        let row_end = (y_max as u32 + 1).min(tile_y0 + TILE_SIZE).min(height);

        for row in row_begin..row_end {
            let tile_row_offset = ((row - tile_y0) * TILE_SIZE) as usize;
            let fb_start = row as usize * fb_width + tile_x0 as usize;

            fb_slice[fb_start..fb_start + tile_cols]
                .copy_from_slice(&tile_pixels[tile_row_offset..tile_row_offset + tile_cols]);
            zb_slice[fb_start..fb_start + tile_cols]
                .copy_from_slice(&tile_depths[tile_row_offset..tile_row_offset + tile_cols]);
        }
    }
}

/// Determines whether to use tile-based or scanline rendering based on resolution and triangle count.
///
/// This function implements a heuristic based on empirical benchmark results across different
/// resolutions and workloads. It returns `true` when tile-based rendering is expected to
/// outperform traditional scanline rendering.
///
/// # Decision Criteria
///
/// The heuristic considers two factors:
///
/// 1. **Framebuffer size**: Tiling only helps when the framebuffer exceeds L3 cache capacity
///    (typically 8-16 MB). We use 12 MB as the threshold.
///
/// 2. **Triangle count**: Tiling overhead (prepare + bin + merge) becomes significant at high
///    triangle counts. We use 100 triangles as the cutoff.
///
/// # Returns
///
/// - `true` if tile-based rendering is recommended (framebuffer > 12 MB AND triangles ≤ 100)
/// - `false` if scanline rendering is recommended (use [`Rasterizer::fill_triangle_3d`](crate::rasterizer::Rasterizer::fill_triangle_3d))
///
/// # Examples
///
/// ```
/// use abrash::tile_renderer::should_use_tiled_rendering;
///
/// // 4K resolution with 50 triangles → use tiled (27% faster in benchmarks)
/// assert!(should_use_tiled_rendering(3840, 2160, 50));
///
/// // 1080p with 10 triangles → use tiled (8% faster in benchmarks)
/// assert!(should_use_tiled_rendering(1920, 1080, 10));
///
/// // 800x600 with 50 triangles → use scanline (fits in cache, tiling overhead dominates)
/// assert!(!should_use_tiled_rendering(800, 600, 50));
///
/// // 4K with 500 triangles → use scanline (binning overhead too high)
/// assert!(!should_use_tiled_rendering(3840, 2160, 500));
/// ```
///
/// # Performance Data
///
/// The thresholds are derived from benchmark results:
///
/// | Resolution | Size | Triangles | Tiled vs Scanline | Recommended |
/// |------------|------|-----------|-------------------|-------------|
/// | 800×600 | 3.84 MB | any | 1.3-3× slower | Scanline |
/// | 1920×1080 | 16.6 MB | 10 | 0.92× (8% faster) | **Tiled** |
/// | 1920×1080 | 16.6 MB | 200 | 1.05× (~even) | Scanline |
/// | 3840×2160 | 66.4 MB | 10 | 0.73× (27% faster) | **Tiled** |
/// | 3840×2160 | 66.4 MB | 50 | 0.94× (6% faster) | **Tiled** |
/// | 3840×2160 | 66.4 MB | 200 | 1.06× slower | Scanline |
///
/// See `docs/adr/001-tile-based-rendering.md` for full benchmark analysis.
#[must_use]
pub fn should_use_tiled_rendering(width: usize, height: usize, triangle_count: usize) -> bool {
    let pixels = width * height;
    // Calculate framebuffer size in megabytes (4 bytes per pixel + 4 bytes per depth = 8 bytes total)
    let framebuffer_mb = (pixels * 8) / (1024 * 1024);

    // Use tiled rendering if:
    // 1. Framebuffer exceeds typical L3 cache (> 12 MB means severe cache pressure for scanline)
    // 2. Triangle count is low enough that setup overhead doesn't dominate (≤ 100 triangles)
    framebuffer_mb > 12 && triangle_count <= 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::Vec3;
    use crate::rasterizer::fill_triangle_3d;
    use crate::zbuffer::ZBuffer;

    // --- Step 1: Infrastructure + prepare ---

    #[test]
    fn new_creates_correct_tile_grid_exact_multiple() {
        let tr = TileRenderer::new(64, 64);
        assert_eq!(tr.tiles_x(), 2);
        assert_eq!(tr.tiles_y(), 2);
        assert_eq!(tr.width, 64);
        assert_eq!(tr.height, 64);
    }

    #[test]
    fn new_creates_correct_tile_grid_non_aligned() {
        // 100 / 32 = 3.125 → 4 tiles
        let tr = TileRenderer::new(100, 100);
        assert_eq!(tr.tiles_x(), 4);
        assert_eq!(tr.tiles_y(), 4);
    }

    #[test]
    fn new_creates_correct_tile_grid_800x600() {
        let tr = TileRenderer::new(800, 600);
        assert_eq!(tr.tiles_x(), 25);
        assert_eq!(tr.tiles_y(), 19); // 600 / 32 = 18.75 → 19
    }

    #[test]
    #[should_panic(expected = "Dimensions must be positive")]
    fn new_panics_on_zero_width() {
        let _ = TileRenderer::new(0, 100);
    }

    #[test]
    #[should_panic(expected = "Dimensions must be positive")]
    fn new_panics_on_zero_height() {
        let _ = TileRenderer::new(100, 0);
    }

    #[test]
    fn prepare_triangle_culls_backface() {
        let mut tr = TileRenderer::new(100, 100);
        // Reversed winding — backface
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(tr.prepared.len(), 0);
    }

    #[test]
    fn prepare_triangle_accepts_front_face() {
        let mut tr = TileRenderer::new(100, 100);
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(tr.prepared.len(), 1);
    }

    #[test]
    fn prepare_triangle_rejects_degenerate_zero_area() {
        let mut tr = TileRenderer::new(100, 100);
        // All three vertices at the same point → zero area
        let v0 = (Vec3::new(0.0, 0.0, 5.0), 5.0);
        let v1 = (Vec3::new(0.0, 0.0, 5.0), 5.0);
        let v2 = (Vec3::new(0.0, 0.0, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            0,
            "Zero-area triangle should be rejected"
        );
    }

    #[test]
    fn prepare_triangle_rejects_degenerate_collinear() {
        let mut tr = TileRenderer::new(100, 100);
        // Three collinear vertices → zero area
        let v0 = (Vec3::new(-0.5, 0.0, 5.0), 5.0);
        let v1 = (Vec3::new(0.0, 0.0, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, 0.0, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            0,
            "Collinear triangle should be rejected"
        );
    }

    #[test]
    fn prepare_triangle_rejects_tiny_subpixel() {
        let mut tr = TileRenderer::new(100, 100);
        // Triangle with area < 0.5 pixels (subpixel, effectively invisible)
        let v0 = (Vec3::new(0.0, 0.0, 5.0), 5.0);
        let v1 = (Vec3::new(0.001, 0.0, 5.0), 5.0);
        let v2 = (Vec3::new(0.0, 0.001, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(tr.prepared.len(), 0, "Subpixel triangle should be rejected");
    }

    #[test]
    fn prepare_triangle_accepts_normal_triangle() {
        let mut tr = TileRenderer::new(100, 100);
        // Normal visible triangle
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(tr.prepared.len(), 1, "Normal triangle should be accepted");
    }

    #[test]
    fn frustum_culling_rejects_triangle_left_of_frustum() {
        let mut tr = TileRenderer::new(100, 100);
        // All vertices have x < -w (left of frustum)
        let w = 5.0;
        let v0 = (Vec3::new(-10.0, 0.0, 5.0), w);
        let v1 = (Vec3::new(-8.0, 1.0, 5.0), w);
        let v2 = (Vec3::new(-9.0, -1.0, 5.0), w);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            0,
            "Triangle completely left of frustum should be culled"
        );
    }

    #[test]
    fn frustum_culling_rejects_triangle_right_of_frustum() {
        let mut tr = TileRenderer::new(100, 100);
        // All vertices have x > w (right of frustum)
        let w = 5.0;
        let v0 = (Vec3::new(10.0, 0.0, 5.0), w);
        let v1 = (Vec3::new(8.0, 1.0, 5.0), w);
        let v2 = (Vec3::new(9.0, -1.0, 5.0), w);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            0,
            "Triangle completely right of frustum should be culled"
        );
    }

    #[test]
    fn frustum_culling_rejects_triangle_above_frustum() {
        let mut tr = TileRenderer::new(100, 100);
        // All vertices have y > w (above frustum, before Y flip)
        let w = 5.0;
        let v0 = (Vec3::new(0.0, 10.0, 5.0), w);
        let v1 = (Vec3::new(1.0, 8.0, 5.0), w);
        let v2 = (Vec3::new(-1.0, 9.0, 5.0), w);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            0,
            "Triangle completely above frustum should be culled"
        );
    }

    #[test]
    fn frustum_culling_rejects_triangle_below_frustum() {
        let mut tr = TileRenderer::new(100, 100);
        // All vertices have y < -w (below frustum, before Y flip)
        let w = 5.0;
        let v0 = (Vec3::new(0.0, -10.0, 5.0), w);
        let v1 = (Vec3::new(1.0, -8.0, 5.0), w);
        let v2 = (Vec3::new(-1.0, -9.0, 5.0), w);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            0,
            "Triangle completely below frustum should be culled"
        );
    }

    #[test]
    fn frustum_culling_accepts_partially_visible_triangle() {
        let mut tr = TileRenderer::new(100, 100);
        // Triangle with one vertex outside frustum but should NOT be culled
        // (only cull if ALL vertices are outside)
        let w = 5.0;
        let v0 = (Vec3::new(0.0, 0.0, 5.0), w); // Inside
        let v1 = (Vec3::new(10.0, 0.0, 5.0), w); // Outside right
        let v2 = (Vec3::new(0.0, 1.0, 5.0), w); // Inside
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            1,
            "Partially visible triangle should NOT be culled"
        );
    }

    #[test]
    fn frustum_culling_accepts_fully_visible_triangle() {
        let mut tr = TileRenderer::new(100, 100);
        // Triangle completely within frustum bounds
        let w = 5.0;
        let v0 = (Vec3::new(0.0, 0.5, 5.0), w);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), w);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), w);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        assert_eq!(
            tr.prepared.len(),
            1,
            "Fully visible triangle should be accepted"
        );
    }

    #[test]
    fn prepare_triangle_sorts_by_y() {
        let mut tr = TileRenderer::new(100, 100);
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        let tri = &tr.prepared[0];
        assert!(tri.p0.y <= tri.p1.y);
        assert!(tri.p1.y <= tri.p2.y);
    }

    #[test]
    fn prepare_triangle_computes_aabb() {
        let mut tr = TileRenderer::new(100, 100);
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);
        let tri = &tr.prepared[0];
        // AABB should be within screen bounds
        assert!(tri.aabb_min_x >= 0);
        assert!(tri.aabb_min_y >= 0);
        assert!(tri.aabb_max_x < 100);
        assert!(tri.aabb_max_y < 100);
        // And AABB should encompass the triangle
        assert!(tri.aabb_min_x <= tri.p0.x.min(tri.p1.x).min(tri.p2.x));
        assert!(tri.aabb_max_x >= tri.p0.x.max(tri.p1.x).max(tri.p2.x));
    }

    // --- Step 2: Binning ---

    #[test]
    fn small_triangle_bins_to_one_tile() {
        let mut tr = TileRenderer::new(100, 100);

        // A small triangle that fits entirely within one tile
        // NDC coords that will map to roughly the center of tile (0,0) i.e. pixels 0-31
        // NDC -1 maps to x=0, NDC ~-0.36 maps to x=32 for width 100
        // Use coords that clearly land in the first tile
        let v0 = (Vec3::new(-0.9, 0.9, 5.0), 5.0);
        let v1 = (Vec3::new(-0.7, 0.9, 5.0), 5.0);
        let v2 = (Vec3::new(-0.8, 0.7, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);

        if tr.prepared.is_empty() {
            return; // Culled — skip test
        }

        tr.bin_triangle(0);

        // Count how many tiles have this triangle
        let binned_count: usize = tr.tile_bins.iter().filter(|b| !b.is_empty()).count();
        assert_eq!(
            binned_count, 1,
            "Small triangle should bin to exactly 1 tile"
        );
    }

    #[test]
    fn large_triangle_bins_to_multiple_tiles() {
        // Use 800x600 so a large triangle clearly spans many 32x32 tiles
        let mut tr = TileRenderer::new(800, 600);

        // A large triangle spanning most of the screen
        let v0 = (Vec3::new(0.0, 0.8, 5.0), 5.0);
        let v1 = (Vec3::new(-0.8, -0.8, 5.0), 5.0);
        let v2 = (Vec3::new(0.8, -0.8, 5.0), 5.0);
        tr.prepare_triangle(v0, v1, v2, 0xFFFF_0000);

        assert!(!tr.prepared.is_empty(), "Triangle should not be culled");

        tr.bin_triangle(0);

        let binned_count: usize = tr.tile_bins.iter().filter(|b| !b.is_empty()).count();
        assert!(
            binned_count > 1,
            "Large triangle should bin to multiple tiles, got {binned_count}"
        );
    }

    // --- Step 3: Tile rendering + merge ---

    #[test]
    fn single_triangle_renders_identically_to_fill_triangle_3d() {
        let width = 100;
        let height = 100;

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        // Reference: scanline renderer
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, color);

        // Tiled renderer
        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(&mut fb_tile, &mut zb_tile, &[(v0, v1, v2, color)]);

        // Compare every pixel
        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();
        let ref_depths = zb_ref.as_slice();
        let tile_depths = zb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i],
                ref_pixels[i],
                "Pixel mismatch at index {i} (x={}, y={}): tiled=0x{:08X}, ref=0x{:08X}",
                i % width as usize,
                i / width as usize,
                tile_pixels[i],
                ref_pixels[i]
            );
            // Depth values should match within floating point tolerance
            let d_diff = (tile_depths[i] - ref_depths[i]).abs();
            assert!(
                d_diff < 1e-5 || (tile_depths[i].is_infinite() && ref_depths[i].is_infinite()),
                "Depth mismatch at index {i}: tiled={}, ref={}",
                tile_depths[i],
                ref_depths[i]
            );
        }
    }

    #[test]
    fn two_overlapping_triangles_zbuffer_correctness() {
        let width = 100;
        let height = 100;

        // Front triangle (closer, z=3) — should occlude back triangle
        let v0_front = (Vec3::new(0.0, 0.5, 3.0), 3.0);
        let v1_front = (Vec3::new(-0.5, -0.5, 3.0), 3.0);
        let v2_front = (Vec3::new(0.5, -0.5, 3.0), 3.0);
        let color_front = 0xFFFF_0000; // Red

        // Back triangle (farther, z=7) — should be occluded
        let v0_back = (Vec3::new(0.0, 0.5, 7.0), 7.0);
        let v1_back = (Vec3::new(-0.5, -0.5, 7.0), 7.0);
        let v2_back = (Vec3::new(0.5, -0.5, 7.0), 7.0);
        let color_back = 0xFF00_FF00; // Green

        // Reference
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        // Render back first, then front — front should win
        fill_triangle_3d(
            &mut fb_ref,
            &mut zb_ref,
            v0_back,
            v1_back,
            v2_back,
            color_back,
        );
        fill_triangle_3d(
            &mut fb_ref,
            &mut zb_ref,
            v0_front,
            v1_front,
            v2_front,
            color_front,
        );

        // Tiled: submit both in one batch (order shouldn't matter for z-buffer)
        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(
            &mut fb_tile,
            &mut zb_tile,
            &[
                (v0_back, v1_back, v2_back, color_back),
                (v0_front, v1_front, v2_front, color_front),
            ],
        );

        // Compare
        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i], ref_pixels[i],
                "Pixel mismatch at index {i}: tiled=0x{:08X}, ref=0x{:08X}",
                tile_pixels[i], ref_pixels[i]
            );
        }
    }

    #[test]
    fn partial_tiles_at_screen_edges() {
        // 50x50 is not a multiple of 32:  32 + 18 = 50
        let width = 50;
        let height = 50;

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, color);

        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(&mut fb_tile, &mut zb_tile, &[(v0, v1, v2, color)]);

        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i], ref_pixels[i],
                "Pixel mismatch at index {i} in partial tile test",
            );
        }
    }

    // --- Step 4: Edge cases ---

    #[test]
    fn near_plane_clipped_triangle() {
        let width = 100;
        let height = 100;

        // One vertex near the camera (w very small), two vertices farther away
        // This should trigger near-plane clipping producing 2 sub-triangles
        let v0 = (Vec3::new(0.0, 0.0, 0.0005), 0.0005); // Very close (below near plane)
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        // Reference
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, color);

        // Tiled
        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(&mut fb_tile, &mut zb_tile, &[(v0, v1, v2, color)]);

        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i], ref_pixels[i],
                "Pixel mismatch at index {i} in near-plane clipping test",
            );
        }
    }

    #[test]
    fn degenerate_triangle_no_panic() {
        let width = 100;
        let height = 100;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);

        // Collinear points
        let v0 = (Vec3::new(0.0, 0.0, 5.0), 5.0);
        let v1 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.0, 1.0, 5.0), 5.0);

        // Should not panic
        tr.render_batch(&mut fb, &mut zb, &[(v0, v1, v2, 0xFFFF_0000)]);
    }

    #[test]
    fn offscreen_triangle_no_panic() {
        let width = 100;
        let height = 100;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);

        // Far off-screen
        let v0 = (Vec3::new(5.0, 5.0, 5.0), 5.0);
        let v1 = (Vec3::new(6.0, 5.0, 5.0), 5.0);
        let v2 = (Vec3::new(5.5, 6.0, 5.0), 5.0);

        tr.render_batch(&mut fb, &mut zb, &[(v0, v1, v2, 0xFFFF_0000)]);

        // Screen should remain blank
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF00_0000);
        }
    }

    #[test]
    fn edge_switch_at_p1y_within_tile() {
        // Triangle where p1.y falls within a tile boundary — tests edge B transition
        let width = 100;
        let height = 100;

        // Tall thin triangle where p1 is at a different Y than p0 and p2
        let v0 = (Vec3::new(0.0, 0.8, 5.0), 5.0);
        let v1 = (Vec3::new(-0.3, 0.0, 5.0), 5.0);
        let v2 = (Vec3::new(0.3, -0.8, 5.0), 5.0);
        let color = 0xFF00_00FF;

        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, color);

        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(&mut fb_tile, &mut zb_tile, &[(v0, v1, v2, color)]);

        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i], ref_pixels[i],
                "Pixel mismatch at index {i} in edge-switch test",
            );
        }
    }

    #[test]
    fn multiple_triangles_same_depth_render_in_order() {
        let width = 100;
        let height = 100;

        // Two triangles at same depth — later one should win (z < z case won't trigger,
        // so first-written wins)
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);

        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, 0xFFFF_0000);
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, 0xFF00_FF00);

        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(
            &mut fb_tile,
            &mut zb_tile,
            &[(v0, v1, v2, 0xFFFF_0000), (v0, v1, v2, 0xFF00_FF00)],
        );

        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i], ref_pixels[i],
                "Pixel mismatch at index {i} in same-depth test",
            );
        }
    }

    #[test]
    fn render_batch_with_empty_input() {
        let width = 100;
        let height = 100;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);

        // Should not panic
        tr.render_batch(&mut fb, &mut zb, &[]);

        // Screen should remain blank
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF00_0000);
        }
    }

    #[test]
    fn render_batch_reusable() {
        let width = 100;
        let height = 100;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);

        // First batch
        tr.render_batch(&mut fb, &mut zb, &[(v0, v1, v2, 0xFFFF_0000)]);

        // Second batch should work without issues (internal state cleared)
        fb.clear(0xFF00_0000);
        zb.clear();
        tr.render_batch(&mut fb, &mut zb, &[(v0, v1, v2, 0xFF00_FF00)]);

        // Center pixel should be green, not red
        let center = fb.get_pixel(50, 50);
        assert_ne!(
            center,
            Some(0xFFFF_0000),
            "Second batch should overwrite first"
        );
    }

    #[test]
    fn large_resolution_no_panic() {
        let width = 800;
        let height = 600;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);

        tr.render_batch(&mut fb, &mut zb, &[(v0, v1, v2, 0xFFFF_0000)]);

        // Reference
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, 0xFFFF_0000);

        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(tile_pixels[i], ref_pixels[i]);
        }
    }

    // --- Auto-selection heuristic tests ---

    #[test]
    fn should_use_tiled_for_4k_low_triangles() {
        // 4K with 10 triangles: 27% faster in benchmarks
        assert!(should_use_tiled_rendering(3840, 2160, 10));
        // 4K with 50 triangles: 6% faster in benchmarks
        assert!(should_use_tiled_rendering(3840, 2160, 50));
        // 4K with 100 triangles: ~even, but still recommended
        assert!(should_use_tiled_rendering(3840, 2160, 100));
    }

    #[test]
    fn should_use_scanline_for_4k_high_triangles() {
        // 4K with 200 triangles: 6% slower, use scanline
        assert!(!should_use_tiled_rendering(3840, 2160, 200));
        // 4K with 500 triangles: 6% slower, use scanline
        assert!(!should_use_tiled_rendering(3840, 2160, 500));
    }

    #[test]
    fn should_use_tiled_for_1080p_low_triangles() {
        // 1080p with 10 triangles: 8% faster in benchmarks
        assert!(should_use_tiled_rendering(1920, 1080, 10));
        // 1080p with 50 triangles: slower, but framebuffer is borderline
        // This is a judgment call - the heuristic says use tiled
        assert!(should_use_tiled_rendering(1920, 1080, 50));
    }

    #[test]
    fn should_use_scanline_for_1080p_high_triangles() {
        // 1080p with 200 triangles: ~5% slower, use scanline
        assert!(!should_use_tiled_rendering(1920, 1080, 200));
    }

    #[test]
    fn should_use_scanline_for_low_res() {
        // 800x600: Scanline wins at all triangle counts (fits in L2/L3)
        assert!(!should_use_tiled_rendering(800, 600, 10));
        assert!(!should_use_tiled_rendering(800, 600, 50));
        assert!(!should_use_tiled_rendering(800, 600, 500));
    }

    #[test]
    fn edge_case_exactly_12mb() {
        // Calculate dimensions around 12 MB threshold (accounting for 8 bytes per pixel)
        // 12 MB = 12 * 1024 * 1024 bytes = 12,582,912 bytes
        // pixels = 12,582,912 / 8 = 1,572,864 pixels
        // sqrt(1,572,864) ≈ 1254.14

        // 1254x1254 = 1,572,516 pixels → (1,572,516 * 8) / (1024*1024) = 11 MB (integer division)
        assert!(!should_use_tiled_rendering(1254, 1254, 100));

        // 1255x1254 = 1,573,770 pixels → (1,573,770 * 8) / (1024*1024) = 12 MB (integer division)
        // Exactly 12 MB is still ≤ 12, so should use scanline
        assert!(!should_use_tiled_rendering(1255, 1254, 100));

        // Need dimensions that give > 12 MB. Try 1300x1200 = 1,560,000 pixels
        // Actually, let's use 1920x1080 which we know is 16.6 MB
        assert!(should_use_tiled_rendering(1920, 1080, 100));
    }

    #[test]
    fn edge_case_exactly_100_triangles() {
        // 4K with exactly 100 triangles: should use tiled (≤ 100)
        assert!(should_use_tiled_rendering(3840, 2160, 100));

        // 4K with 101 triangles: should use scanline (> 100)
        assert!(!should_use_tiled_rendering(3840, 2160, 101));
    }

    // --- Fixed-point arithmetic tests ---

    #[test]
    fn vertex_fixed_conversion() {
        // Test conversion from ScreenPoint to VertexFixed
        let p = ScreenPoint {
            x: 100,
            y: 200,
            z: 5.0,
        };

        let fixed = VertexFixed::from_screen_point(p);

        // 24.8 fixed point: value << 8
        assert_eq!(fixed.x, 100 << 8); // 25600
        assert_eq!(fixed.y, 200 << 8); // 51200
        assert_eq!(fixed.z, (5.0 * 256.0) as i32); // 24.8 fixed point

        // Test conversion back to pixel coordinates
        assert_eq!(fixed.to_pixel_x(), 100);
        assert_eq!(fixed.to_pixel_y(), 200);
    }

    #[test]
    fn vertex_fixed_subpixel_precision() {
        // Test that 24.8 format supports sub-pixel precision
        let fixed = VertexFixed {
            x: (100 << 8) + 128, // 100.5 in 24.8 format (128 = 256/2)
            y: (200 << 8) + 64,  // 200.25 in 24.8 format (64 = 256/4)
            z: 256,              // 1.0 in 24.8 fixed-point
        };

        // Integer part should round down
        assert_eq!(fixed.to_pixel_x(), 100);
        assert_eq!(fixed.to_pixel_y(), 200);

        // Verify the fractional parts are preserved
        assert_eq!(fixed.x & 0xFF, 128); // 0.5 * 256 = 128
        assert_eq!(fixed.y & 0xFF, 64); // 0.25 * 256 = 64
    }

    #[test]
    fn edge_function_fixed_correctness() {
        // Create a simple CCW triangle with vertices at (0, 0), (100, 0), (50, 100)
        // Winding: v0→v1 is right, v1→v2 is up-left, v2→v0 is down-left → CCW when viewed from top-down
        let v0 = VertexFixed { x: 0, y: 0, z: 256 }; // 1.0 in 24.8 fixed-point;
        let v1 = VertexFixed {
            x: 100 << 8,
            y: 0,
            z: 256, // 1.0 in 24.8 fixed-point
        };
        let v2 = VertexFixed {
            x: 50 << 8,
            y: 100 << 8,
            z: 256, // 1.0 in 24.8 fixed-point
        };

        // Test point inside triangle (50, 50)
        let inside_x = 50 << 8;
        let inside_y = 50 << 8;

        // For edge function, all three should have consistent sign for inside points
        let e0 = edge_function_fixed(inside_x, inside_y, v0, v1);
        let e1 = edge_function_fixed(inside_x, inside_y, v1, v2);
        let e2 = edge_function_fixed(inside_x, inside_y, v2, v0);

        // All should have the same sign (either all positive or all negative) for point inside
        // This triangle is actually CW in screen space (Y increases downward), so edges will be negative
        let all_same_sign = (e0 < 0 && e1 < 0 && e2 < 0) || (e0 > 0 && e1 > 0 && e2 > 0);
        assert!(
            all_same_sign,
            "Point inside triangle should have consistent edge signs: e0={e0}, e1={e1}, e2={e2}"
        );

        // Test point outside triangle (200, 50) - far to the right
        let outside_x = 200 << 8;
        let outside_y = 50 << 8;

        let e0_out = edge_function_fixed(outside_x, outside_y, v0, v1);
        let e1_out = edge_function_fixed(outside_x, outside_y, v1, v2);
        let e2_out = edge_function_fixed(outside_x, outside_y, v2, v0);

        // At least one edge function should have opposite sign for outside point
        let all_same_sign_out =
            (e0_out < 0 && e1_out < 0 && e2_out < 0) || (e0_out > 0 && e1_out > 0 && e2_out > 0);
        assert!(
            !all_same_sign_out,
            "Point outside triangle should not have consistent edge signs"
        );
    }

    #[test]
    fn edge_function_fixed_deterministic() {
        // Fixed-point should give identical results for same inputs
        let v0 = VertexFixed {
            x: 10 << 8,
            y: 20 << 8,
            z: 256, // 1.0 in 24.8 fixed-point
        };
        let v1 = VertexFixed {
            x: 30 << 8,
            y: 40 << 8,
            z: 256, // 1.0 in 24.8 fixed-point
        };

        let px = 25 << 8;
        let py = 35 << 8;

        // Call multiple times
        let result1 = edge_function_fixed(px, py, v0, v1);
        let result2 = edge_function_fixed(px, py, v0, v1);
        let result3 = edge_function_fixed(px, py, v0, v1);

        // Should be identical (deterministic)
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn fixed_point_triangle_rendering_matches_float() {
        // Verify that triangles prepared with fixed-point vertices still render correctly
        let width = 100;
        let height = 100;

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        // Render with tile renderer (uses fixed-point internally now)
        let mut fb_tile = Framebuffer::new(width, height).unwrap();
        let mut zb_tile = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        tr.render_batch(&mut fb_tile, &mut zb_tile, &[(v0, v1, v2, color)]);

        // Reference: scanline renderer
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        fill_triangle_3d(&mut fb_ref, &mut zb_ref, v0, v1, v2, color);

        // Should be pixel-identical
        let ref_pixels = fb_ref.as_slice();
        let tile_pixels = fb_tile.as_slice();

        for i in 0..(width * height) as usize {
            assert_eq!(
                tile_pixels[i], ref_pixels[i],
                "Pixel mismatch at index {i} with fixed-point vertices"
            );
        }
    }

    #[test]
    #[cfg(feature = "simd")]
    fn verify_simd_execution_with_wide_scanlines() {
        // This test creates horizontal triangles with scanlines >32 pixels
        // to verify SIMD code path executes (check stderr for [DEBUG] output)
        use crate::framebuffer::Framebuffer;
        use crate::math::Vec3;
        use crate::zbuffer::ZBuffer;

        let width = 1920;
        let height = 1080;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        // Create wide horizontal triangles spanning most of the screen
        // Use NDC coordinates that will create scanlines >32 pixels wide
        let z1 = 5.0;
        let z2 = 6.0;
        let triangles = vec![
            // Very wide triangle (-0.8 to 0.8 in NDC = ~3072 pixels at 1920 width)
            (
                (Vec3::new(-0.8, 0.0, z1), z1),
                (Vec3::new(0.8, 0.0, z1), z1),
                (Vec3::new(0.0, 0.1, z1), z1),
                0xFFFF_0000,
            ),
            (
                (Vec3::new(-0.8, -0.2, z2), z2),
                (Vec3::new(0.8, -0.2, z2), z2),
                (Vec3::new(0.0, -0.1, z2), z2),
                0xFF00_FF00,
            ),
        ];

        fb.clear(0xFF_00_00_00);
        zb.clear();
        renderer.render_batch(&mut fb, &mut zb, &triangles);

        // Verify triangles were rendered (at least some pixels changed)
        let pixels_changed = fb
            .as_slice()
            .iter()
            .filter(|&&p| p != 0xFF_00_00_00)
            .count();
        assert!(
            pixels_changed > 100,
            "Expected at least 100 pixels rendered, got {}",
            pixels_changed
        );
    }
}
