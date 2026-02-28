#![allow(clippy::collapsible_if)]
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
//! Use [`fill_triangle_3d`](super::fill_triangle_3d) otherwise.
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
//! use abrash::rasterizer::{TileRenderer, ClipTriangle};
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

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
use super::gouraud::draw_scanline_gouraud_simd_fast;
use super::gouraud::{GouraudEdgeWalker, GouraudGradients};
use super::texture::{draw_span_bilinear, draw_span_nearest, draw_span_trilinear};
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
use super::texture::{draw_span_bilinear_simd, draw_span_nearest_simd, draw_span_trilinear_simd};
use super::{
    EdgeWalker, PerspectiveSpanStart, PerspectiveTextureEdgeWalker, PerspectiveTextureGradients,
    RECIPROCAL_TABLE, is_backface, sort_by_y,
};
use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::hiz_buffer::{AABB3D, HiZBuffer};
use crate::math::{ScreenPoint, Vec2, Vec3, project_triangle_to_screen};
use crate::texture::{FilterMode, Texture};
use crate::zbuffer::ZBuffer;
use std::ops::{Deref, DerefMut};

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

#[allow(dead_code)]
struct AlignedBuffer<T> {
    _data: Vec<T>,
    ptr: *mut T,
    len: usize,
}

#[allow(dead_code)]
impl<T: Default + Copy> AlignedBuffer<T> {
    fn new(len: usize) -> Self {
        // We want 32-byte alignment.
        let align_bytes = 32;
        let elem_size = std::mem::size_of::<T>();
        // Ensure we allocate enough extra space to align the pointer
        // Worst case offset is align_bytes - 1. We need ceil((align_bytes)/elem_size) extra elements.
        let extra_elements = (align_bytes + elem_size - 1) / elem_size;

        let mut data = vec![T::default(); len + extra_elements];

        let start_ptr = data.as_mut_ptr();
        let start_addr = start_ptr as usize;

        // Calculate offset to next 32-byte boundary
        let offset_bytes = (align_bytes - (start_addr % align_bytes)) % align_bytes;
        // Assume elem_size divides align_bytes or at least offset_bytes (true for u32/f32 and align 32)
        let offset_elements = offset_bytes / elem_size;

        let ptr = unsafe { start_ptr.add(offset_elements) };

        Self {
            _data: data,
            ptr,
            len,
        }
    }
}

impl<T> Deref for AlignedBuffer<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> DerefMut for AlignedBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

unsafe impl<T: Send> Send for AlignedBuffer<T> {}
unsafe impl<T: Sync> Sync for AlignedBuffer<T> {}

/// Tile size in pixels. 32x32 = 1024 pixels * 4 bytes = 4KB per buffer.
pub const TILE_SIZE: u32 = 32;

/// A clip-space triangle with three vertices `(position, w)` and a flat color.
pub type ClipTriangle = ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32);

/// A clip-space triangle with three vertices `(position, w)` and UV coordinates.
pub type TexturedClipTriangle = ((Vec3, f32), Vec2, (Vec3, f32), Vec2, (Vec3, f32), Vec2);

/// A compact screen point for storing vertices in `PreparedTriangle`.
///
/// Reduces memory usage by using i16 for coordinates (sufficient for up to 32k resolution)
/// and omitting unused `inv_w` for flat shading.
#[derive(Clone, Copy, Debug)]
pub struct CompactScreenPoint {
    pub x: i16,
    pub y: i16,
    pub z: f32,
}

impl CompactScreenPoint {
    fn to_screen_point(self, inv_w: f32) -> ScreenPoint {
        ScreenPoint {
            x: i32::from(self.x),
            y: i32::from(self.y),
            z: self.z,
            inv_w,
        }
    }
}

/// A triangle that has been clipped, projected, culled, Y-sorted, and had gradients computed.
#[derive(Clone, Copy)]
pub struct PreparedTriangle {
    pub p0: CompactScreenPoint,
    pub p1: CompactScreenPoint,
    pub p2: CompactScreenPoint,
    pub dz_dx: f32,
    pub long_edge_is_left: bool,
    pub color: u32,
    pub aabb_min_x: i16,
    pub aabb_min_y: i16,
    pub aabb_max_x: i16,
    pub aabb_max_y: i16,
    pub min_depth: f32, // Minimum depth across triangle
    pub max_depth: f32, // Maximum depth across triangle
}

/// A Gouraud-shaded triangle prepared for rasterization.
#[derive(Clone, Copy)]
pub struct PreparedGouraudTriangle {
    pub p0: CompactScreenPoint,
    pub p1: CompactScreenPoint,
    pub p2: CompactScreenPoint,
    pub c0: (i32, i32, i32), // Fixed-point color at p0
    pub c1: (i32, i32, i32),
    pub c2: (i32, i32, i32),
    pub gradients: GouraudGradients,
    pub long_edge_is_left: bool,
    pub aabb_min_x: i16,
    pub aabb_min_y: i16,
    pub aabb_max_x: i16,
    pub aabb_max_y: i16,
    pub min_depth: f32,
    pub max_depth: f32,
}

/// A textured triangle prepared for rasterization.
///
/// Optimized to fit in exactly 128 bytes (2 cache lines).
#[derive(Clone, Copy)]
pub struct PreparedTexturedTriangle {
    pub p0: ScreenPoint,
    pub p1: ScreenPoint,
    pub p2: ScreenPoint,
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

use std::mem::MaybeUninit;

pub struct PreparedGouraudTrianglesList {
    pub tris: [MaybeUninit<PreparedGouraudTriangle>; 8],
    pub count: usize,
}

impl PreparedGouraudTrianglesList {
    pub fn new() -> Self {
        Self {
            tris: unsafe { MaybeUninit::uninit().assume_init() },
            count: 0,
        }
    }

    pub fn push(&mut self, tri: PreparedGouraudTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }
}

impl IntoIterator for PreparedGouraudTrianglesList {
    type Item = PreparedGouraudTriangle;
    type IntoIter = PreparedGouraudTrianglesIter;

    fn into_iter(self) -> Self::IntoIter {
        PreparedGouraudTrianglesIter {
            list: self,
            index: 0,
        }
    }
}

pub struct PreparedGouraudTrianglesIter {
    list: PreparedGouraudTrianglesList,
    index: usize,
}

impl Iterator for PreparedGouraudTrianglesIter {
    type Item = PreparedGouraudTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { self.list.tris[self.index].assume_init() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct PreparedTrianglesList {
    pub tris: [MaybeUninit<PreparedTriangle>; 8],
    pub count: usize,
}

impl PreparedTrianglesList {
    pub fn new() -> Self {
        Self {
            tris: unsafe { MaybeUninit::uninit().assume_init() },
            count: 0,
        }
    }

    pub fn push(&mut self, tri: PreparedTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }
}

impl IntoIterator for PreparedTrianglesList {
    type Item = PreparedTriangle;
    type IntoIter = PreparedTrianglesIter;

    fn into_iter(self) -> Self::IntoIter {
        PreparedTrianglesIter {
            list: self,
            index: 0,
        }
    }
}

pub struct PreparedTrianglesIter {
    list: PreparedTrianglesList,
    index: usize,
}

impl Iterator for PreparedTrianglesIter {
    type Item = PreparedTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { self.list.tris[self.index].assume_init() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct PreparedTexturedTrianglesList {
    pub tris: [MaybeUninit<PreparedTexturedTriangle>; 8],
    pub count: usize,
}

impl PreparedTexturedTrianglesList {
    pub fn new() -> Self {
        Self {
            tris: unsafe { MaybeUninit::uninit().assume_init() },
            count: 0,
        }
    }

    pub fn push(&mut self, tri: PreparedTexturedTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }
}

impl IntoIterator for PreparedTexturedTrianglesList {
    type Item = PreparedTexturedTriangle;
    type IntoIter = PreparedTexturedTrianglesIter;

    fn into_iter(self) -> Self::IntoIter {
        PreparedTexturedTrianglesIter {
            list: self,
            index: 0,
        }
    }
}

pub struct PreparedTexturedTrianglesIter {
    list: PreparedTexturedTrianglesList,
    index: usize,
}

impl Iterator for PreparedTexturedTrianglesIter {
    type Item = PreparedTexturedTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { self.list.tris[self.index].assume_init() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

/// Flattened linked-list structure for tile binning.
///
/// Replaces `Vec<Vec<usize>>` to reduce heap allocations and improve cache locality.
pub struct TileBins {
    pub heads: Vec<u32>, // Index into nexts/tris. u32::MAX = None
    pub tails: Vec<u32>, // Index into nexts/tris. u32::MAX = None
    pub nexts: Vec<u32>, // Link to next node
    pub tris: Vec<u32>,  // Triangle index
}

impl TileBins {
    pub fn new(num_tiles: usize) -> Self {
        Self {
            heads: vec![u32::MAX; num_tiles],
            tails: vec![u32::MAX; num_tiles],
            nexts: Vec::with_capacity(1024),
            tris: Vec::with_capacity(1024),
        }
    }

    pub fn clear(&mut self) {
        self.heads.fill(u32::MAX);
        self.tails.fill(u32::MAX);
        self.nexts.clear();
        self.tris.clear();
    }

    #[inline]
    pub fn push(&mut self, tile_idx: usize, tri_idx: usize) {
        let node_idx = self.tris.len() as u32;
        self.tris.push(tri_idx as u32);
        self.nexts.push(u32::MAX);

        let head = self.heads[tile_idx];
        if head == u32::MAX {
            self.heads[tile_idx] = node_idx;
        } else {
            let tail = self.tails[tile_idx];
            self.nexts[tail as usize] = node_idx;
        }
        self.tails[tile_idx] = node_idx;
    }

    pub fn iter(&self, tile_idx: usize) -> TileBinIter<'_> {
        TileBinIter {
            bins: self,
            curr: self.heads[tile_idx],
        }
    }
}

pub struct TileBinIter<'a> {
    bins: &'a TileBins,
    curr: u32,
}

impl<'a> Iterator for TileBinIter<'a> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr == u32::MAX {
            None
        } else {
            let idx = self.curr as usize;
            let tri_idx = self.bins.tris[idx] as usize;
            self.curr = self.bins.nexts[idx];
            Some(tri_idx)
        }
    }
}

/// Render a single tile: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_single_tile(
    tx: u32,
    ty: u32,
    tile_bins: &TileBins,
    prepared: &[PreparedTriangle],
    tiles_x: u32,
    width: u32,
    _height: u32,
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins.heads[bin_idx] == u32::MAX {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;

    // Compute Y range covered by triangles in this bin (partial tile clear)
    let mut clear_y_min = tile_y1;
    let mut clear_y_max = tile_y0;

    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        clear_y_min = clear_y_min.min(i32::from(tri.aabb_min_y).max(tile_y0));
        clear_y_max = clear_y_max.max(i32::from(tri.aabb_max_y).min(tile_y1 - 1));
    }

    // Clear only the rows that will be touched
    let row_start = ((clear_y_min - tile_y0) as u32 * TILE_SIZE) as usize;
    let row_end = (((clear_y_max - tile_y0) as u32 + 1) * TILE_SIZE) as usize;
    tile_pixels[row_start..row_end].fill(0xFF00_0000);
    tile_depths[row_start..row_end].fill(f32::INFINITY);

    // Render all triangles in bin
    let screen_w = width as i32;
    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile(
            tile_pixels,
            tile_depths,
            tri,
            tile_x0,
            tile_y0,
            tile_x1,
            tile_y1,
            screen_w,
        );
    }

    Some((clear_y_min, clear_y_max))
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
    let p0_y = i32::from(tri.p0.y);
    let p1_y = i32::from(tri.p1.y);
    let p2_y = i32::from(tri.p2.y);

    let y_start = p0_y.max(tile_y0);
    let y_end = p2_y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    // Reconstruct ScreenPoint for EdgeWalker (inv_w unused for flat shading)
    let p0 = ScreenPoint {
        x: i32::from(tri.p0.x),
        y: p0_y,
        z: tri.p0.z,
        inv_w: 1.0,
    };
    let p1 = ScreenPoint {
        x: i32::from(tri.p1.x),
        y: p1_y,
        z: tri.p1.z,
        inv_w: 1.0,
    };
    let p2 = ScreenPoint {
        x: i32::from(tri.p2.x),
        y: p2_y,
        z: tri.p2.z,
        inv_w: 1.0,
    };

    // Edge A: always p0→p2 (long edge)
    let mut edge_a = EdgeWalker::new(p0, p2);
    if y_start > p0_y {
        edge_a.step_n(i64::from(y_start) - i64::from(p0_y));
    }

    // Edge B: depends on whether y_start is above or below p1.y
    let mut edge_b = if y_start < p1_y {
        let mut e = EdgeWalker::new(p0, p1);
        if y_start > p0_y {
            e.step_n(i64::from(y_start) - i64::from(p0_y));
        }
        e
    } else {
        let mut e = EdgeWalker::new(p1, p2);
        if y_start > p1_y {
            e.step_n(i64::from(y_start) - i64::from(p1_y));
        }
        e
    };

    let dz_dx = tri.dz_dx;
    let color = tri.color;

    for y in y_start..=y_end {
        if y == p1_y && y != p0_y {
            edge_b = EdgeWalker::new(p1, p2);
        }

        let (x_start, x_end, z_left) = if tri.long_edge_is_left {
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
                if z_left < tile_depths[tile_idx] {
                    tile_depths[tile_idx] = z_left;
                    tile_pixels[tile_idx] = color;
                }
            }
        } else {
            // Clamp X to tile and screen bounds
            let xs = x_start.max(tile_x0).max(0);
            let xe = x_end.min(tile_x1 - 1).min(screen_x_max);

            if xs <= xe {
                // Calculate z at xs
                let dx_start = (i64::from(xs) - i64::from(x_start)) as f32;
                let z_at_xs = z_left + dx_start * dz_dx;

                let row_offset = ((y - tile_y0) as u32 * TILE_SIZE) as usize;
                let col_start = (xs - tile_x0) as usize;
                let col_end = (xe - tile_x0) as usize;

                let pixels = &mut tile_pixels[row_offset + col_start..=row_offset + col_end];
                let depths = &mut tile_depths[row_offset + col_start..=row_offset + col_end];

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                {
                    // Adaptive SIMD Rasterization
                    //
                    // History:
                    // - Previously disabled due to maskstore performance regression (4x slower).
                    // - Optimized to use _mm256_blendv_ps instead of maskstores.
                    // - Benchmarks verify ~11-14% speedup for large triangles (scanlines >= 32 pixels).
                    // - Adaptive threshold protects against regression on small triangles.
                    //
                    // See: benches/scanline_micro.rs results.
                    if pixels.len() >= 8 {
                        rasterize_scanline_simd(pixels, depths, z_at_xs, dz_dx, color);
                    } else {
                        rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
                    }
                }
                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
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
    tile_bins: &TileBins,
    prepared: &[PreparedTexturedTriangle],
    tiles_x: u32,
    width: u32,
    _height: u32,
    texture: &Texture,
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins.heads[bin_idx] == u32::MAX {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;

    // Compute Y range covered by triangles in this bin (partial tile clear)
    let mut clear_y_min = tile_y1;
    let mut clear_y_max = tile_y0;

    for tri_idx in tile_bins.iter(bin_idx) {
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
    for tri_idx in tile_bins.iter(bin_idx) {
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
        tri.p0,
        tri.p2,
        tri.p0.inv_w,
        tri.p2.inv_w,
        tri.u0,
        tri.u2,
        tri.v0,
        tri.v2,
    );
    if y_start > tri.p0.y {
        edge_a.step_n(i64::from(y_start) - i64::from(tri.p0.y));
    }

    let mut edge_b = if y_start < tri.p1.y {
        let mut e = PerspectiveTextureEdgeWalker::new(
            tri.p0,
            tri.p1,
            tri.p0.inv_w,
            tri.p1.inv_w,
            tri.u0,
            tri.u1,
            tri.v0,
            tri.v1,
        );
        if y_start > tri.p0.y {
            e.step_n(i64::from(y_start) - i64::from(tri.p0.y));
        }
        e
    } else {
        let mut e = PerspectiveTextureEdgeWalker::new(
            tri.p1,
            tri.p2,
            tri.p1.inv_w,
            tri.p2.inv_w,
            tri.u1,
            tri.u2,
            tri.v1,
            tri.v2,
        );
        if y_start > tri.p1.y {
            e.step_n(i64::from(y_start) - i64::from(tri.p1.y));
        }
        e
    };

    for y in y_start..=y_end {
        if y == tri.p1.y && y != tri.p0.y {
            edge_b = PerspectiveTextureEdgeWalker::new(
                tri.p1,
                tri.p2,
                tri.p1.inv_w,
                tri.p2.inv_w,
                tri.u1,
                tri.u2,
                tri.v1,
                tri.v2,
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
                            // Calculate LOD for single pixel
                            // For a single pixel, we can estimate gradients based on the triangle gradients
                            // projected to this pixel.
                            // q = 1/w.
                            // u_tex = u/q.
                            // du_tex/dx = (du/dx * q - u * dq/dx) / q^2
                            let w = 1.0 / q_left;
                            let w_sq = w * w;

                            let du_tex_dx = (tri.gradients.du_dx * q_left
                                - u_left * tri.gradients.dq_dx)
                                * w_sq;
                            let dv_tex_dx = (tri.gradients.dv_dx * q_left
                                - v_left * tri.gradients.dq_dx)
                                * w_sq;
                            let du_tex_dy = (tri.gradients.du_dy * q_left
                                - u_left * tri.gradients.dq_dy)
                                * w_sq;
                            let dv_tex_dy = (tri.gradients.dv_dy * q_left
                                - v_left * tri.gradients.dq_dy)
                                * w_sq;

                            let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
                                .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                            let lod = 0.5 * max_rho_sq.log2();
                            texture.get_pixel_trilinear(u_tex, v_tex, lod)
                        }
                    };
                }
            }
        } else {
            // Clamp X to tile and screen bounds
            let xs = x_start.max(tile_x0).max(0);
            let xe = x_end.min(tile_x1 - 1).min(screen_x_max);

            if xs <= xe {
                let dx_start = (i64::from(xs) - i64::from(x_start)) as f32;
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

        let pixels_slice = &mut pixels[i..i + count];
        let depths_slice = &mut depths[i..i + count];

        match texture.filter_mode {
            FilterMode::Nearest => {
                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_nearest_simd(
                            pixels_slice,
                            depths_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                        );
                    }
                } else {
                    draw_span_nearest(
                        pixels_slice,
                        depths_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                    );
                }

                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_span_nearest(
                    pixels_slice,
                    depths_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Bilinear => {
                let u_fix = ((u_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let v_fix = ((v_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_bilinear_simd(
                            pixels_slice,
                            depths_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                        );
                    }
                } else {
                    draw_span_bilinear(
                        pixels_slice,
                        depths_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                    );
                }

                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_span_bilinear(
                    pixels_slice,
                    depths_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Trilinear => {
                let w = w_start; // 1/q
                let w_sq = w * w;

                let du_tex_dx = (gradients.du_dx * q - u * gradients.dq_dx) * w_sq;
                let dv_tex_dx = (gradients.dv_dx * q - v * gradients.dq_dx) * w_sq;
                let du_tex_dy = (gradients.du_dy * q - u * gradients.dq_dy) * w_sq;
                let dv_tex_dy = (gradients.dv_dy * q - v * gradients.dq_dy) * w_sq;

                let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
                    .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                let lod = 0.5 * max_rho_sq.log2();

                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_trilinear_simd(
                            pixels_slice,
                            depths_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                            lod,
                        );
                    }
                } else {
                    draw_span_trilinear(
                        pixels_slice,
                        depths_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                        lod,
                    );
                }

                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_span_trilinear(
                    pixels_slice,
                    depths_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                    lod,
                );
            }
        }

        z += gradients.dz_dx * count as f32;
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
    let mut z = z_start;
    let len = pixels.len();
    let mut i = 0;

    // Unroll loop 4x for better pipeline utilization
    while i + 4 <= len {
        // SAFETY: Bounds checked by loop condition
        unsafe {
            // Pixel 0
            let d0 = depths.get_unchecked_mut(i);
            if z < *d0 {
                *d0 = z;
                *pixels.get_unchecked_mut(i) = color;
            }
            z += dz_dx;

            // Pixel 1
            let d1 = depths.get_unchecked_mut(i + 1);
            if z < *d1 {
                *d1 = z;
                *pixels.get_unchecked_mut(i + 1) = color;
            }
            z += dz_dx;

            // Pixel 2
            let d2 = depths.get_unchecked_mut(i + 2);
            if z < *d2 {
                *d2 = z;
                *pixels.get_unchecked_mut(i + 2) = color;
            }
            z += dz_dx;

            // Pixel 3
            let d3 = depths.get_unchecked_mut(i + 3);
            if z < *d3 {
                *d3 = z;
                *pixels.get_unchecked_mut(i + 3) = color;
            }
            z += dz_dx;
        }
        i += 4;
    }

    // Handle remaining pixels
    for k in i..len {
        unsafe {
            let d = depths.get_unchecked_mut(k);
            if z < *d {
                *d = z;
                *pixels.get_unchecked_mut(k) = color;
            }
        }
        z += dz_dx;
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
    use std::arch::x86_64::{
        __m256i, _CMP_LT_OQ, _mm256_add_ps, _mm256_blendv_ps, _mm256_castps_si256,
        _mm256_castsi256_ps, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_loadu_si256,
        _mm256_movemask_ps, _mm256_mul_ps, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps,
        _mm256_storeu_ps, _mm256_storeu_si256,
    };

    let len = pixels.len();
    let mut i = 0;

    // --- Optimization Idea 1: Align the loop ---
    // Handle the first few pixels (0-7) with scalar code until the pointer is 32-byte aligned.
    // AVX2 loads/stores are faster when aligned to 32 bytes (256 bits).
    // The depths buffer is allocated via AlignedBuffer so it's aligned, but `pixels`
    // is a slice into that buffer, so it might start at an unaligned offset depending on x_start.
    //
    // Actually, `TileRenderer` slices `tile_pixels` based on `x - tile_x0`. Since `tile_x0`
    // is always a multiple of 32, and `AlignedBuffer` is 32-byte aligned, the offset depends on `x`.
    // We align based on the destination address of `pixels` (color buffer).
    // Depths and pixels have the same alignment offset relative to 32 bytes because they are
    // both accessed with the same index `i`.

    let align_mask = 0x1F; // 32 bytes - 1
    let addr = pixels.as_ptr() as usize;
    let misalign = addr & align_mask;
    let pre_simd_count = if misalign == 0 {
        0
    } else {
        (32 - misalign) / 4 // 4 bytes per pixel
    };

    // Ensure we don't overrun the buffer if it's very small
    let pre_simd_count = pre_simd_count.min(len);

    let mut z = z_at_xs;

    // Process initial unaligned pixels
    for k in 0..pre_simd_count {
        unsafe {
            let d = depths.get_unchecked_mut(k);
            if z < *d {
                *d = z;
                *pixels.get_unchecked_mut(k) = color;
            }
        }
        z += dz_dx;
    }

    i += pre_simd_count;

    // --- Main SIMD Loop (Aligned) ---
    unsafe {
        use std::arch::x86_64::{_CMP_GE_OQ, _mm256_cmp_ps, _mm256_store_ps, _mm256_store_si256};

        // Setup: stride vector for incrementing depths by 8*dz_dx per iteration
        let stride_vec = _mm256_set1_ps(8.0 * dz_dx);

        // Initialize depth vector using vector arithmetic:
        // depths = z (current) + [0, 1, 2, 3, 4, 5, 6, 7] * dz_dx
        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let dz_vec = _mm256_set1_ps(dz_dx);
        let base = _mm256_set1_ps(z);
        let mut depths_vec = _mm256_add_ps(base, _mm256_mul_ps(offsets, dz_vec));

        let color_vec = _mm256_set1_epi32(color as i32);

        // Process 8 pixels at a time with AVX2
        while i + 8 <= len {
            // Load zbuffer values for 8 pixels
            // If we aligned correctly, this should be an aligned load for `pixels`.
            // However, `depths` buffer is separate. `AlignedBuffer` ensures start is aligned.
            // Since we advanced `i` to align `pixels`, `depths` at `i` is also aligned
            // ONLY IF `tile_pixels` and `tile_depths` had same initial alignment modulo 32.
            // `AlignedBuffer::new` ensures 32-byte alignment for start.
            // Since we index both by the same `i` (relative to start of slice), and slices start
            // at same offset relative to aligned base (same x_start), they are both aligned.
            //
            // Use aligned load/store intrinsics where possible.
            // Note: `loadu` is still safe and fast on Haswell+ even if aligned.
            // `store` (aligned) traps if unaligned, so we must be sure.
            // We aligned based on `pixels`. `depths` should match.

            let zb_ptr = depths.as_mut_ptr().add(i);
            // We use loadu just to be safe in case of weird offsets, but stores will be aligned.
            // Actually, let's use loadu for reads to be robust, and aligned stores because we calculated alignment.
            let zb_vals = _mm256_loadu_ps(zb_ptr);

            // Optimization Idea 2: Early Out (Occlusion Culling)
            // Check if ALL pixels fail the depth test (depth >= zbuffer)
            // _CMP_GE_OQ: Greater-than or Equal (Ordered, Non-signaling)
            let ge_mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_GE_OQ);
            let ge_bits = _mm256_movemask_ps(ge_mask);

            if ge_bits == 0xFF {
                // All pixels occluded. Skip write.
            } else {
                // At least one pixel is visible.
                // Compare: depth < zbuffer
                let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);
                let mask_bits = _mm256_movemask_ps(mask);

                if mask_bits == 0xFF {
                    // Fast path: All pixels visible.
                    // Store depths and colors directly using Aligned Stores.
                    _mm256_store_ps(zb_ptr, depths_vec);

                    let pixels_ptr = pixels.as_mut_ptr().add(i) as *mut __m256i;
                    _mm256_store_si256(pixels_ptr, color_vec);
                } else {
                    // Partial write path
                    // 1. Update depths
                    let blended_depths = _mm256_blendv_ps(zb_vals, depths_vec, mask);
                    // Use aligned store since we are aligned
                    _mm256_store_ps(zb_ptr, blended_depths);

                    // 2. Update pixels
                    let pixels_ptr = pixels.as_mut_ptr().add(i) as *mut __m256i;
                    // Read old pixels (aligned load)
                    let old_pixels = _mm256_load_si256(pixels_ptr as *const __m256i);

                    let old_pixels_ps = _mm256_castsi256_ps(old_pixels);
                    let color_vec_ps = _mm256_castsi256_ps(color_vec);

                    let blended_pixels_ps = _mm256_blendv_ps(old_pixels_ps, color_vec_ps, mask);

                    // Aligned store
                    _mm256_store_si256(pixels_ptr, _mm256_castps_si256(blended_pixels_ps));
                }
            }

            // Increment depths by stride (8*dz_dx) for next iteration
            depths_vec = _mm256_add_ps(depths_vec, stride_vec);
            i += 8;
        }

        // Update the scalar z tracker for the tail loop
        // z corresponds to depth at 'i' (start of this iteration block)
        // But we incremented depths_vec already for the *next* block.
        // We need to sync the scalar 'z' to the current 'i'.
        // Actually, easiest is just to recalculate z from scratch or extract from vector.
        // Or just maintain 'z' mathematically.
        // We've processed `i` pixels (including pre-simd).
        // `z` variable currently holds value at start of SIMD loop.
        // We need z at `i` (current).
        // Since we didn't update scalar `z` inside SIMD loop, we do it now.
        // The SIMD loop ran (i - pre_simd_count) / 8 iterations.
        let simd_pixels = i - pre_simd_count;
        z += (simd_pixels as f32) * dz_dx;
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
/// [`fill_triangle_3d`](super::fill_triangle_3d) instead,
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
    tile_pixels: AlignedBuffer<u32>,
    #[cfg(not(feature = "parallel"))]
    tile_depths: AlignedBuffer<f32>,
    tiles_x: u32,
    tiles_y: u32,
    width: u32,
    height: u32,
    tile_bins: TileBins,
    prepared: Vec<PreparedTriangle>,
    prepared_gouraud: Vec<PreparedGouraudTriangle>,
    prepared_textured: Vec<PreparedTexturedTriangle>,
    hiz_buffer: Option<HiZBuffer>,
    #[cfg(feature = "gpu-binning")]
    gpu_binner: Option<crate::gpu::GpuBinner>,
    use_two_level_binning: bool,
    // Pre-calculated half dimensions for projection
    half_width: f32,
    half_height: f32,
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
            tile_pixels: AlignedBuffer::new(tile_area),
            #[cfg(not(feature = "parallel"))]
            tile_depths: AlignedBuffer::new(tile_area),
            tiles_x,
            tiles_y,
            width,
            height,
            tile_bins: TileBins::new(tile_count),
            prepared: Vec::new(),
            prepared_gouraud: Vec::new(),
            prepared_textured: Vec::new(),
            hiz_buffer: None,
            #[cfg(feature = "gpu-binning")]
            gpu_binner: None,
            use_two_level_binning: false,
            half_width: width as f32 * 0.5,
            half_height: height as f32 * 0.5,
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

    /// Enable software-based two-level hierarchical binning.
    ///
    /// This method enables a software optimization that uses a two-level binning strategy:
    /// 1. Coarse binning: Triangles are first checked against large 128x128 pixel bins.
    /// 2. Hi-Z culling: Coarse bins are checked for visibility against the Hi-Z buffer (if enabled).
    /// 3. Fine binning: Only visible coarse bins are subdivided into 32x32 tiles.
    ///
    /// This is particularly effective for large triangles or when Hi-Z culling is enabled,
    /// as it allows skipping fine-grained binning for occluded regions.
    pub fn enable_software_two_level_binning(&mut self) {
        self.use_two_level_binning = true;
        // Two-level binning benefits significantly from Hi-Z, so enable it if not already enabled
        if self.hiz_buffer.is_none() {
            self.enable_hiz();
        }
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

    /// Begin a new frame. Clears internal buffers and prepares for triangle submission.
    ///
    /// This should be called before submitting any meshes via `submit_mesh`.
    pub fn begin_frame(&mut self) {
        self.prepared.clear();
        self.prepared_gouraud.clear();
        self.prepared_textured.clear();
        self.tile_bins.clear();
    }

    /// Submit a mesh for rendering.
    ///
    /// This method iterates over the indices, fetches vertices from the `vertices` buffer,
    /// and prepares triangles for rasterization (clipping, projection, etc.).
    ///
    /// # Arguments
    ///
    /// * `indices` - A list of triangle indices (triplets of indices into `vertices`).
    /// * `vertices` - A buffer of transformed vertices (position + w).
    /// * `color` - The flat color of the mesh.
    pub fn submit_mesh(&mut self, indices: &[[usize; 3]], vertices: &[(Vec3, f32)], color: u32) {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let width = self.width;
            let height = self.height;
            let half_width = self.half_width;
            let half_height = self.half_height;

            // Process triangles in parallel and collect prepared results
            let results: Vec<PreparedTriangle> = indices
                .par_iter()
                .fold(Vec::new, |mut acc, &[i0, i1, i2]| {
                    // Safety: We trust the indices are within bounds of the vertices slice.
                    // The caller must ensure this or it will panic inside the thread.
                    let v0 = vertices[i0];
                    let v1 = vertices[i1];
                    let v2 = vertices[i2];

                    let tris = Self::prepare_triangle_static(
                        v0,
                        v1,
                        v2,
                        color,
                        width,
                        height,
                        half_width,
                        half_height,
                    );
                    acc.extend(tris);
                    acc
                })
                .flatten()
                .collect();

            self.prepared.extend(results);
        }

        #[cfg(not(feature = "parallel"))]
        {
            for &[i0, i1, i2] in indices {
                // Using direct indexing which panics on out-of-bounds, ensuring safety
                let v0 = vertices[i0];
                let v1 = vertices[i1];
                let v2 = vertices[i2];

                self.prepare_triangle(v0, v1, v2, color);
            }
        }
    }

    /// Finish the frame: bin triangles, build Hi-Z, render tiles, and merge to framebuffer.
    ///
    /// # Panics
    ///
    /// Panics if framebuffer or zbuffer dimensions do not match the renderer configuration.
    pub fn end_frame(&mut self, fb: &mut Framebuffer, zb: &mut ZBuffer) {
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

        // Build Hi-Z pyramid from previous frame (temporal coherence)
        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid(zb);
            }
        }

        // Currently, TileRenderer handles either flat or textured batches per frame (via tile_bins index reuse).
        // If 'prepared' is non-empty, we assume flat rendering mode.
        if !self.prepared.is_empty() {
            // Phase 2: Bin (GPU or CPU with optional Hi-Z occlusion culling)
            #[cfg(feature = "gpu-binning")]
            if let Some(ref mut gpu) = self.gpu_binner {
                // GPU binning path - check if two-level binning is enabled
                if gpu.is_two_level_enabled() {
                    // Two-level hierarchical binning with Hi-Z culling
                    match gpu.bin_triangles_two_level(
                        &self.prepared,
                        self.hiz_buffer.as_ref(),
                        &mut self.tile_bins.heads,
                        &mut self.tile_bins.tails,
                        &mut self.tile_bins.nexts,
                        &mut self.tile_bins.tris,
                    ) {
                        Ok(_stats) => {
                            // Two-level binning succeeded
                        }
                        Err(e) => {
                            eprintln!("Two-level GPU binning failed: {e}, falling back to CPU");
                            self.bin_triangles_cpu();
                        }
                    }
                } else {
                    // Single-level GPU binning
                    if let Err(e) = gpu.bin_triangles(
                        &self.prepared,
                        &mut self.tile_bins.heads,
                        &mut self.tile_bins.tails,
                        &mut self.tile_bins.nexts,
                        &mut self.tile_bins.tris,
                    ) {
                        eprintln!("GPU binning failed: {e}, falling back to CPU");
                        self.bin_triangles_cpu();
                    }
                }
            } else {
                self.bin_triangles_cpu();
            }

            #[cfg(not(feature = "gpu-binning"))]
            self.bin_triangles_cpu();

            // Sort triangles front-to-back for early-Z optimization
            self.sort_bins_flat();

            // Phase 3+4: Render and merge each tile
            #[cfg(not(feature = "parallel"))]
            {
                // Sequential rendering
                for ty in 0..self.tiles_y {
                    for tx in 0..self.tiles_x {
                        if let Some((clear_y_min, clear_y_max)) = render_single_tile(
                            tx,
                            ty,
                            &self.tile_bins,
                            &self.prepared,
                            self.tiles_x,
                            self.width,
                            self.height,
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

                // SAFETY: Each tile writes to a non-overlapping region of the framebuffer/zbuffer.
                unsafe {
                    let fb_ptr = SendPtr(fb.as_mut_slice().as_mut_ptr());
                    let zb_ptr = SendPtr(zb.as_mut_slice().as_mut_ptr());
                    let width = self.width;
                    let height = self.height;
                    let tiles_x = self.tiles_x;
                    let tile_bins = &self.tile_bins;
                    let prepared = &self.prepared;

                    tiles.par_iter().for_each_init(
                        || {
                            let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                            (vec![0u32; tile_area], vec![f32::INFINITY; tile_area])
                        },
                        |buffers, &(tx, ty)| {
                            let (tile_pixels, tile_depths) = &mut *buffers;
                            if let Some((clear_y_min, clear_y_max)) = render_single_tile(
                                tx,
                                ty,
                                tile_bins,
                                prepared,
                                tiles_x,
                                width,
                                height,
                                tile_pixels,
                                tile_depths,
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
                                        fb_ptr.write(
                                            fb_start + col,
                                            tile_pixels[tile_row_offset + col],
                                        );
                                        zb_ptr.write(
                                            fb_start + col,
                                            tile_depths[tile_row_offset + col],
                                        );
                                    }
                                }
                            }
                        },
                    );
                }
            }
        }

        // Invalidate Hi-Z for next frame
        if let Some(ref mut hiz) = self.hiz_buffer {
            hiz.invalidate();
        }
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
    /// # Panics
    ///
    /// Panics if framebuffer or zbuffer dimensions do not match the renderer configuration.
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
    /// # use abrash::rasterizer::TileRenderer;
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
        self.begin_frame();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let width = self.width;
            let height = self.height;
            let half_width = self.half_width;
            let half_height = self.half_height;

            let results: Vec<PreparedTriangle> = triangles
                .par_iter()
                .fold(Vec::new, |mut acc, &(v0, v1, v2, color)| {
                    let tris = Self::prepare_triangle_static(
                        v0,
                        v1,
                        v2,
                        color,
                        width,
                        height,
                        half_width,
                        half_height,
                    );
                    acc.extend(tris);
                    acc
                })
                .flatten()
                .collect();

            self.prepared.extend(results);
        }

        #[cfg(not(feature = "parallel"))]
        {
            for &(v0, v1, v2, color) in triangles {
                self.prepare_triangle(v0, v1, v2, color);
            }
        }

        self.end_frame(fb, zb);
    }

    /// Render a batch of textured clip-space triangles.
    ///
    /// # Panics
    ///
    /// Panics if framebuffer or zbuffer dimensions do not match the renderer configuration.
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
        self.tile_bins.clear();

        // Phase 1: Prepare
        let tex_w = texture.width as f32;
        let tex_h = texture.height as f32;

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let width = self.width;
            let height = self.height;
            let half_width = self.half_width;
            let half_height = self.half_height;

            let results: Vec<PreparedTexturedTriangle> = triangles
                .par_iter()
                .fold(Vec::new, |mut acc, &(v0, uv0, v1, uv1, v2, uv2)| {
                    let tris = Self::prepare_triangle_textured_static(
                        (v0, uv0),
                        (v1, uv1),
                        (v2, uv2),
                        tex_w,
                        tex_h,
                        width,
                        height,
                        half_width,
                        half_height,
                    );
                    acc.extend(tris);
                    acc
                })
                .flatten()
                .collect();

            self.prepared_textured.extend(results);
        }

        #[cfg(not(feature = "parallel"))]
        {
            for &(v0, uv0, v1, uv1, v2, uv2) in triangles {
                self.prepare_triangle_textured((v0, uv0), (v1, uv1), (v2, uv2), tex_w, tex_h);
            }
        }

        // Build Hi-Z pyramid from previous frame (temporal coherence)
        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid(zb);
            }
        }

        // Phase 2: Bin (CPU only for now)
        self.bin_triangles_textured_cpu();

        // Sort triangles front-to-back for early-Z optimization
        self.sort_bins_textured();

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

                tiles.par_iter().for_each_init(
                    || {
                        let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                        (vec![0u32; tile_area], vec![f32::INFINITY; tile_area])
                    },
                    |buffers, &(tx, ty)| {
                        let (tile_pixels, tile_depths) = &mut *buffers;
                        if let Some((clear_y_min, clear_y_max)) = render_single_tile_textured(
                            tx,
                            ty,
                            tile_bins,
                            prepared,
                            tiles_x,
                            width,
                            height,
                            texture,
                            tile_pixels,
                            tile_depths,
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
                                    fb_ptr
                                        .write(fb_start + col, tile_pixels[tile_row_offset + col]);
                                    zb_ptr
                                        .write(fb_start + col, tile_depths[tile_row_offset + col]);
                                }
                            }
                        }
                    },
                );
            }
        }

        // Invalidate Hi-Z for next frame
        if let Some(ref mut hiz) = self.hiz_buffer {
            hiz.invalidate();
        }
    }

    /// Render a batch of gouraud-shaded clip-space triangles.
    ///
    /// # Panics
    ///
    /// Panics if framebuffer or zbuffer dimensions do not match the renderer configuration.
    pub fn render_batch_gouraud(
        &mut self,
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        triangles: &[(
            ((Vec3, f32), Vec3),
            ((Vec3, f32), Vec3),
            ((Vec3, f32), Vec3),
        )],
    ) {
        assert_eq!(fb.width(), self.width);
        assert_eq!(fb.height(), self.height);
        assert_eq!(zb.width(), self.width);
        assert_eq!(zb.height(), self.height);

        self.prepared_gouraud.clear();
        self.tile_bins.clear();

        // Phase 1: Prepare
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let width = self.width;
            let height = self.height;
            let half_width = self.half_width;
            let half_height = self.half_height;

            let results: Vec<PreparedGouraudTriangle> = triangles
                .par_iter()
                .fold(Vec::new, |mut acc, &(v0, v1, v2)| {
                    let tris = Self::prepare_triangle_gouraud_static(
                        v0,
                        v1,
                        v2,
                        width,
                        height,
                        half_width,
                        half_height,
                    );
                    acc.extend(tris);
                    acc
                })
                .flatten()
                .collect();

            self.prepared_gouraud.extend(results);
        }

        #[cfg(not(feature = "parallel"))]
        {
            for &(v0, v1, v2) in triangles {
                self.prepare_triangle_gouraud(v0, v1, v2);
            }
        }

        // Build Hi-Z pyramid from previous frame (temporal coherence)
        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid(zb);
            }
        }

        // Phase 2: Bin (CPU only)
        self.bin_triangles_gouraud_cpu();

        // Sort triangles front-to-back for early-Z optimization
        self.sort_bins_gouraud();

        // Phase 3+4: Render and merge each tile
        #[cfg(not(feature = "parallel"))]
        {
            // Sequential rendering
            for ty in 0..self.tiles_y {
                for tx in 0..self.tiles_x {
                    if let Some((clear_y_min, clear_y_max)) = render_single_tile_gouraud(
                        tx,
                        ty,
                        &self.tile_bins,
                        &self.prepared_gouraud,
                        self.tiles_x,
                        self.width,
                        self.height,
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
                let prepared = &self.prepared_gouraud;

                tiles.par_iter().for_each_init(
                    || {
                        let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                        (vec![0u32; tile_area], vec![f32::INFINITY; tile_area])
                    },
                    |buffers, &(tx, ty)| {
                        let (tile_pixels, tile_depths) = &mut *buffers;
                        if let Some((clear_y_min, clear_y_max)) = render_single_tile_gouraud(
                            tx,
                            ty,
                            tile_bins,
                            prepared,
                            tiles_x,
                            width,
                            height,
                            tile_pixels,
                            tile_depths,
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
                                    fb_ptr
                                        .write(fb_start + col, tile_pixels[tile_row_offset + col]);
                                    zb_ptr
                                        .write(fb_start + col, tile_depths[tile_row_offset + col]);
                                }
                            }
                        }
                    },
                );
            }
        }

        // Invalidate Hi-Z for next frame
        if let Some(ref mut hiz) = self.hiz_buffer {
            hiz.invalidate();
        }
    }

    #[cfg_attr(feature = "parallel", allow(dead_code))]
    fn prepare_triangle_gouraud(
        &mut self,
        v0: ((Vec3, f32), Vec3),
        v1: ((Vec3, f32), Vec3),
        v2: ((Vec3, f32), Vec3),
    ) {
        let results = Self::prepare_triangle_gouraud_static(
            v0,
            v1,
            v2,
            self.width,
            self.height,
            self.half_width,
            self.half_height,
        );
        self.prepared_gouraud.extend(results);
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_triangle_gouraud_static(
        v0: ((Vec3, f32), Vec3),
        v1: ((Vec3, f32), Vec3),
        v2: ((Vec3, f32), Vec3),
        width: u32,
        height: u32,
        half_width: f32,
        half_height: f32,
    ) -> PreparedGouraudTrianglesList {
        let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);
        let mut results = PreparedGouraudTrianglesList::new();

        for i in 0..clipped.count {
            let base = i * 3;
            let v0 = clipped[base];
            let v1 = clipped[base + 1];
            let v2 = clipped[base + 2];

            let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
                v0.0.0,
                v0.0.1,
                v1.0.0,
                v1.0.1,
                v2.0.0,
                v2.0.1,
                half_width,
                half_height,
            );

            if is_backface(p0_orig, p1_orig, p2_orig) {
                continue;
            }

            // Scale colors to 0..255 for fixed point
            let c0 = v0.1 * 255.0;
            let c1 = v1.1 * 255.0;
            let c2 = v2.1 * 255.0;

            let mut verts = [(p0_orig, c0), (p1_orig, c1), (p2_orig, c2)];
            sort_by_y(&mut verts, |(p, _)| p.y);
            let [(p0, c0), (p1, c1), (p2, c2)] = verts;

            let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            if total_height == 0.0 {
                continue;
            }

            // Gradients
            let (gradients, long_edge_is_left) = GouraudGradients::new(p0, p1, p2, c0, c1, c2);

            // AABB
            let min_x = p0.x.min(p1.x).min(p2.x).max(0);
            let min_y = p0.y.max(0);
            let max_x = p0.x.max(p1.x).max(p2.x).min(width as i32 - 1);
            let max_y = p2.y.min(height as i32 - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            let min_depth = p0.z.min(p1.z).min(p2.z);
            let max_depth = p0.z.max(p1.z).max(p2.z);

            // Store colors as 16.16 fixed point tuples for starting values
            let c0_fixed = (
                (c0.x * 65536.0) as i32,
                (c0.y * 65536.0) as i32,
                (c0.z * 65536.0) as i32,
            );
            let c1_fixed = (
                (c1.x * 65536.0) as i32,
                (c1.y * 65536.0) as i32,
                (c1.z * 65536.0) as i32,
            );
            let c2_fixed = (
                (c2.x * 65536.0) as i32,
                (c2.y * 65536.0) as i32,
                (c2.z * 65536.0) as i32,
            );

            results.push(PreparedGouraudTriangle {
                p0: CompactScreenPoint {
                    x: p0.x as i16,
                    y: p0.y as i16,
                    z: p0.z,
                },
                p1: CompactScreenPoint {
                    x: p1.x as i16,
                    y: p1.y as i16,
                    z: p1.z,
                },
                p2: CompactScreenPoint {
                    x: p2.x as i16,
                    y: p2.y as i16,
                    z: p2.z,
                },
                c0: c0_fixed,
                c1: c1_fixed,
                c2: c2_fixed,
                gradients,
                long_edge_is_left,
                aabb_min_x: min_x as i16,
                aabb_min_y: min_y as i16,
                aabb_max_x: max_x as i16,
                aabb_max_y: max_y as i16,
                min_depth,
                max_depth,
            });
        }
        results
    }

    fn bin_triangles_gouraud_cpu(&mut self) {
        let prepared_len = self.prepared_gouraud.len();
        for i in 0..prepared_len {
            if let Some(ref hiz) = self.hiz_buffer {
                let tri = &self.prepared_gouraud[i];
                let aabb = AABB3D {
                    min_x: i32::from(tri.aabb_min_x),
                    max_x: i32::from(tri.aabb_max_x),
                    min_y: i32::from(tri.aabb_min_y),
                    max_y: i32::from(tri.aabb_max_y),
                    min_depth: tri.min_depth,
                    max_depth: tri.max_depth,
                };

                if !hiz.is_potentially_visible(aabb) {
                    continue;
                }
            }
            self.bin_triangle_gouraud(i);
        }
    }

    fn bin_triangle_gouraud(&mut self, tri_idx: usize) {
        let tri = &self.prepared_gouraud[tri_idx];
        let tile_size_i32 = TILE_SIZE as i32;

        let tx_min = (i32::from(tri.aabb_min_x) / tile_size_i32) as u32;
        let ty_min = (i32::from(tri.aabb_min_y) / tile_size_i32) as u32;
        let tx_max = ((i32::from(tri.aabb_max_x) / tile_size_i32) as u32).min(self.tiles_x - 1);
        let ty_max = ((i32::from(tri.aabb_max_y) / tile_size_i32) as u32).min(self.tiles_y - 1);

        for ty in ty_min..=ty_max {
            for tx in tx_min..=tx_max {
                let bin_idx = (ty * self.tiles_x + tx) as usize;
                self.tile_bins.push(bin_idx, tri_idx);
            }
        }
    }

    /// Sorts gouraud triangles in each bin by depth.
    fn sort_bins_gouraud(&mut self) {
        // FIXME: Sorting disabled
        /*
        let prepared_gouraud = &self.prepared_gouraud;
        if prepared_gouraud.is_empty() {
            return;
        }

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            self.tile_bins.par_iter_mut().for_each(|bin| {
                bin.sort_unstable_by(|&a, &b| {
                    let depth_a = unsafe { prepared_gouraud.get_unchecked(a).min_depth };
                    let depth_b = unsafe { prepared_gouraud.get_unchecked(b).min_depth };
                    depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
                });
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for bin in &mut self.tile_bins {
                bin.sort_unstable_by(|&a, &b| {
                    let depth_a = unsafe { prepared_gouraud.get_unchecked(a).min_depth };
                    let depth_b = unsafe { prepared_gouraud.get_unchecked(b).min_depth };
                    depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        */
    }

    #[cfg_attr(feature = "parallel", allow(dead_code))]
    fn prepare_triangle_textured(
        &mut self,
        v0: ((Vec3, f32), Vec2),
        v1: ((Vec3, f32), Vec2),
        v2: ((Vec3, f32), Vec2),
        tex_w: f32,
        tex_h: f32,
    ) {
        let results = Self::prepare_triangle_textured_static(
            v0,
            v1,
            v2,
            tex_w,
            tex_h,
            self.width,
            self.height,
            self.half_width,
            self.half_height,
        );
        self.prepared_textured.extend(results);
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_triangle_textured_static(
        v0: ((Vec3, f32), Vec2),
        v1: ((Vec3, f32), Vec2),
        v2: ((Vec3, f32), Vec2),
        tex_w: f32,
        tex_h: f32,
        width: u32,
        height: u32,
        half_width: f32,
        half_height: f32,
    ) -> PreparedTexturedTrianglesList {
        let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);
        let mut results = PreparedTexturedTrianglesList::new();

        for i in 0..clipped.count {
            let base = i * 3;
            let cv0 = clipped[base];
            let cv1 = clipped[base + 1];
            let cv2 = clipped[base + 2];

            let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
                cv0.0.0,
                cv0.0.1,
                cv1.0.0,
                cv1.0.1,
                cv2.0.0,
                cv2.0.1,
                half_width,
                half_height,
            );

            if is_backface(p0_orig, p1_orig, p2_orig) {
                continue;
            }

            // Optimization: Reuse inv_w from projection
            let inv_w0 = p0_orig.inv_w;
            let inv_w1 = p1_orig.inv_w;
            let inv_w2 = p2_orig.inv_w;

            let u0 = cv0.1.x * tex_w * inv_w0;
            let v0_val = cv0.1.y * tex_h * inv_w0;
            let u1 = cv1.1.x * tex_w * inv_w1;
            let v1_val = cv1.1.y * tex_h * inv_w1;
            let u2 = cv2.1.x * tex_w * inv_w2;
            let v2_val = cv2.1.y * tex_h * inv_w2;

            let mut verts = [
                (p0_orig, u0, v0_val),
                (p1_orig, u1, v1_val),
                (p2_orig, u2, v2_val),
            ];
            sort_by_y(&mut verts, |(p, _, _)| p.y);
            let [(p0, u0, v0), (p1, u1, v1), (p2, u2, v2)] = verts;

            let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            if total_height == 0.0 {
                continue;
            }

            // Gradients
            let (gradients, long_edge_is_left) = PerspectiveTextureGradients::new_with_winding(
                p0, p1, p2, p0.inv_w, p1.inv_w, p2.inv_w, u0, u1, u2, v0, v1, v2,
            );

            // AABB
            let min_x = p0.x.min(p1.x).min(p2.x).max(0);
            let min_y = p0.y.max(0);
            let max_x = p0.x.max(p1.x).max(p2.x).min(width as i32 - 1);
            let max_y = p2.y.min(height as i32 - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            let min_depth = p0.z.min(p1.z).min(p2.z);
            let max_depth = p0.z.max(p1.z).max(p2.z);

            results.push(PreparedTexturedTriangle {
                p0,
                p1,
                p2,
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
        results
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
                self.tile_bins.push(bin_idx, tri_idx);
            }
        }
    }

    #[cfg_attr(feature = "parallel", allow(dead_code))]
    fn prepare_triangle(&mut self, v0: (Vec3, f32), v1: (Vec3, f32), v2: (Vec3, f32), color: u32) {
        let results = Self::prepare_triangle_static(
            v0,
            v1,
            v2,
            color,
            self.width,
            self.height,
            self.half_width,
            self.half_height,
        );
        self.prepared.extend(results);
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_triangle_static(
        v0: (Vec3, f32),
        v1: (Vec3, f32),
        v2: (Vec3, f32),
        color: u32,
        width: u32,
        height: u32,
        half_width: f32,
        half_height: f32,
    ) -> PreparedTrianglesList {
        let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| (v.0, v.1));
        let mut results = PreparedTrianglesList::new();

        for i in 0..clipped.count {
            let base = i * 3;
            let cv0 = clipped[base];
            let cv1 = clipped[base + 1];
            let cv2 = clipped[base + 2];

            let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
                cv0.0,
                cv0.1,
                cv1.0,
                cv1.1,
                cv2.0,
                cv2.1,
                half_width,
                half_height,
            );

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
            let max_x = p0.x.max(p1.x).max(p2.x).min(width as i32 - 1);
            let max_y = p2.y.min(height as i32 - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            // Compute min/max depth for Hi-Z occlusion culling
            let min_depth = p0.z.min(p1.z).min(p2.z);
            let max_depth = p0.z.max(p1.z).max(p2.z);

            results.push(PreparedTriangle {
                p0: CompactScreenPoint {
                    x: p0.x as i16,
                    y: p0.y as i16,
                    z: p0.z,
                },
                p1: CompactScreenPoint {
                    x: p1.x as i16,
                    y: p1.y as i16,
                    z: p1.z,
                },
                p2: CompactScreenPoint {
                    x: p2.x as i16,
                    y: p2.y as i16,
                    z: p2.z,
                },
                dz_dx,
                long_edge_is_left,
                color,
                aabb_min_x: min_x as i16,
                aabb_min_y: min_y as i16,
                aabb_max_x: max_x as i16,
                aabb_max_y: max_y as i16,
                min_depth,
                max_depth,
            });
        }
        results
    }

    /// CPU binning path with optional Hi-Z occlusion culling
    fn bin_triangles_cpu(&mut self) {
        if self.use_two_level_binning {
            self.bin_triangles_two_level_cpu();
            return;
        }

        let prepared_len = self.prepared.len();
        for i in 0..prepared_len {
            // Occlusion test before binning (if Hi-Z is enabled)
            if let Some(ref hiz) = self.hiz_buffer {
                let tri = &self.prepared[i];
                let aabb = AABB3D {
                    min_x: i32::from(tri.aabb_min_x),
                    max_x: i32::from(tri.aabb_max_x),
                    min_y: i32::from(tri.aabb_min_y),
                    max_y: i32::from(tri.aabb_max_y),
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

    fn bin_triangles_two_level_cpu(&mut self) {
        let prepared_len = self.prepared.len();
        let coarse_size = 4; // 4x4 tiles = 128x128 pixels

        for i in 0..prepared_len {
            let tri = &self.prepared[i];

            // If Hi-Z is enabled, we can use it to cull coarse bins
            // First, check if the whole triangle is occluded (fast rejection)
            if let Some(ref hiz) = self.hiz_buffer {
                let aabb = AABB3D {
                    min_x: i32::from(tri.aabb_min_x),
                    max_x: i32::from(tri.aabb_max_x),
                    min_y: i32::from(tri.aabb_min_y),
                    max_y: i32::from(tri.aabb_max_y),
                    min_depth: tri.min_depth,
                    max_depth: tri.max_depth,
                };

                if !hiz.is_potentially_visible(aabb) {
                    continue;
                }
            }

            let tile_size_i32 = TILE_SIZE as i32;

            // Calculate triangle bounds in tile coordinates
            let tx_min_tri = (i32::from(tri.aabb_min_x) / tile_size_i32) as u32;
            let ty_min_tri = (i32::from(tri.aabb_min_y) / tile_size_i32) as u32;
            let tx_max_tri =
                ((i32::from(tri.aabb_max_x) / tile_size_i32) as u32).min(self.tiles_x - 1);
            let ty_max_tri =
                ((i32::from(tri.aabb_max_y) / tile_size_i32) as u32).min(self.tiles_y - 1);

            // Calculate bounds in coarse bin coordinates
            let cx_min = tx_min_tri / coarse_size;
            let cy_min = ty_min_tri / coarse_size;
            let cx_max = tx_max_tri / coarse_size;
            let cy_max = ty_max_tri / coarse_size;

            for cy in cy_min..=cy_max {
                for cx in cx_min..=cx_max {
                    // Check visibility of this coarse bin
                    let mut visible = true;
                    if let Some(ref hiz) = self.hiz_buffer {
                        let bin_min_x = (cx * coarse_size * TILE_SIZE) as i32;
                        let bin_min_y = (cy * coarse_size * TILE_SIZE) as i32;
                        let bin_max_x = bin_min_x + (coarse_size * TILE_SIZE) as i32 - 1;
                        let bin_max_y = bin_min_y + (coarse_size * TILE_SIZE) as i32 - 1;

                        // Clamp to screen
                        let bin_aabb = AABB3D {
                            min_x: bin_min_x.max(0),
                            max_x: bin_max_x.min(self.width as i32 - 1),
                            min_y: bin_min_y.max(0),
                            max_y: bin_max_y.min(self.height as i32 - 1),
                            min_depth: tri.min_depth,
                            max_depth: tri.max_depth,
                        };

                        if !hiz.is_potentially_visible(bin_aabb) {
                            visible = false;
                        }
                    }

                    if visible {
                        // Iterate over fine tiles within this coarse bin
                        let tx_start = (cx * coarse_size).max(tx_min_tri);
                        let ty_start = (cy * coarse_size).max(ty_min_tri);
                        let tx_end = ((cx + 1) * coarse_size - 1).min(tx_max_tri);
                        let ty_end = ((cy + 1) * coarse_size - 1).min(ty_max_tri);

                        for ty in ty_start..=ty_end {
                            for tx in tx_start..=tx_end {
                                let bin_idx = (ty * self.tiles_x + tx) as usize;
                                self.tile_bins.push(bin_idx, i);
                            }
                        }
                    }
                }
            }
        }
    }

    fn bin_triangle(&mut self, tri_idx: usize) {
        let tri = &self.prepared[tri_idx];
        let tile_size_i32 = TILE_SIZE as i32;

        let tx_min = (i32::from(tri.aabb_min_x) / tile_size_i32) as u32;
        let ty_min = (i32::from(tri.aabb_min_y) / tile_size_i32) as u32;
        let tx_max = ((i32::from(tri.aabb_max_x) / tile_size_i32) as u32).min(self.tiles_x - 1);
        let ty_max = ((i32::from(tri.aabb_max_y) / tile_size_i32) as u32).min(self.tiles_y - 1);

        for ty in ty_min..=ty_max {
            for tx in tx_min..=tx_max {
                let bin_idx = (ty * self.tiles_x + tx) as usize;
                self.tile_bins.push(bin_idx, tri_idx);
            }
        }
    }

    /// Sorts flat triangles in each bin by depth.
    fn sort_bins_flat(&mut self) {
        // FIXME: Sorting is temporarily disabled due to TileBins SoA refactor breaking the iterator.
        // Needs proper implementation for linked-list sorting or reverting to Vec<Vec>.
        /*
        let prepared = &self.prepared;
        if prepared.is_empty() {
            return;
        }

        // Helper to sort a single bin (linked list)
        let sort_bin = |head: &mut u32, nexts: &mut [u32], tris: &[u32]| {
            if *head == u32::MAX {
                return;
            }

            // 1. Collect indices into a temporary vector
            // We reuse a thread-local buffer to avoid allocations?
            // For now, just allocate a small vec. Most tiles have < 100 triangles.
            let mut indices = Vec::with_capacity(64);
            let mut curr = *head;
            while curr != u32::MAX {
                indices.push(curr);
                curr = nexts[curr as usize];
            }

            // 2. Sort indices by depth
            indices.sort_unstable_by(|&a, &b| {
                // tris[a] is the index into `prepared`
                let tri_idx_a = tris[a as usize] as usize;
                let tri_idx_b = tris[b as usize] as usize;
                // Safety: indices guaranteed within bounds
                let depth_a = unsafe { prepared.get_unchecked(tri_idx_a).min_depth };
                let depth_b = unsafe { prepared.get_unchecked(tri_idx_b).min_depth };
                depth_a
                    .partial_cmp(&depth_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // 3. Rebuild linked list
            *head = indices[0];
            let len = indices.len();
            for i in 0..len - 1 {
                nexts[indices[i] as usize] = indices[i + 1];
            }
            nexts[indices[len - 1] as usize] = u32::MAX;
            // Tail update is not strictly needed unless we append more, but TileBins struct has tails.
            // We should update tails if we want to support appending after sorting (unlikely).
            // But let's keep it consistent if possible.
            // Accessing self.tails here is hard due to borrow check if we are iterating.
            // We are iterating indices, so we can't easily update tails unless we pass tails slice.
        };

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            // We need to split the struct to mutate parts in parallel
            let heads = &mut self.tile_bins.heads;
            let nexts = &mut self.tile_bins.nexts;
            let tris = &self.tile_bins.tris; // Read-only

            // Rayon doesn't like splitting mutable slices with disjoint indices easily without unsafe.
            // Or we can just iterate over indices 0..heads.len()
            // But nexts is a single big vector. Parallel mutation of nexts is safe ONLY if disjoint.
            // Since each bin owns a disjoint set of nodes in the linked list, it IS disjoint.
            // But the borrow checker doesn't know that `nexts` indices are disjoint per bin.
            // We would need UnsafeCell or similar wrapper.

            // For now, let's use the sequential sort or a "safe" parallel approach if possible.
            // Since we can't easily prove disjointness to Rust, we might have to fallback to sequential
            // OR use a `par_chunks` on `heads` but we need mutable access to `nexts` everywhere.
            // Actually, sorting bins is important for performance but maybe not critical to parallelize
            // if the bin count is high and per-bin count is low.
            //
            // Let's implement sequential sort first to restore correctness.

            let mut tails = &mut self.tile_bins.tails;

            for (tile_idx, head) in heads.iter_mut().enumerate() {
                if *head == u32::MAX { continue; }

                // Collect
                let mut indices = Vec::with_capacity(64);
                let mut curr = *head;
                while curr != u32::MAX {
                    indices.push(curr);
                    curr = nexts[curr as usize];
                }

                // Sort
                indices.sort_unstable_by(|&a, &b| {
                    let tri_idx_a = tris[a as usize] as usize;
                    let tri_idx_b = tris[b as usize] as usize;
                    let depth_a = unsafe { prepared.get_unchecked(tri_idx_a).min_depth };
                    let depth_b = unsafe { prepared.get_unchecked(tri_idx_b).min_depth };
                    depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
                });

                // Rebuild
                *head = indices[0];
                let len = indices.len();
                for i in 0..len - 1 {
                    nexts[indices[i] as usize] = indices[i + 1];
                }
                nexts[indices[len - 1] as usize] = u32::MAX;
                tails[tile_idx] = indices[len - 1];
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            let heads = &mut self.tile_bins.heads;
            let nexts = &mut self.tile_bins.nexts;
            let tails = &mut self.tile_bins.tails;
            let tris = &self.tile_bins.tris;

            for (tile_idx, head) in heads.iter_mut().enumerate() {
                if *head == u32::MAX { continue; }

                let mut indices = Vec::with_capacity(64);
                let mut curr = *head;
                while curr != u32::MAX {
                    indices.push(curr);
                    curr = nexts[curr as usize];
                }

                indices.sort_unstable_by(|&a, &b| {
                    let tri_idx_a = tris[a as usize] as usize;
                    let tri_idx_b = tris[b as usize] as usize;
                    let depth_a = unsafe { prepared.get_unchecked(tri_idx_a).min_depth };
                    let depth_b = unsafe { prepared.get_unchecked(tri_idx_b).min_depth };
                    depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
                });

                *head = indices[0];
                let len = indices.len();
                for i in 0..len - 1 {
                    nexts[indices[i] as usize] = indices[i + 1];
                }
                nexts[indices[len - 1] as usize] = u32::MAX;
                tails[tile_idx] = indices[len - 1];
            }
        }
        */
    }

    /// Sorts textured triangles in each bin by depth.
    fn sort_bins_textured(&mut self) {
        // FIXME: Sorting is temporarily disabled due to TileBins SoA refactor breaking the iterator.
        /*
        let prepared_textured = &self.prepared_textured;
        if prepared_textured.is_empty() {
            return;
        }

        let heads = &mut self.tile_bins.heads;
        let nexts = &mut self.tile_bins.nexts;
        let tails = &mut self.tile_bins.tails;
        let tris = &self.tile_bins.tris;

        // Sequential implementation for both parallel/not parallel features for now
        // to avoid code duplication and safety issues with parallel linked list modification.
        for (tile_idx, head) in heads.iter_mut().enumerate() {
            if *head == u32::MAX { continue; }

            let mut indices = Vec::with_capacity(64);
            let mut curr = *head;
            while curr != u32::MAX {
                indices.push(curr);
                curr = nexts[curr as usize];
            }

            indices.sort_unstable_by(|&a, &b| {
                let tri_idx_a = tris[a as usize] as usize;
                let tri_idx_b = tris[b as usize] as usize;
                let depth_a = unsafe { prepared_textured.get_unchecked(tri_idx_a).min_depth };
                let depth_b = unsafe { prepared_textured.get_unchecked(tri_idx_b).min_depth };
                depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
            });

            *head = indices[0];
            let len = indices.len();
            for i in 0..len - 1 {
                nexts[indices[i] as usize] = indices[i + 1];
            }
            nexts[indices[len - 1] as usize] = u32::MAX;
            tails[tile_idx] = indices[len - 1];
        }
        */
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
/// - `false` if scanline rendering is recommended (use [`fill_triangle_3d`](super::fill_triangle_3d))
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::should_use_tiled_rendering;
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
pub const fn should_use_tiled_rendering(
    width: usize,
    height: usize,
    triangle_count: usize,
) -> bool {
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
        // With full frustum clipping, this triangle is clipped into a quad (2 triangles)
        assert!(
            !tr.prepared.is_empty(),
            "Partially visible triangle should NOT be culled (got {})",
            tr.prepared.len()
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
        let binned_count = tr
            .tile_bins
            .heads
            .iter()
            .filter(|&&h| h != u32::MAX)
            .count();
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

        let binned_count = tr
            .tile_bins
            .heads
            .iter()
            .filter(|&&h| h != u32::MAX)
            .count();
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
    fn verify_prepared_textured_triangle_size() {
        use std::mem::size_of;
        // Optimization: Ensure PreparedTexturedTriangle fits in exactly 128 bytes (2 cache lines)
        // 128 bytes = 3*16 (pts) + 3*4 (uv) + 7*4 (grads) + 1 (bool) + 3 (pad) + 4*4 (aabb) + 2*4 (depth) = 48+12+28+4+16+8 = 116 + padding?
        // Wait, alignment.
        // ScreenPoint (16 bytes, align 4).
        // Gradients (28 bytes, align 4).
        // It should definitely be <= 128.
        assert!(
            size_of::<PreparedTexturedTriangle>() <= 128,
            "Struct grew beyond 128 bytes!"
        );
        // We assert equality to catch if we can shrink it further or if it regresses.
        assert_eq!(size_of::<PreparedTexturedTriangle>(), 128);
    }

    #[test]
    fn verify_prepared_triangle_size() {
        use std::mem::size_of;
        // Optimization: PreparedTriangle should fit in 64 bytes (1 cache line).
        // Original size: 84 bytes (with ScreenPoint and i32 AABBs).
        // New size: ~52 bytes (with CompactScreenPoint and i16 AABBs).
        assert!(
            size_of::<PreparedTriangle>() <= 64,
            "PreparedTriangle should fit in a cache line"
        );
        println!("PreparedTriangle size: {}", size_of::<PreparedTriangle>());
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

/// Render a single tile gouraud: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_single_tile_gouraud(
    tx: u32,
    ty: u32,
    tile_bins: &TileBins,
    prepared: &[PreparedGouraudTriangle],
    tiles_x: u32,
    width: u32,
    _height: u32,
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins.heads[bin_idx] == u32::MAX {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;

    // Compute Y range covered by triangles in this bin (partial tile clear)
    let mut clear_y_min = tile_y1;
    let mut clear_y_max = tile_y0;

    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        clear_y_min = clear_y_min.min(i32::from(tri.aabb_min_y).max(tile_y0));
        clear_y_max = clear_y_max.max(i32::from(tri.aabb_max_y).min(tile_y1 - 1));
    }

    // Clear only the rows that will be touched
    let row_start = ((clear_y_min - tile_y0) as u32 * TILE_SIZE) as usize;
    let row_end = (((clear_y_max - tile_y0) as u32 + 1) * TILE_SIZE) as usize;
    tile_pixels[row_start..row_end].fill(0xFF00_0000);
    tile_depths[row_start..row_end].fill(f32::INFINITY);

    // Render all triangles in bin
    let screen_w = width as i32;
    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile_gouraud(
            tile_pixels,
            tile_depths,
            tri,
            tile_x0,
            tile_y0,
            tile_x1,
            tile_y1,
            screen_w,
        );
    }

    Some((clear_y_min, clear_y_max))
}

/// Render a gouraud triangle into tile-local buffers.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn render_triangle_in_tile_gouraud(
    tile_pixels: &mut [u32],
    tile_depths: &mut [f32],
    tri: &PreparedGouraudTriangle,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    screen_w: i32,
) {
    let p0_y = i32::from(tri.p0.y);
    let _p1_y = i32::from(tri.p1.y);
    let p2_y = i32::from(tri.p2.y);

    let y_start = p0_y.max(tile_y0);
    let y_end = p2_y.min(tile_y1 - 1);

    if y_start > y_end {
        return;
    }

    let screen_x_max = screen_w - 1;

    // Edge Walking
    let p0 = tri.p0.to_screen_point(1.0);
    let p1 = tri.p1.to_screen_point(1.0);
    let p2 = tri.p2.to_screen_point(1.0);

    // Convert fixed point colors back to Vec3 for EdgeWalker initialization
    // (This seems inefficient, maybe adapt EdgeWalker to take fixed point?)
    // GouraudEdgeWalker takes Vec3 for color to handle interpolation precisely?
    // Actually GouraudEdgeWalker::new takes Vec3 start/end.
    // And it computes gradients in fixed point internally.
    // But we already HAVE gradients in the PreparedGouraudTriangle.
    // We just need to step X, Z, and C.
    // Wait, PreparedGouraudTriangle has `gradients` which are `GouraudGradients` (dX only).
    // It does NOT have dY gradients for stepping edges.
    // `GouraudEdgeWalker` computes dY gradients.
    // So we need to reconstruct the full walker or adapt it.

    // Reconstruction from vertices:
    // We need Vec3 colors.
    let c0 = Vec3::new(
        tri.c0.0 as f32 / 65536.0,
        tri.c0.1 as f32 / 65536.0,
        tri.c0.2 as f32 / 65536.0,
    );
    let c1 = Vec3::new(
        tri.c1.0 as f32 / 65536.0,
        tri.c1.1 as f32 / 65536.0,
        tri.c1.2 as f32 / 65536.0,
    );
    let c2 = Vec3::new(
        tri.c2.0 as f32 / 65536.0,
        tri.c2.1 as f32 / 65536.0,
        tri.c2.2 as f32 / 65536.0,
    );

    let mut edge_a = GouraudEdgeWalker::new(p0, p2, c0, c2);
    if y_start > p0.y {
        edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
    }

    let mut edge_b = if y_start < p1.y {
        let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
        if y_start > p0.y {
            e.step_n(i64::from(y_start) - i64::from(p0.y));
        }
        e
    } else {
        let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
        if y_start > p1.y {
            e.step_n(i64::from(y_start) - i64::from(p1.y));
        }
        e
    };

    let dz_dx = tri.gradients.dz_dx;
    let dc_dx = tri.gradients.dc_dx;

    for y in y_start..=y_end {
        if y == p1.y && y != p0.y {
            edge_b = GouraudEdgeWalker::new(p1, p2, c1, c2);
        }

        let (x_start, x_end, z_left, c_left) = if tri.long_edge_is_left {
            (
                (edge_a.x >> 16) as i32,
                (edge_b.x >> 16) as i32,
                edge_a.z,
                edge_a.c,
            )
        } else {
            (
                (edge_b.x >> 16) as i32,
                (edge_a.x >> 16) as i32,
                edge_b.z,
                edge_b.c,
            )
        };

        let dx = i64::from(x_end) - i64::from(x_start);

        if dx <= 0 {
            // Single-pixel scanline
            if x_start >= tile_x0 && x_start < tile_x1 && x_start >= 0 && x_start <= screen_x_max {
                let tile_idx =
                    ((y - tile_y0) as u32 * TILE_SIZE + (x_start - tile_x0) as u32) as usize;
                if z_left < tile_depths[tile_idx] {
                    tile_depths[tile_idx] = z_left;
                    // c_left is (i32, i32, i32) fixed point.
                    // Need to pack to u32.
                    // Reuse pack_color_fixed_i32 from core/gouraud?
                    // core::pack_color_fixed_i32 is likely private or not exported to here.
                    // Let's implement inline packing.
                    let r = (c_left.0 >> 16).clamp(0, 255) as u32;
                    let g = (c_left.1 >> 16).clamp(0, 255) as u32;
                    let b = (c_left.2 >> 16).clamp(0, 255) as u32;
                    tile_pixels[tile_idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
                }
            }
        } else {
            // Clamp X to tile and screen bounds
            let xs = x_start.max(tile_x0).max(0);
            let xe = x_end.min(tile_x1 - 1).min(screen_x_max);

            if xs <= xe {
                // Calculate z and color at xs
                let dx_start = (i64::from(xs) - i64::from(x_start)) as f32;
                let z_at_xs = z_left + dx_start * dz_dx;

                let dx_start_i32 = (i64::from(xs) - i64::from(x_start)) as i32;
                let c_at_xs = (
                    c_left.0.wrapping_add(dc_dx.0.wrapping_mul(dx_start_i32)),
                    c_left.1.wrapping_add(dc_dx.1.wrapping_mul(dx_start_i32)),
                    c_left.2.wrapping_add(dc_dx.2.wrapping_mul(dx_start_i32)),
                );

                let row_offset = ((y - tile_y0) as u32 * TILE_SIZE) as usize;
                let col_start = (xs - tile_x0) as usize;
                let col_end = (xe - tile_x0) as usize;

                let pixels = &mut tile_pixels[row_offset + col_start..=row_offset + col_end];
                let depths = &mut tile_depths[row_offset + col_start..=row_offset + col_end];

                #[cfg(all(feature = "simd", target_arch = "x86_64"))]
                {
                    if pixels.len() >= 8 && is_x86_feature_detected!("avx2") {
                        unsafe {
                            draw_scanline_gouraud_simd_fast(
                                pixels, depths, z_at_xs, c_at_xs, dz_dx, dc_dx,
                            );
                        }
                    } else {
                        draw_scanline_gouraud_i32_tile(
                            pixels, depths, z_at_xs, c_at_xs, dz_dx, dc_dx,
                        );
                    }
                }
                #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
                draw_scanline_gouraud_i32_tile(pixels, depths, z_at_xs, c_at_xs, dz_dx, dc_dx);
            }
        }

        edge_a.step();
        edge_b.step();
    }
}

/// Helper for drawing gouraud scanline into a slice (no bounds checking needed)
#[inline(always)]
fn draw_scanline_gouraud_i32_tile(
    pixels: &mut [u32],
    depths: &mut [f32],
    z_start: f32,
    c_start: (i32, i32, i32),
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    let mut z = z_start;
    let mut r = c_start.0;
    let mut g = c_start.1;
    let mut b = c_start.2;
    let dr = dc_dx.0;
    let dg = dc_dx.1;
    let db = dc_dx.2;

    for (pixel, depth_val) in pixels.iter_mut().zip(depths.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;
            let rv = (r >> 16).clamp(0, 255) as u32;
            let gv = (g >> 16).clamp(0, 255) as u32;
            let bv = (b >> 16).clamp(0, 255) as u32;
            *pixel = 0xFF000000 | (rv << 16) | (gv << 8) | bv;
        }
        z += dz_dx;
        r = r.wrapping_add(dr);
        g = g.wrapping_add(dg);
        b = b.wrapping_add(db);
    }
}
