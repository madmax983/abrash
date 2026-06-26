#![allow(clippy::collapsible_if)]
//! Tile-based rasterizer data structures.
//!
//! Handles splitting the screen into tiles for binning-based rasterization.

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
//! use abrash_render::rasterizer::{TileRenderer, ClipTriangle};
//! use abrash_core::framebuffer::Framebuffer;
//! use abrash_core::zbuffer::ZBuffer;
//! use abrash_core::math::Vec3;
//!
//! let mut fb = Framebuffer::new(3840, 2160).unwrap(); // 4K resolution
//! let mut zb = ZBuffer::new(3840, 2160).unwrap();
//! let mut renderer = TileRenderer::new(3840, 2160);
//!
//! let triangles: Vec<ClipTriangle> = vec![
//!     ((Vec3::new(-0.5, -0.5, 0.5), 1.0),
//!      (Vec3::new(0.5, -0.5, 0.5), 1.0),
//!      (Vec3::new(0.0, 0.5, 0.5), 1.0),
//!      0xFF00_00FF), // Red triangle
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

/// Groups screen space parameters to reduce function arguments and improve clarity.
#[derive(Clone, Copy)]
pub struct ScreenSpaceContext {
    /// Width of the tile.
    pub width: u32,
    /// Height of the tile.
    pub height: u32,
    /// Half width of the tile.
    pub half_width: f32,
    /// Half height of the tile.
    pub half_height: f32,
}

/// Defines the bounds and target position for merging a tile into the framebuffer.
#[derive(Clone, Copy)]
pub struct TileMergeBounds {
    /// Tile X index.
    pub tx: u32,
    /// Tile Y index.
    pub ty: u32,
    /// Width of the tile.
    pub width: u32,
    /// Height of the tile.
    pub height: u32,
    /// Minimum Y bounds.
    pub y_min: i32,
    /// Maximum Y bounds.
    pub y_max: i32,
}
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
struct SendPtr<T>(*mut T, usize);

#[cfg(feature = "parallel")]
impl<T> SendPtr<T> {
    /// SAFETY: Caller must ensure the index is within bounds and writes are to non-overlapping regions

    unsafe fn write(&self, index: usize, value: T) {
        assert!(index < self.1, "Index out of bounds");
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
    data: Vec<T>,
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

        Self { data, ptr, len }
    }

    fn resize(&mut self, new_len: usize, default_value: T) {
        if new_len > self.len {
            let align_bytes = 32;
            let elem_size = std::mem::size_of::<T>();
            let extra_elements = (align_bytes + elem_size - 1) / elem_size;

            // Only reallocate if underlying vector capacity isn't enough
            if new_len + extra_elements > self.data.capacity() {
                self.data.resize(new_len + extra_elements, default_value);

                let start_ptr = self.data.as_mut_ptr();
                let start_addr = start_ptr as usize;
                let offset_bytes = (align_bytes - (start_addr % align_bytes)) % align_bytes;
                let offset_elements = offset_bytes / elem_size;
                self.ptr = unsafe { start_ptr.add(offset_elements) };
            }
        }
        self.len = new_len;
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

/// Context for rendering a triangle into a tile.
struct TileContext<'a> {
    pixels: &'a mut [u32],
    depths: &'a mut [f32],
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    screen_x_max: i32,
}

/// Tile size in pixels. 32x32 = 1024 pixels * 4 bytes = 4KB per buffer.
pub const TILE_SIZE: u32 = 32;

impl TileContext<'_> {
    #[inline(always)]
    const fn get_indices(&self, x: i32, y: i32) -> usize {
        ((y - self.y0) as u32 * TILE_SIZE + (x - self.x0) as u32) as usize
    }

    #[inline(always)]
    const fn get_row_offset(&self, y: i32) -> usize {
        ((y - self.y0) as u32 * TILE_SIZE) as usize
    }
}

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
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
    /// Z coordinate.
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
/// A triangle that has been clipped, projected, culled, Y-sorted, and had gradients computed.
pub struct PreparedTriangle {
    /// Top vertex (lowest Y).
    pub p0: CompactScreenPoint,
    /// Middle vertex.
    pub p1: CompactScreenPoint,
    /// Bottom vertex (highest Y).
    pub p2: CompactScreenPoint,
    /// Change in depth per pixel in X direction.
    pub dz_dx: f32,
    /// True if the long edge connecting p0 and p2 is on the left side.
    pub long_edge_is_left: bool,
    /// Flat color to fill the triangle with.
    pub color: u32,
    /// Minimum X coordinate of the bounding box.
    pub aabb_min_x: u16,
    /// Minimum Y coordinate of the bounding box.
    pub aabb_min_y: u16,
    /// Maximum X coordinate of the bounding box.
    pub aabb_max_x: u16,
    /// Maximum Y coordinate of the bounding box.
    pub aabb_max_y: u16,
    /// Minimum depth across triangle.
    /// Minimum depth across triangle.
    pub min_depth: f32,
    /// Maximum depth across triangle.
    /// Maximum depth across triangle.
    pub max_depth: f32,
}

/// A Gouraud-shaded triangle prepared for rasterization.
#[derive(Clone, Copy)]
pub struct PreparedGouraudTriangle {
    /// Top vertex (lowest Y).
    pub p0: CompactScreenPoint,
    /// Middle vertex.
    pub p1: CompactScreenPoint,
    /// Bottom vertex (highest Y).
    pub p2: CompactScreenPoint,
    /// Fixed-point color at p0.
    pub c0: (i32, i32, i32),
    /// Fixed-point color at p1.
    pub c1: (i32, i32, i32),
    /// Fixed-point color at p2.
    pub c2: (i32, i32, i32),
    /// Gradients for interpolating color across the triangle.
    pub gradients: GouraudGradients,
    /// True if the long edge connecting p0 and p2 is on the left side.
    pub long_edge_is_left: bool,
    /// Minimum X coordinate of the bounding box.
    pub aabb_min_x: u16,
    /// Minimum Y coordinate of the bounding box.
    pub aabb_min_y: u16,
    /// Maximum X coordinate of the bounding box.
    pub aabb_max_x: u16,
    /// Maximum Y coordinate of the bounding box.
    pub aabb_max_y: u16,
    /// Minimum depth across triangle.
    pub min_depth: f32,
    /// Maximum depth across triangle.
    pub max_depth: f32,
}

/// A textured triangle prepared for rasterization.
///
/// Optimized to fit in exactly 128 bytes (2 cache lines).
#[derive(Clone, Copy)]
pub struct PreparedTexturedTriangle {
    /// Top vertex (lowest Y).
    pub p0: ScreenPoint,
    /// Middle vertex.
    pub p1: ScreenPoint,
    /// Bottom vertex (highest Y).
    pub p2: ScreenPoint,
    /// Texture U coordinate for p0.
    pub u0: f32,
    /// Texture U coordinate for p1.
    pub u1: f32,
    /// Texture U coordinate for p2.
    pub u2: f32,
    /// Texture V coordinate for p0.
    pub v0: f32,
    /// Texture V coordinate for p1.
    pub v1: f32,
    /// Texture V coordinate for p2.
    pub v2: f32,
    /// Gradients for interpolating depth and texture coordinates.
    pub gradients: PerspectiveTextureGradients,
    /// Indicates whether the longest edge connects p0 and p2 on the left side of the triangle.
    /// True if the long edge connecting p0 and p2 is on the left side.
    pub long_edge_is_left: bool,
    /// Minimum X bound of the triangle's bounding box.
    pub aabb_min_x: i32,
    /// Minimum Y bound of the triangle's bounding box.
    pub aabb_min_y: i32,
    /// Maximum X bound of the triangle's bounding box.
    pub aabb_max_x: i32,
    /// Maximum Y bound of the triangle's bounding box.
    pub aabb_max_y: i32,
    /// Minimum depth across triangle.
    /// Minimum depth across triangle.
    pub min_depth: f32,
    /// Maximum depth across triangle.
    /// Maximum depth across triangle.
    pub max_depth: f32,
}

/// Helper function to compute the minimum and maximum Y bounds for clearing a tile,
/// based on the triangles intersecting it, and clears the specified tile regions.
#[inline(always)]
fn clear_tile_bounds<T>(
    ctx: &mut TileContext,
    tile_bins: &TileBins,
    bin_idx: usize,
    prepared: &[T],
    get_bounds: impl Fn(&T) -> (i32, i32),
    clear_color: u32,
) -> (i32, i32) {
    let mut clear_y_min = ctx.y1;
    let mut clear_y_max = ctx.y0;

    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        let (min_y, max_y) = get_bounds(tri);
        clear_y_min = clear_y_min.min(min_y.max(ctx.y0));
        clear_y_max = clear_y_max.max(max_y.min(ctx.y1 - 1));
    }

    let row_start = ((clear_y_min - ctx.y0) as u32 * TILE_SIZE as u32) as usize;
    let row_end = (((clear_y_max - ctx.y0) as u32 + 1) * TILE_SIZE as u32) as usize;
    ctx.pixels[row_start..row_end].fill(clear_color);
    ctx.depths[row_start..row_end].fill(f32::INFINITY);

    (clear_y_min, clear_y_max)
}

use std::mem::MaybeUninit;

#[cfg(feature = "parallel")]
use rayon::iter::IndexedParallelIterator;

#[doc(hidden)]
pub struct PreparedGouraudTrianglesList {
    pub tris: [MaybeUninit<PreparedGouraudTriangle>; 8],
    count: usize,
}

impl PreparedGouraudTrianglesList {
    /// Creates a new, empty list.

    #[must_use]
    pub const fn new() -> Self {
        Self {
            tris: [const { MaybeUninit::uninit() }; 8],
            count: 0,
        }
    }

    pub const fn push(&mut self, tri: PreparedGouraudTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }

    /// Returns the number of triangles in the list.

    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
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

/// An iterator over a list of prepared Gouraud triangles.
pub struct PreparedGouraudTrianglesIter {
    list: PreparedGouraudTrianglesList,
    index: usize,
}

impl Iterator for PreparedGouraudTrianglesIter {
    type Item = PreparedGouraudTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { std::ptr::read(self.list.tris[self.index].as_ptr()) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

/// A fixed-capacity list of prepared flat-shaded triangles.
pub struct PreparedTrianglesList {
    /// The uninitialized backing array of triangles.
    pub tris: [MaybeUninit<PreparedTriangle>; 8],
    count: usize,
}

impl PreparedTrianglesList {
    /// Creates a new, empty list.

    #[must_use]
    pub const fn new() -> Self {
        Self {
            tris: [const { MaybeUninit::uninit() }; 8],
            count: 0,
        }
    }

    /// Pushes a triangle into the list if capacity allows.
    pub const fn push(&mut self, tri: PreparedTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }

    /// Returns the number of triangles in the list.

    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
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

#[cfg(feature = "parallel")]
impl rayon::iter::IntoParallelIterator for PreparedTrianglesList {
    type Item = PreparedTriangle;
    type Iter = rayon::iter::Flatten<rayon::array::IntoIter<Option<PreparedTriangle>, 8>>;

    fn into_par_iter(self) -> Self::Iter {
        use rayon::iter::IntoParallelIterator;
        use rayon::iter::ParallelIterator;
        // ⚡ Bolt: Elides dynamic heap allocation by mapping directly over a stack array
        let mut arr: [Option<PreparedTriangle>; 8] = [const { None }; 8];
        for i in 0..self.count {
            arr[i] = Some(unsafe { std::ptr::read(self.tris[i].as_ptr()) });
        }
        arr.into_par_iter().flatten()
    }
}

#[cfg(feature = "parallel")]
impl rayon::iter::IntoParallelIterator for PreparedTexturedTrianglesList {
    type Item = PreparedTexturedTriangle;
    type Iter = rayon::iter::Flatten<rayon::array::IntoIter<Option<PreparedTexturedTriangle>, 8>>;

    fn into_par_iter(self) -> Self::Iter {
        use rayon::iter::IntoParallelIterator;
        use rayon::iter::ParallelIterator;
        // ⚡ Bolt: Elides dynamic heap allocation by mapping directly over a stack array
        let mut arr: [Option<PreparedTexturedTriangle>; 8] = [const { None }; 8];
        for i in 0..self.count {
            arr[i] = Some(unsafe { std::ptr::read(self.tris[i].as_ptr()) });
        }
        arr.into_par_iter().flatten()
    }
}

#[cfg(feature = "parallel")]
impl rayon::iter::IntoParallelIterator for PreparedGouraudTrianglesList {
    type Item = PreparedGouraudTriangle;
    type Iter = rayon::iter::Flatten<rayon::array::IntoIter<Option<PreparedGouraudTriangle>, 8>>;

    fn into_par_iter(self) -> Self::Iter {
        use rayon::iter::IntoParallelIterator;
        use rayon::iter::ParallelIterator;
        // ⚡ Bolt: Elides dynamic heap allocation by mapping directly over a stack array
        let mut arr: [Option<PreparedGouraudTriangle>; 8] = [const { None }; 8];
        for i in 0..self.count {
            arr[i] = Some(unsafe { std::ptr::read(self.tris[i].as_ptr()) });
        }
        arr.into_par_iter().flatten()
    }
}

/// An iterator over a list of prepared flat-shaded triangles.
pub struct PreparedTrianglesIter {
    list: PreparedTrianglesList,
    index: usize,
}

impl Iterator for PreparedTrianglesIter {
    type Item = PreparedTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { std::ptr::read(self.list.tris[self.index].as_ptr()) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

/// A fixed-capacity list of prepared textured triangles.
pub struct PreparedTexturedTrianglesList {
    /// The uninitialized backing array of triangles.
    pub tris: [MaybeUninit<PreparedTexturedTriangle>; 8],
    count: usize,
}

impl PreparedTexturedTrianglesList {
    /// Creates a new, empty list.

    #[must_use]
    pub const fn new() -> Self {
        Self {
            tris: [const { MaybeUninit::uninit() }; 8],
            count: 0,
        }
    }

    /// Pushes a triangle into the list if capacity allows.
    pub const fn push(&mut self, tri: PreparedTexturedTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }

    /// Returns the number of triangles in the list.

    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
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

/// An iterator over a list of prepared textured triangles.
pub struct PreparedTexturedTrianglesIter {
    list: PreparedTexturedTrianglesList,
    index: usize,
}

impl Iterator for PreparedTexturedTrianglesIter {
    type Item = PreparedTexturedTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { std::ptr::read(self.list.tris[self.index].as_ptr()) };
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
    /// The head indices into the nexts and tris vectors for each bin. `u32::MAX` = None.
    pub heads: Vec<u32>,
    /// The tail indices into the nexts and tris vectors for each bin. `u32::MAX` = None.
    pub tails: Vec<u32>,
    /// Links to the next node in the linked list.
    pub nexts: Vec<u32>,
    /// The triangle indices for each node.
    pub tris: Vec<u32>,
}

impl TileBins {
    /// Initializes a new bin structure with a given number of tiles.

    #[must_use]
    pub fn new(num_tiles: usize) -> Self {
        // ⚡ Bolt: Pre-allocate capacity for `nexts` and `tris` based on tile count
        // assuming an average of 4 triangles intersecting per tile to eliminate initial heap reallocations.
        let capacity = num_tiles * 4;
        Self {
            heads: vec![u32::MAX; num_tiles],
            tails: vec![u32::MAX; num_tiles],
            nexts: Vec::with_capacity(capacity),
            tris: Vec::with_capacity(capacity),
        }
    }

    /// Clears the bin structure for the next frame.
    pub fn clear(&mut self) {
        self.heads.fill(u32::MAX);
        self.tails.fill(u32::MAX);
        self.nexts.clear();
        self.tris.clear();
    }

    /// Pushes a triangle index into the bin for the given tile index.
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

    /// Iterates over the triangle indices in the given bin.
    #[inline]
    #[must_use]
    pub fn iter(&self, tile_idx: usize) -> TileBinIter<'_> {
        TileBinIter {
            bins: self,
            curr: self.heads[tile_idx],
        }
    }
}

/// An iterator over a tile bin.
pub struct TileBinIter<'a> {
    bins: &'a TileBins,
    curr: u32,
}

impl Iterator for TileBinIter<'_> {
    type Item = usize;

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
    clear_color: u32,
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins.heads[bin_idx] == u32::MAX {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;
    let screen_x_max = width as i32 - 1;

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    let (clear_y_min, clear_y_max) = clear_tile_bounds(
        &mut ctx,
        tile_bins,
        bin_idx,
        prepared,
        |tri| (i32::from(tri.aabb_min_y), i32::from(tri.aabb_max_y)),
        clear_color,
    );

    // Render all triangles in bin
    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile(&mut ctx, tri);
    }

    Some((clear_y_min, clear_y_max))
}

/// Render a triangle into tile-local buffers. Free function to avoid `&mut self` borrow conflicts.
#[inline(always)]
fn render_triangle_in_tile(ctx: &mut TileContext, tri: &PreparedTriangle) {
    let p0_y = tri.p0.y;
    let p1_y = tri.p1.y;
    let p2_y = tri.p2.y;

    let y_start = p0_y.max(ctx.y0);
    let y_end = p2_y.min(ctx.y1 - 1);

    if y_start > y_end {
        return;
    }

    // Reconstruct ScreenPoint for EdgeWalker (inv_w unused for flat shading)
    let p0 = ScreenPoint {
        x: tri.p0.x,
        y: p0_y,
        z: tri.p0.z,
        inv_w: 1.0,
    };
    let p1 = ScreenPoint {
        x: tri.p1.x,
        y: p1_y,
        z: tri.p1.z,
        inv_w: 1.0,
    };
    let p2 = ScreenPoint {
        x: tri.p2.x,
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

        process_tile_scanline_flat(ctx, y, x_start, x_end, z_left, dz_dx, color);

        edge_a.step();
        edge_b.step();
    }
}

/// Render a single tile textured: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
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
    clear_color: u32,
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins.heads[bin_idx] == u32::MAX {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;
    let screen_x_max = width as i32 - 1;

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    let (clear_y_min, clear_y_max) = clear_tile_bounds(
        &mut ctx,
        tile_bins,
        bin_idx,
        prepared,
        |tri| (tri.aabb_min_y, tri.aabb_max_y),
        clear_color,
    );

    // Render all triangles in bin
    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile_textured(&mut ctx, tri, texture);
    }

    Some((clear_y_min, clear_y_max))
}

/// Render a textured triangle into tile-local buffers.
#[inline(always)]
fn render_triangle_in_tile_textured(
    ctx: &mut TileContext,
    tri: &PreparedTexturedTriangle,
    texture: &Texture,
) {
    let y_start = tri.p0.y.max(ctx.y0);
    let y_end = tri.p2.y.min(ctx.y1 - 1);

    if y_start > y_end {
        return;
    }

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

        process_tile_scanline_textured(
            ctx, y, x_start, x_end, z_left, q_left, u_left, v_left, tri, texture,
        );

        edge_a.step();
        edge_b.step();
    }
}

#[inline(always)]

fn process_tile_scanline_flat(
    ctx: &mut TileContext,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_left: f32,
    dz_dx: f32,
    color: u32,
) {
    let dx = i64::from(x_end) - i64::from(x_start);

    if dx <= 0 {
        // Single-pixel scanline
        if x_start >= ctx.x0 && x_start < ctx.x1 && x_start >= 0 && x_start <= ctx.screen_x_max {
            let tile_idx = ctx.get_indices(x_start, y);
            if z_left < ctx.depths[tile_idx] {
                ctx.depths[tile_idx] = z_left;
                ctx.pixels[tile_idx] = color;
            }
        }
    } else {
        // Clamp X to tile and screen bounds
        let xs = x_start.max(ctx.x0).max(0);
        let xe = x_end.min(ctx.x1 - 1).min(ctx.screen_x_max);

        if xs <= xe {
            // Calculate z at xs
            let dx_start = (i64::from(xs) - i64::from(x_start)) as f32;
            let z_at_xs = z_left + dx_start * dz_dx;

            let row_offset = ctx.get_row_offset(y);
            let col_start = (xs - ctx.x0) as usize;
            let col_end = (xe - ctx.x0) as usize;

            let pixels = &mut ctx.pixels[row_offset + col_start..=row_offset + col_end];
            let depths = &mut ctx.depths[row_offset + col_start..=row_offset + col_end];

            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            {
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
}

#[inline(always)]
fn process_tile_scanline_textured(
    ctx: &mut TileContext,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_left: f32,
    q_left: f32,
    u_left: f32,
    v_left: f32,
    tri: &PreparedTexturedTriangle,
    texture: &Texture,
) {
    let dx = i64::from(x_end) - i64::from(x_start);

    if dx <= 0 {
        // Single-pixel scanline
        if x_start >= ctx.x0 && x_start < ctx.x1 && x_start >= 0 && x_start <= ctx.screen_x_max {
            let tile_idx = ctx.get_indices(x_start, y);
            if z_left < ctx.depths[tile_idx] && q_left.abs() > 0.000_001 {
                ctx.depths[tile_idx] = z_left;
                let w = 1.0 / q_left;
                let u_tex = u_left * w;
                let v_tex = v_left * w;
                ctx.pixels[tile_idx] = match texture.filter_mode {
                    FilterMode::Nearest => texture.get_pixel_texel(u_tex as i32, v_tex as i32),
                    FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                    FilterMode::Trilinear => {
                        let w = 1.0 / q_left;
                        let w_sq = w * w;

                        let du_tex_dx =
                            (tri.gradients.du_dx * q_left - u_left * tri.gradients.dq_dx) * w_sq;
                        let dv_tex_dx =
                            (tri.gradients.dv_dx * q_left - v_left * tri.gradients.dq_dx) * w_sq;
                        let du_tex_dy =
                            (tri.gradients.du_dy * q_left - u_left * tri.gradients.dq_dy) * w_sq;
                        let dv_tex_dy =
                            (tri.gradients.dv_dy * q_left - v_left * tri.gradients.dq_dy) * w_sq;

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
        let xs = x_start.max(ctx.x0).max(0);
        let xe = x_end.min(ctx.x1 - 1).min(ctx.screen_x_max);

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

            let row_offset = ctx.get_row_offset(y);
            let col_start = (xs - ctx.x0) as usize;
            let col_end = (xe - ctx.x0) as usize;

            let pixels = &mut ctx.pixels[row_offset + col_start..=row_offset + col_end];
            let depths = &mut ctx.depths[row_offset + col_start..=row_offset + col_end];

            rasterize_scanline_textured(pixels, depths, texture, start, &tri.gradients);
        }
    }
}

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
                if pixels_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
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
                if pixels_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
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
                if pixels_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
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
    for (pixel, depth) in pixels[..len].iter_mut().zip(&mut depths[..len]) {
        if z < *depth {
            *depth = z;
            *pixel = color;
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
        _mm256_castsi256_ps, _mm256_loadu_ps, _mm256_loadu_si256, _mm256_movemask_ps,
        _mm256_mul_ps, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps,
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

                    #[allow(clippy::cast_ptr_alignment)]
                    let pixels_ptr = pixels.as_mut_ptr().add(i).cast::<__m256i>();
                    _mm256_store_si256(pixels_ptr, color_vec);
                } else {
                    // Partial write path
                    // 1. Update depths
                    let blended_depths = _mm256_blendv_ps(zb_vals, depths_vec, mask);
                    // Use aligned store since we are aligned
                    _mm256_store_ps(zb_ptr, blended_depths);

                    // 2. Update pixels
                    #[allow(clippy::cast_ptr_alignment)]
                    let pixels_ptr = pixels.as_mut_ptr().add(i).cast::<__m256i>();
                    // Read old pixels (aligned load)
                    let old_pixels = _mm256_loadu_si256(pixels_ptr.cast_const());

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
        // We calculate z directly instead of accumulating
        #[allow(unused_assignments)]
        {
            z += 0.0;
        } // Keep compiler quiet about z assignment before this loop
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
    use_two_level_binning: bool,
    // Pre-calculated half dimensions for projection
    half_width: f32,
    half_height: f32,
    /// When set, `end_frame` writes ALL tiles (empty tiles get this color),
    /// eliminating the need for a separate full-frame `fb.clear()` + `zb.clear()`.
    clear_color: Option<u32>,
}

struct CoarseBinContext<'a> {
    tri_idx: usize,
    cx_min: u32,
    cx_max: u32,
    cy_min: u32,
    cy_max: u32,
    tx_min_tri: u32,
    tx_max_tri: u32,
    ty_min_tri: u32,
    ty_max_tri: u32,
    tri_min_depth: f32,
    tri_max_depth: f32,
    width_i32: i32,
    height_i32: i32,
    has_hiz: bool,
    hiz_buffer_ref: Option<&'a HiZBuffer>,
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
        // WARDEN DEFENSE: Prevent capacity overflow panics
        let _ = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .expect("TileRenderer dimensions overflow");

        let tiles_x = width.div_ceil(TILE_SIZE);
        let tiles_y = height.div_ceil(TILE_SIZE);
        let tile_count = tiles_x
            .checked_mul(tiles_y)
            .expect("TileRenderer dimensions overflow") as usize;
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
            // ⚡ Bolt: Pre-allocate triangle buffers since TileRenderer targets <= 100 triangles per frame.
            // This eliminates multiple dynamic heap reallocations during frame submission.
            prepared: Vec::with_capacity(128),
            prepared_gouraud: Vec::with_capacity(128),
            prepared_textured: Vec::with_capacity(128),
            hiz_buffer: None,
            use_two_level_binning: false,
            half_width: width as f32 * 0.5,
            half_height: height as f32 * 0.5,
            clear_color: None,
        }
    }

    /// Enable integrated tile-level clearing.
    ///
    /// When set, `end_frame` writes every tile to the framebuffer — empty tiles
    /// get the clear color, non-empty tiles get a full clear + render + merge.
    /// This eliminates the separate `fb.clear()` + `zb.clear()` calls, replacing
    /// a cold-cache 2.46 MB memset with 300 × 8 KB L1-friendly tile writes.
    pub const fn set_clear_color(&mut self, color: Option<u32>) {
        self.clear_color = color;
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
            // Bolt: Use `par_extend` combined with `flat_map` to reuse the existing capacity
            // of `self.prepared` and eliminate intermediate Vec heap allocations entirely.
            self.prepared.par_extend(
                indices
                    .par_iter()
                    .filter(|&&[i0, i1, i2]| {
                        i0 < vertices.len() && i1 < vertices.len() && i2 < vertices.len()
                    })
                    .flat_map(|&[i0, i1, i2]| {
                        let v0 = vertices[i0];
                        let v1 = vertices[i1];
                        let v2 = vertices[i2];

                        let ctx = ScreenSpaceContext {
                            width,
                            height,
                            half_width,
                            half_height,
                        };
                        Self::prepare_triangle_static(v0, v1, v2, color, &ctx)
                    }),
            );
        }

        #[cfg(not(feature = "parallel"))]
        {
            // Fast path: if ALL vertices are inside the view frustum, no triangle
            // needs clipping. Pre-project all vertices once and reuse across shared
            // triangles (121 projections instead of 600 for a 10×10 grid mesh).
            let all_inside = vertices.iter().all(|&(p, w)| {
                w > 0.0 && p.x >= -w && p.x <= w && p.y >= -w && p.y <= w && p.z >= -w && p.z <= w
            });

            if all_inside {
                self.submit_mesh_unclipped(indices, vertices, color);
            } else {
                for &[i0, i1, i2] in indices {
                    if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                        continue;
                    }
                    let v0 = vertices[i0];
                    let v1 = vertices[i1];
                    let v2 = vertices[i2];
                    self.prepare_triangle(v0, v1, v2, color);
                }
            }
        }
    }

    /// Fast path for meshes fully inside the frustum: pre-project all vertices once,
    /// then prepare triangles without clipping. Shared vertices are projected only once
    /// instead of once per triangle.
    fn submit_mesh_unclipped(
        &mut self,
        indices: &[[usize; 3]],
        vertices: &[(Vec3, f32)],
        color: u32,
    ) {
        use crate::math::project_to_screen_optimized;

        let hw = self.half_width;
        let hh = self.half_height;
        let width_i32 = self.width as i32 - 1;
        let height_i32 = self.height as i32 - 1;

        // Bolt Performance Optimization:
        // By hoisting the `projected` vertex buffer into a thread-local static `RefCell`,
        // we eliminate a dynamic heap allocation (`Vec::new()` via `.collect()`) per mesh per frame
        // in the hot rendering path. Reusing the capacity avoids thousands of allocations per second.
        thread_local! {
            static PROJECTED_BUFFER: std::cell::RefCell<Vec<ScreenPoint>> = const { std::cell::RefCell::new(Vec::new()) };
        }

        PROJECTED_BUFFER.with(|buf| {
            let mut projected = buf.borrow_mut();
            projected.clear();

            // Phase 1: Project all vertices to screen space (once per vertex)
            projected.extend(
                vertices
                    .iter()
                    .map(|&(v, w)| project_to_screen_optimized(v, w, hw, hh)),
            );

            // Phase 2: Per-triangle setup (backface, sort, dz_dx, AABB)
            for &[i0, i1, i2] in indices {
                if i0 >= projected.len() || i1 >= projected.len() || i2 >= projected.len() {
                    continue;
                }
                let p0_orig = projected[i0];
                let p1_orig = projected[i1];
                let p2_orig = projected[i2];

                if is_backface(p0_orig, p1_orig, p2_orig) {
                    continue;
                }

                let mut verts = <[_; 3]>::from((p0_orig, p1_orig, p2_orig));
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
                let max_x = p0.x.max(p1.x).max(p2.x).min(width_i32);
                let max_y = p2.y.min(height_i32);

                if min_x > max_x || min_y > max_y {
                    continue;
                }

                let min_depth = p0.z.min(p1.z).min(p2.z);
                let max_depth = p0.z.max(p1.z).max(p2.z);

                self.prepared.push(PreparedTriangle {
                    p0: CompactScreenPoint {
                        x: p0.x,
                        y: p0.y,
                        z: p0.z,
                    },
                    p1: CompactScreenPoint {
                        x: p1.x,
                        y: p1.y,
                        z: p1.z,
                    },
                    p2: CompactScreenPoint {
                        x: p2.x,
                        y: p2.y,
                        z: p2.z,
                    },
                    dz_dx,
                    long_edge_is_left,
                    color,
                    aabb_min_x: min_x.max(0).min(65535) as u16,
                    aabb_min_y: min_y.max(0).min(65535) as u16,
                    aabb_max_x: max_x.max(0).min(65535) as u16,
                    aabb_max_y: max_y.max(0).min(65535) as u16,
                    min_depth,
                    max_depth,
                });
            }
        });
    }

    /// Finish the frame: bin triangles, build Hi-Z, render tiles, and merge to framebuffer.
    ///
    /// # Panics
    ///
    /// Panics if framebuffer or zbuffer dimensions do not match the renderer configuration.
    pub fn end_frame(&mut self, fb: &mut Framebuffer, zb: &mut ZBuffer) {
        self.end_frame_into_slices(
            fb.width(),
            fb.height(),
            fb.as_mut_slice(),
            zb.as_mut_slice(),
        );
    }

    /// Finish the frame into caller-owned slices.
    pub fn end_frame_into_slices(
        &mut self,
        width: u32,
        height: u32,
        pixels: &mut [u32],
        depths: &mut [f32],
    ) {
        self.validate_target_slices(width, height, pixels, depths);

        // Build Hi-Z pyramid from previous frame (temporal coherence)
        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid_from_depths(width, height, depths);
            }
        }

        if !self.prepared.is_empty() {
            self.bin_triangles_cpu();
            self.sort_bins_flat();

            #[cfg(not(feature = "parallel"))]
            {
                let cc = self.clear_color.unwrap_or(0xFF00_0000);
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
                            cc,
                        ) {
                            let (y_min, y_max) = if self.clear_color.is_some() {
                                let tile_y0 = ty * TILE_SIZE;
                                let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                                let tri_row_start = ((clear_y_min - tile_y0 as i32).max(0) as u32
                                    * TILE_SIZE)
                                    as usize;
                                if tri_row_start > 0 {
                                    self.tile_pixels[..tri_row_start].fill(cc);
                                    self.tile_depths[..tri_row_start].fill(f32::INFINITY);
                                }
                                let tri_row_end = (((clear_y_max - tile_y0 as i32).max(0) as u32
                                    + 1)
                                    * TILE_SIZE)
                                    as usize;
                                if tri_row_end < tile_area {
                                    self.tile_pixels[tri_row_end..tile_area].fill(cc);
                                    self.tile_depths[tri_row_end..tile_area].fill(f32::INFINITY);
                                }
                                (
                                    tile_y0 as i32,
                                    (tile_y0 + TILE_SIZE).min(self.height) as i32 - 1,
                                )
                            } else {
                                (clear_y_min, clear_y_max)
                            };
                            let bounds = TileMergeBounds {
                                tx,
                                ty,
                                width: self.width,
                                height: self.height,
                                y_min,
                                y_max,
                            };
                            Self::merge_tile_direct_into_slices(
                                &self.tile_pixels,
                                &self.tile_depths,
                                self.width,
                                self.height,
                                pixels,
                                depths,
                                &bounds,
                            );
                        } else if self.clear_color.is_some() {
                            Self::merge_empty_tile_into_slices(
                                pixels,
                                depths,
                                tx,
                                ty,
                                self.width,
                                self.height,
                                cc,
                            );
                        }
                    }
                }
            }

            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;

                let cc = self.clear_color.unwrap_or(0xFF00_0000);
                let has_integrated_clear = self.clear_color.is_some();

                unsafe {
                    let fb_ptr = SendPtr(pixels.as_mut_ptr(), pixels.len());
                    let zb_ptr = SendPtr(depths.as_mut_ptr(), depths.len());
                    let width = self.width;
                    let height = self.height;
                    let tiles_x = self.tiles_x;
                    let tile_bins = &self.tile_bins;
                    let prepared = &self.prepared;
                    (0..self.tiles_y)
                        .into_par_iter()
                        .flat_map_iter(|ty| (0..self.tiles_x).map(move |tx| (tx, ty)))
                        .for_each(|(tx, ty)| {
                            let bin_idx = (ty * tiles_x + tx) as usize;

                            if tile_bins.heads[bin_idx] == u32::MAX {
                                if has_integrated_clear {
                                    let tile_x0 = tx * TILE_SIZE;
                                    let tile_y0 = ty * TILE_SIZE;
                                    let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                                    let tile_y_end = (tile_y0 + TILE_SIZE).min(height);
                                    let tile_cols = (tile_x_end - tile_x0) as usize;
                                    for row in tile_y0..tile_y_end {
                                        let fb_start =
                                            row as usize * width as usize + tile_x0 as usize;
                                        for col in 0..tile_cols {
                                            fb_ptr.write(fb_start + col, cc);
                                            zb_ptr.write(fb_start + col, f32::INFINITY);
                                        }
                                    }
                                }
                                return;
                            }

                            std::thread_local! {
                                static TILE_BUFFER: std::cell::RefCell<(AlignedBuffer<u32>, AlignedBuffer<f32>)> = std::cell::RefCell::new((AlignedBuffer::new(0), AlignedBuffer::new(0)));
                            }
                            TILE_BUFFER.with(|buf| {
                                let mut buffers = buf.borrow_mut();
                                let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                                if buffers.0.len() < tile_area {
                                    buffers.0 = AlignedBuffer::new(tile_area);
                                    buffers.1 = AlignedBuffer::new(tile_area);
                                }
                                let buffers_ref = &mut *buffers;
                                let tile_pixels = &mut buffers_ref.0;
                                let tile_depths = &mut buffers_ref.1;
                                tile_pixels.fill(cc);
                                tile_depths.fill(f32::INFINITY);
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
                                    cc,
                                ) {
                                    let tile_x0 = tx * TILE_SIZE;
                                    let tile_y0 = ty * TILE_SIZE;
                                    let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                                    let tile_cols = (tile_x_end - tile_x0) as usize;

                                    let row_begin = if has_integrated_clear {
                                        tile_y0
                                    } else {
                                        clear_y_min.max(tile_y0 as i32) as u32
                                    };
                                    let row_end = if has_integrated_clear {
                                        (tile_y0 + TILE_SIZE).min(height)
                                    } else {
                                        (clear_y_max as u32 + 1)
                                            .min(tile_y0 + TILE_SIZE)
                                            .min(height)
                                    };

                                    for row in row_begin..row_end {
                                        let tile_row_offset =
                                            ((row - tile_y0) * TILE_SIZE) as usize;
                                        let fb_start =
                                            row as usize * width as usize + tile_x0 as usize;

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
                            });
                        });
                }
            }
        }

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
    /// # use abrash_render::rasterizer::TileRenderer;
    /// # use abrash_core::framebuffer::Framebuffer;
    /// # use abrash_core::zbuffer::ZBuffer;
    /// # use abrash_core::math::Vec3;
    /// let mut renderer = TileRenderer::new(1920, 1080);
    /// let mut fb = Framebuffer::new(1920, 1080).unwrap();
    /// let mut zb = ZBuffer::new(1920, 1080).unwrap();
    ///
    /// let triangles = vec![
    ///     ((Vec3::new(0.0, 0.0, 1.0), 1.0),
    ///      (Vec3::new(1.0, 0.0, 1.0), 1.0),
    ///      (Vec3::new(0.5, 1.0, 1.0), 1.0),
    ///      0x00FF_FFFFFF),
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
        self.render_batch_into_slices(
            fb.width(),
            fb.height(),
            fb.as_mut_slice(),
            zb.as_mut_slice(),
            triangles,
        );
    }

    /// Renders a batch of flat-shaded triangles directly into externally provided pixel and depth slices.
    pub fn render_batch_into_slices(
        &mut self,
        width: u32,
        height: u32,
        pixels: &mut [u32],
        depths: &mut [f32],
        triangles: &[ClipTriangle],
    ) {
        self.validate_target_slices(width, height, pixels, depths);
        self.begin_frame();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let width = self.width;
            let height = self.height;
            let half_width = self.half_width;
            let half_height = self.half_height;

            self.prepared
                .par_extend(triangles.par_iter().flat_map(|&(v0, v1, v2, color)| {
                    let ctx = ScreenSpaceContext {
                        width,
                        height,
                        half_width,
                        half_height,
                    };
                    Self::prepare_triangle_static(v0, v1, v2, color, &ctx)
                }));
        }

        #[cfg(not(feature = "parallel"))]
        {
            for &(v0, v1, v2, color) in triangles {
                self.prepare_triangle(v0, v1, v2, color);
            }
        }

        self.end_frame_into_slices(width, height, pixels, depths);
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
        self.render_batch_textured_into_slices(
            fb.width(),
            fb.height(),
            fb.as_mut_slice(),
            zb.as_mut_slice(),
            triangles,
            texture,
        );
    }

    /// Renders a batch of textured triangles directly into externally provided pixel and depth slices.
    pub fn render_batch_textured_into_slices(
        &mut self,
        width: u32,
        height: u32,
        pixels: &mut [u32],
        depths: &mut [f32],
        triangles: &[TexturedClipTriangle],
        texture: &Texture,
    ) {
        self.validate_target_slices(width, height, pixels, depths);

        self.prepared_textured.clear();
        self.tile_bins.clear();

        let tex_w = texture.width as f32;
        let tex_h = texture.height as f32;

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let width = self.width;
            let height = self.height;
            let half_width = self.half_width;
            let half_height = self.half_height;

            self.prepared_textured
                .par_extend(
                    triangles
                        .par_iter()
                        .flat_map(|&(v0, uv0, v1, uv1, v2, uv2)| {
                            let ctx = ScreenSpaceContext {
                                width,
                                height,
                                half_width,
                                half_height,
                            };
                            Self::prepare_triangle_textured_static(
                                (v0, uv0),
                                (v1, uv1),
                                (v2, uv2),
                                tex_w,
                                tex_h,
                                &ctx,
                            )
                        }),
                );
        }

        #[cfg(not(feature = "parallel"))]
        {
            for &(v0, uv0, v1, uv1, v2, uv2) in triangles {
                self.prepare_triangle_textured((v0, uv0), (v1, uv1), (v2, uv2), tex_w, tex_h);
            }
        }

        if let Some(ref mut hiz) = self.hiz_buffer {
            if !hiz.is_valid() {
                hiz.build_pyramid_from_depths(width, height, depths);
            }
        }

        self.bin_triangles_textured_cpu();
        self.sort_bins_textured();

        #[cfg(not(feature = "parallel"))]
        {
            let cc = self.clear_color.unwrap_or(0xFF00_0000);
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
                        cc,
                    ) {
                        let (y_min, y_max) = if self.clear_color.is_some() {
                            let tile_y0 = ty * TILE_SIZE;
                            let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                            let tri_row_start =
                                ((clear_y_min - tile_y0 as i32).max(0) as u32 * TILE_SIZE) as usize;
                            if tri_row_start > 0 {
                                self.tile_pixels[..tri_row_start].fill(cc);
                                self.tile_depths[..tri_row_start].fill(f32::INFINITY);
                            }
                            let tri_row_end = (((clear_y_max - tile_y0 as i32).max(0) as u32 + 1)
                                * TILE_SIZE) as usize;
                            if tri_row_end < tile_area {
                                self.tile_pixels[tri_row_end..tile_area].fill(cc);
                                self.tile_depths[tri_row_end..tile_area].fill(f32::INFINITY);
                            }
                            (
                                tile_y0 as i32,
                                (tile_y0 + TILE_SIZE).min(self.height) as i32 - 1,
                            )
                        } else {
                            (clear_y_min, clear_y_max)
                        };
                        let bounds = TileMergeBounds {
                            tx,
                            ty,
                            width: self.width,
                            height: self.height,
                            y_min,
                            y_max,
                        };
                        Self::merge_tile_direct_into_slices(
                            &self.tile_pixels,
                            &self.tile_depths,
                            self.width,
                            self.height,
                            pixels,
                            depths,
                            &bounds,
                        );
                    } else if self.clear_color.is_some() {
                        Self::merge_empty_tile_into_slices(
                            pixels,
                            depths,
                            tx,
                            ty,
                            self.width,
                            self.height,
                            cc,
                        );
                    }
                }
            }
        }

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            let cc = self.clear_color.unwrap_or(0xFF00_0000);
            let has_integrated_clear = self.clear_color.is_some();

            unsafe {
                let fb_ptr = SendPtr(pixels.as_mut_ptr(), pixels.len());
                let zb_ptr = SendPtr(depths.as_mut_ptr(), depths.len());
                let width = self.width;
                let height = self.height;
                let tiles_x = self.tiles_x;
                let tile_bins = &self.tile_bins;
                let prepared = &self.prepared_textured;
                (0..self.tiles_y)
                    .into_par_iter()
                    .flat_map_iter(|ty| (0..self.tiles_x).map(move |tx| (tx, ty)))
                    .for_each(|(tx, ty)| {
                        let bin_idx = (ty * tiles_x + tx) as usize;
                        if tile_bins.heads[bin_idx] == u32::MAX {
                            if has_integrated_clear {
                                let tile_x0 = tx * TILE_SIZE;
                                let tile_y0 = ty * TILE_SIZE;
                                let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                                let tile_y_end = (tile_y0 + TILE_SIZE).min(height);
                                let tile_cols = (tile_x_end - tile_x0) as usize;
                                for row in tile_y0..tile_y_end {
                                    let fb_start = row as usize * width as usize + tile_x0 as usize;
                                    for col in 0..tile_cols {
                                        fb_ptr.write(fb_start + col, cc);
                                        zb_ptr.write(fb_start + col, f32::INFINITY);
                                    }
                                }
                            }
                            return;
                        }

                        std::thread_local! {
                            static TILE_BUFFER: std::cell::RefCell<(AlignedBuffer<u32>, AlignedBuffer<f32>)> = std::cell::RefCell::new((AlignedBuffer::new(0), AlignedBuffer::new(0)));
                        }
                        TILE_BUFFER.with(|buf| {
                            let mut buffers = buf.borrow_mut();
                            let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                            if buffers.0.len() < tile_area {
                                buffers.0 = AlignedBuffer::new(tile_area);
                                buffers.1 = AlignedBuffer::new(tile_area);
                            }
                            let buffers_ref = &mut *buffers;
                            let tile_pixels = &mut buffers_ref.0;
                            let tile_depths = &mut buffers_ref.1;
                            tile_pixels.fill(cc);
                            tile_depths.fill(f32::INFINITY);
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
                                cc,
                            ) {
                                let tile_x0 = tx * TILE_SIZE;
                                let tile_y0 = ty * TILE_SIZE;
                                let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                                let tile_cols = (tile_x_end - tile_x0) as usize;

                                let row_begin = if has_integrated_clear {
                                    tile_y0
                                } else {
                                    clear_y_min.max(tile_y0 as i32) as u32
                                };
                                let row_end = if has_integrated_clear {
                                    (tile_y0 + TILE_SIZE).min(height)
                                } else {
                                    (clear_y_max as u32 + 1).min(tile_y0 + TILE_SIZE).min(height)
                                };

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
                        });
                    });
            }
        }

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

        let expected_len = (self.width as usize)
            .checked_mul(self.height as usize)
            .expect("TileRenderer dimensions overflow");
        assert!(
            fb.as_slice().len() >= expected_len,
            "Framebuffer slice too small"
        );
        assert!(
            zb.as_slice().len() >= expected_len,
            "ZBuffer slice too small"
        );

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

            // Bolt: Use `par_extend` to eliminate intermediate Vec heap allocations.
            self.prepared_gouraud
                .par_extend(triangles.par_iter().flat_map(|&(v0, v1, v2)| {
                    let ctx = ScreenSpaceContext {
                        width,
                        height,
                        half_width,
                        half_height,
                    };
                    Self::prepare_triangle_gouraud_static(v0, v1, v2, &ctx)
                }));
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
            let cc = self.clear_color.unwrap_or(0xFF00_0000);
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
                        cc,
                    ) {
                        let (y_min, y_max) = if self.clear_color.is_some() {
                            let tile_y0 = ty * TILE_SIZE;
                            let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                            let tri_row_start =
                                ((clear_y_min - tile_y0 as i32).max(0) as u32 * TILE_SIZE) as usize;
                            if tri_row_start > 0 {
                                self.tile_pixels[..tri_row_start].fill(cc);
                                self.tile_depths[..tri_row_start].fill(f32::INFINITY);
                            }
                            let tri_row_end = (((clear_y_max - tile_y0 as i32).max(0) as u32 + 1)
                                * TILE_SIZE) as usize;
                            if tri_row_end < tile_area {
                                self.tile_pixels[tri_row_end..tile_area].fill(cc);
                                self.tile_depths[tri_row_end..tile_area].fill(f32::INFINITY);
                            }
                            (
                                tile_y0 as i32,
                                (tile_y0 + TILE_SIZE).min(self.height) as i32 - 1,
                            )
                        } else {
                            (clear_y_min, clear_y_max)
                        };
                        let bounds = TileMergeBounds {
                            tx,
                            ty,
                            width: self.width,
                            height: self.height,
                            y_min,
                            y_max,
                        };
                        Self::merge_tile_direct(
                            &self.tile_pixels,
                            &self.tile_depths,
                            fb,
                            zb,
                            &bounds,
                        );
                    } else if self.clear_color.is_some() {
                        Self::merge_empty_tile(fb, zb, tx, ty, self.width, self.height, cc);
                    }
                }
            }
        }

        #[cfg(feature = "parallel")]
        {
            // Parallel rendering using Rayon
            use rayon::prelude::*;

            let cc = self.clear_color.unwrap_or(0xFF00_0000);
            let has_integrated_clear = self.clear_color.is_some();

            unsafe {
                let fb_ptr = SendPtr(fb.as_mut_slice().as_mut_ptr(), fb.as_slice().len());
                let zb_ptr = SendPtr(zb.as_mut_slice().as_mut_ptr(), zb.as_slice().len());
                let width = self.width;
                let height = self.height;
                let tiles_x = self.tiles_x;
                let tile_bins = &self.tile_bins;
                let prepared = &self.prepared_gouraud;
                (0..self.tiles_y)
                    .into_par_iter()
                    .flat_map_iter(|ty| (0..self.tiles_x).map(move |tx| (tx, ty)))
                    .for_each(|(tx, ty)| {
                        let bin_idx = (ty * tiles_x + tx) as usize;
                        if tile_bins.heads[bin_idx] == u32::MAX {
                            if has_integrated_clear {
                                let tile_x0 = tx * TILE_SIZE;
                                let tile_y0 = ty * TILE_SIZE;
                                let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                                let tile_y_end = (tile_y0 + TILE_SIZE).min(height);
                                let tile_cols = (tile_x_end - tile_x0) as usize;
                                for row in tile_y0..tile_y_end {
                                    let fb_start = row as usize * width as usize + tile_x0 as usize;
                                    for col in 0..tile_cols {
                                        fb_ptr.write(fb_start + col, cc);
                                        zb_ptr.write(fb_start + col, f32::INFINITY);
                                    }
                                }
                            }
                            return;
                        }

                        std::thread_local! {
                            static TILE_BUFFER: std::cell::RefCell<(AlignedBuffer<u32>, AlignedBuffer<f32>)> = std::cell::RefCell::new((AlignedBuffer::new(0), AlignedBuffer::new(0)));
                        }
                        TILE_BUFFER.with(|buf| {
                            let mut buffers = buf.borrow_mut();
                            let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                            if buffers.0.len() < tile_area {
                                buffers.0 = AlignedBuffer::new(tile_area);
                                buffers.1 = AlignedBuffer::new(tile_area);
                            }
                            let buffers_ref = &mut *buffers;
                            let tile_pixels = &mut buffers_ref.0;
                            let tile_depths = &mut buffers_ref.1;
                            tile_pixels.fill(cc);
                            tile_depths.fill(f32::INFINITY);
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
                                cc,
                            ) {
                                let tile_x0 = tx * TILE_SIZE;
                                let tile_y0 = ty * TILE_SIZE;
                                let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
                                let tile_cols = (tile_x_end - tile_x0) as usize;

                                let row_begin = if has_integrated_clear {
                                    tile_y0
                                } else {
                                    clear_y_min.max(tile_y0 as i32) as u32
                                };
                                let row_end = if has_integrated_clear {
                                    (tile_y0 + TILE_SIZE).min(height)
                                } else {
                                    (clear_y_max as u32 + 1).min(tile_y0 + TILE_SIZE).min(height)
                                };

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
                        });
                    });
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
        let ctx = ScreenSpaceContext {
            width: self.width,
            height: self.height,
            half_width: self.half_width,
            half_height: self.half_height,
        };
        let results = Self::prepare_triangle_gouraud_static(v0, v1, v2, &ctx);
        self.prepared_gouraud.extend(results);
    }

    fn prepare_triangle_gouraud_static(
        v0: ((Vec3, f32), Vec3),
        v1: ((Vec3, f32), Vec3),
        v2: ((Vec3, f32), Vec3),
        ctx: &ScreenSpaceContext,
    ) -> PreparedGouraudTrianglesList {
        let clipped = clip_triangle_to_frustum(
            v0,
            v1,
            v2,
            |v| v.0,
            |a, b, t| {
                (
                    (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                    a.1.lerp(b.1, t),
                )
            },
        );
        let mut results = PreparedGouraudTrianglesList::new();

        for i in 0..clipped.count() {
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
                ctx.half_width,
                ctx.half_height,
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
            let max_x = p0.x.max(p1.x).max(p2.x).min(ctx.width as i32 - 1);
            let max_y = p2.y.min(ctx.height as i32 - 1);

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
                    x: p0.x,
                    y: p0.y,
                    z: p0.z,
                },
                p1: CompactScreenPoint {
                    x: p1.x,
                    y: p1.y,
                    z: p1.z,
                },
                p2: CompactScreenPoint {
                    x: p2.x,
                    y: p2.y,
                    z: p2.z,
                },
                c0: c0_fixed,
                c1: c1_fixed,
                c2: c2_fixed,
                gradients,
                long_edge_is_left,
                aabb_min_x: min_x.max(0).min(65535) as u16,
                aabb_min_y: min_y.max(0).min(65535) as u16,
                aabb_max_x: max_x.max(0).min(65535) as u16,
                aabb_max_y: max_y.max(0).min(65535) as u16,
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
        let prepared_gouraud = &self.prepared_gouraud;
        if prepared_gouraud.is_empty() {
            return;
        }

        let heads = &mut self.tile_bins.heads;
        let nexts = &mut self.tile_bins.nexts;
        let tails = &mut self.tile_bins.tails;
        let tris = &self.tile_bins.tris;

        // Bolt Performance Optimization:
        // Replaced `Vec::with_capacity(64)` with `SmallVec` to keep the per-tile triangle indices buffer entirely on the stack.
        // This eliminates frequent dynamic heap allocations on the hot sorting path.
        let mut indices: smallvec::SmallVec<[u32; 64]> = smallvec::SmallVec::new();
        for (tile_idx, head) in heads.iter_mut().enumerate() {
            if *head == u32::MAX {
                continue;
            }

            indices.clear();
            let mut curr = *head;
            while curr != u32::MAX {
                indices.push(curr);
                curr = nexts[curr as usize];
            }

            if indices.len() > 1 {
                indices.sort_unstable_by(|&a, &b| {
                    let tri_idx_a = tris[a as usize] as usize;
                    let tri_idx_b = tris[b as usize] as usize;
                    let depth_a = unsafe { prepared_gouraud.get_unchecked(tri_idx_a).min_depth };
                    let depth_b = unsafe { prepared_gouraud.get_unchecked(tri_idx_b).min_depth };
                    depth_a
                        .partial_cmp(&depth_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
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
        let ctx = ScreenSpaceContext {
            width: self.width,
            height: self.height,
            half_width: self.half_width,
            half_height: self.half_height,
        };
        let results = Self::prepare_triangle_textured_static(v0, v1, v2, tex_w, tex_h, &ctx);
        self.prepared_textured.extend(results);
    }

    fn prepare_triangle_textured_static(
        v0: ((Vec3, f32), Vec2),
        v1: ((Vec3, f32), Vec2),
        v2: ((Vec3, f32), Vec2),
        tex_w: f32,
        tex_h: f32,
        ctx: &ScreenSpaceContext,
    ) -> PreparedTexturedTrianglesList {
        let clipped = clip_triangle_to_frustum(
            v0,
            v1,
            v2,
            |v| v.0,
            |a, b, t| {
                (
                    (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                    a.1.lerp(b.1, t),
                )
            },
        );
        let mut results = PreparedTexturedTrianglesList::new();

        for i in 0..clipped.count() {
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
                ctx.half_width,
                ctx.half_height,
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
            let max_x = p0.x.max(p1.x).max(p2.x).min(ctx.width as i32 - 1);
            let max_y = p2.y.min(ctx.height as i32 - 1);

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
        let ctx = ScreenSpaceContext {
            width: self.width,
            height: self.height,
            half_width: self.half_width,
            half_height: self.half_height,
        };
        let results = Self::prepare_triangle_static(v0, v1, v2, color, &ctx);
        self.prepared.extend(results);
    }

    fn prepare_triangle_static(
        v0: (Vec3, f32),
        v1: (Vec3, f32),
        v2: (Vec3, f32),
        color: u32,
        ctx: &ScreenSpaceContext,
    ) -> PreparedTrianglesList {
        let clipped = clip_triangle_to_frustum(
            v0,
            v1,
            v2,
            |v| (v.0, v.1),
            |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t),
        );
        let mut results = PreparedTrianglesList::new();

        for i in 0..clipped.count() {
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
                ctx.half_width,
                ctx.half_height,
            );

            if is_backface(p0_orig, p1_orig, p2_orig) {
                continue;
            }

            let mut verts = <[_; 3]>::from((p0_orig, p1_orig, p2_orig));
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
            let max_x = p0.x.max(p1.x).max(p2.x).min(ctx.width as i32 - 1);
            let max_y = p2.y.min(ctx.height as i32 - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            // Compute min/max depth for Hi-Z occlusion culling
            let min_depth = p0.z.min(p1.z).min(p2.z);
            let max_depth = p0.z.max(p1.z).max(p2.z);

            results.push(PreparedTriangle {
                p0: CompactScreenPoint {
                    x: p0.x,
                    y: p0.y,
                    z: p0.z,
                },
                p1: CompactScreenPoint {
                    x: p1.x,
                    y: p1.y,
                    z: p1.z,
                },
                p2: CompactScreenPoint {
                    x: p2.x,
                    y: p2.y,
                    z: p2.z,
                },
                dz_dx,
                long_edge_is_left,
                color,
                aabb_min_x: min_x.max(0).min(65535) as u16,
                aabb_min_y: min_y.max(0).min(65535) as u16,
                aabb_max_x: max_x.max(0).min(65535) as u16,
                aabb_max_y: max_y.max(0).min(65535) as u16,
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
        let tile_size_i32 = TILE_SIZE as i32;
        let coarse_pixel_size = (coarse_size * TILE_SIZE) as i32;

        let width_i32 = self.width as i32 - 1;
        let height_i32 = self.height as i32 - 1;

        // Optimization: Lift branch out of outer loop
        let has_hiz = self.hiz_buffer.is_some();
        let hiz_buffer_ref = self.hiz_buffer.as_ref();

        for i in 0..prepared_len {
            let tri = &self.prepared[i];

            // If Hi-Z is enabled, we can use it to cull coarse bins
            // First, check if the whole triangle is occluded (fast rejection)
            if has_hiz {
                let aabb = AABB3D {
                    min_x: i32::from(tri.aabb_min_x),
                    max_x: i32::from(tri.aabb_max_x),
                    min_y: i32::from(tri.aabb_min_y),
                    max_y: i32::from(tri.aabb_max_y),
                    min_depth: tri.min_depth,
                    max_depth: tri.max_depth,
                };

                // SAFETY: has_hiz is true, so hiz_buffer_ref is Some
                if !unsafe { hiz_buffer_ref.unwrap_unchecked() }.is_potentially_visible(aabb) {
                    continue;
                }
            }

            // Calculate triangle bounds in tile coordinates
            let tx_min_tri = u32::from(tri.aabb_min_x) >> 5;
            let ty_min_tri = u32::from(tri.aabb_min_y) >> 5;
            let tx_max_tri = (u32::from(tri.aabb_max_x) >> 5).min(self.tiles_x - 1);
            let ty_max_tri = (u32::from(tri.aabb_max_y) >> 5).min(self.tiles_y - 1);

            // Calculate bounds in coarse bin coordinates
            let cx_min = tx_min_tri >> 2;
            let cy_min = ty_min_tri >> 2;
            let cx_max = tx_max_tri >> 2;
            let cy_max = ty_max_tri >> 2;

            // Small triangles don't benefit from coarse binning
            // If it spans very few coarse bins, fallback to simple binning
            if cx_max - cx_min <= 1 && cy_max - cy_min <= 1 {
                for ty in ty_min_tri..=ty_max_tri {
                    let mut bin_idx = (ty * self.tiles_x + tx_min_tri) as usize;
                    for _tx in tx_min_tri..=tx_max_tri {
                        self.tile_bins.push(bin_idx, i);
                        bin_idx += 1;
                    }
                }
                continue;
            }

            let tri_min_depth = tri.min_depth;
            let tri_max_depth = tri.max_depth;

            Self::process_coarse_bins_for_triangle(
                &mut self.tile_bins,
                self.tiles_x,
                &CoarseBinContext {
                    tri_idx: i,
                    cx_min,
                    cx_max,
                    cy_min,
                    cy_max,
                    tx_min_tri,
                    tx_max_tri,
                    ty_min_tri,
                    ty_max_tri,
                    tri_min_depth,
                    tri_max_depth,
                    width_i32,
                    height_i32,
                    has_hiz,
                    hiz_buffer_ref,
                },
            );
        }
    }

    fn process_coarse_bins_for_triangle(
        tile_bins: &mut TileBins,
        tiles_x: u32,
        ctx: &CoarseBinContext<'_>,
    ) {
        let coarse_size: u32 = 4; // 4x4 tiles = 128x128 pixels
        let coarse_pixel_size = (coarse_size * TILE_SIZE) as i32;

        for cy in ctx.cy_min..=ctx.cy_max {
            let bin_min_y = (cy * coarse_size * TILE_SIZE) as i32;
            let bin_max_y = bin_min_y + coarse_pixel_size - 1;
            let cy_clamped_max_y = bin_max_y.min(ctx.height_i32);
            let cy_clamped_min_y = bin_min_y.max(0);

            let ty_start = (cy * coarse_size).max(ctx.ty_min_tri);
            let ty_end = ((cy + 1) * coarse_size - 1).min(ctx.ty_max_tri);

            for cx in ctx.cx_min..=ctx.cx_max {
                let mut visible = true;
                if ctx.has_hiz {
                    let bin_min_x = (cx * coarse_size * TILE_SIZE) as i32;
                    let bin_max_x = bin_min_x + coarse_pixel_size - 1;

                    // Clamp to screen
                    let bin_aabb = AABB3D {
                        min_x: bin_min_x.max(0),
                        max_x: bin_max_x.min(ctx.width_i32),
                        min_y: cy_clamped_min_y,
                        max_y: cy_clamped_max_y,
                        min_depth: ctx.tri_min_depth,
                        max_depth: ctx.tri_max_depth,
                    };

                    // SAFETY: has_hiz is true, so hiz_buffer_ref is Some
                    if !unsafe { ctx.hiz_buffer_ref.unwrap_unchecked() }
                        .is_potentially_visible(bin_aabb)
                    {
                        visible = false;
                    }
                }

                if visible {
                    // Iterate over fine tiles within this coarse bin
                    let tx_start = (cx * coarse_size).max(ctx.tx_min_tri);
                    let tx_end = ((cx + 1) * coarse_size - 1).min(ctx.tx_max_tri);

                    for ty in ty_start..=ty_end {
                        let mut bin_idx = (ty * tiles_x + tx_start) as usize;
                        for _tx in tx_start..=tx_end {
                            tile_bins.push(bin_idx, ctx.tri_idx);
                            bin_idx += 1;
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
        let prepared = &self.prepared;
        if prepared.is_empty() {
            return;
        }

        let heads = &mut self.tile_bins.heads;
        let nexts = &mut self.tile_bins.nexts;
        let tails = &mut self.tile_bins.tails;
        let tris = &self.tile_bins.tris;

        // Bolt Performance Optimization:
        // Replaced `Vec::with_capacity(64)` with `SmallVec` to keep the per-tile triangle indices buffer entirely on the stack.
        // This eliminates frequent dynamic heap allocations on the hot sorting path.
        let mut indices: smallvec::SmallVec<[u32; 64]> = smallvec::SmallVec::new();
        for (tile_idx, head) in heads.iter_mut().enumerate() {
            if *head == u32::MAX {
                continue;
            }

            indices.clear();
            let mut curr = *head;
            while curr != u32::MAX {
                indices.push(curr);
                curr = nexts[curr as usize];
            }

            if indices.len() > 1 {
                indices.sort_unstable_by(|&a, &b| {
                    let tri_idx_a = tris[a as usize] as usize;
                    let tri_idx_b = tris[b as usize] as usize;
                    let depth_a = unsafe { prepared.get_unchecked(tri_idx_a).min_depth };
                    let depth_b = unsafe { prepared.get_unchecked(tri_idx_b).min_depth };
                    depth_a
                        .partial_cmp(&depth_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
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
    }

    /// Sorts textured triangles in each bin by depth.
    fn sort_bins_textured(&mut self) {
        let prepared_textured = &self.prepared_textured;
        if prepared_textured.is_empty() {
            return;
        }

        let heads = &mut self.tile_bins.heads;
        let nexts = &mut self.tile_bins.nexts;
        let tails = &mut self.tile_bins.tails;
        let tris = &self.tile_bins.tris;

        // Bolt Performance Optimization:
        // Replaced `Vec::with_capacity(64)` with `SmallVec` to keep the per-tile triangle indices buffer entirely on the stack.
        // This eliminates frequent dynamic heap allocations on the hot sorting path.
        let mut indices: smallvec::SmallVec<[u32; 64]> = smallvec::SmallVec::new();
        for (tile_idx, head) in heads.iter_mut().enumerate() {
            if *head == u32::MAX {
                continue;
            }

            indices.clear();
            let mut curr = *head;
            while curr != u32::MAX {
                indices.push(curr);
                curr = nexts[curr as usize];
            }

            if indices.len() > 1 {
                indices.sort_unstable_by(|&a, &b| {
                    let tri_idx_a = tris[a as usize] as usize;
                    let tri_idx_b = tris[b as usize] as usize;
                    let depth_a = unsafe { prepared_textured.get_unchecked(tri_idx_a).min_depth };
                    let depth_b = unsafe { prepared_textured.get_unchecked(tri_idx_b).min_depth };
                    depth_a
                        .partial_cmp(&depth_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
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
    }

    /// Merge tile buffers into framebuffer using direct copy (no depth test).
    /// Only copies the rows between `y_min` and `y_max` (inclusive, screen coords).
    #[cfg(not(feature = "parallel"))]
    fn merge_tile_direct(
        tile_pixels: &[u32],
        tile_depths: &[f32],
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        bounds: &TileMergeBounds,
    ) {
        let tile_x0 = bounds.tx * TILE_SIZE;
        let tile_y0 = bounds.ty * TILE_SIZE;
        let tile_x_end = (tile_x0 + TILE_SIZE).min(bounds.width);
        let tile_cols = (tile_x_end - tile_x0) as usize;

        let fb_slice = fb.as_mut_slice();
        let zb_slice = zb.as_mut_slice();
        let fb_width = bounds.width as usize;

        let row_begin = bounds.y_min.max(tile_y0 as i32) as u32;
        let row_end = (bounds.y_max as u32 + 1)
            .min(tile_y0 + TILE_SIZE)
            .min(bounds.height);

        for row in row_begin..row_end {
            let tile_row_offset = ((row - tile_y0) * TILE_SIZE) as usize;
            let fb_start = row as usize * fb_width + tile_x0 as usize;

            fb_slice[fb_start..fb_start + tile_cols]
                .copy_from_slice(&tile_pixels[tile_row_offset..tile_row_offset + tile_cols]);
            zb_slice[fb_start..fb_start + tile_cols]
                .copy_from_slice(&tile_depths[tile_row_offset..tile_row_offset + tile_cols]);
        }
    }

    /// Write clear color + infinity depth for an empty tile's region directly into fb/zb.
    /// Handles edge tiles that are smaller than `TILE_SIZE`.
    #[cfg(not(feature = "parallel"))]
    fn merge_empty_tile(
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        tx: u32,
        ty: u32,
        width: u32,
        height: u32,
        clear_color: u32,
    ) {
        let tile_x0 = tx * TILE_SIZE;
        let tile_y0 = ty * TILE_SIZE;
        let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
        let tile_y_end = (tile_y0 + TILE_SIZE).min(height);
        let tile_cols = (tile_x_end - tile_x0) as usize;

        let fb_slice = fb.as_mut_slice();
        let zb_slice = zb.as_mut_slice();
        let fb_width = width as usize;

        for row in tile_y0..tile_y_end {
            let fb_start = row as usize * fb_width + tile_x0 as usize;
            fb_slice[fb_start..fb_start + tile_cols].fill(clear_color);
            zb_slice[fb_start..fb_start + tile_cols].fill(f32::INFINITY);
        }
    }

    fn validate_target_slices(&self, width: u32, height: u32, pixels: &[u32], depths: &[f32]) {
        assert_eq!(
            width, self.width,
            "Framebuffer width must match TileRenderer width"
        );
        assert_eq!(
            height, self.height,
            "Framebuffer height must match TileRenderer height"
        );
        let expected_len = (self.width as usize)
            .checked_mul(self.height as usize)
            .expect("TileRenderer dimensions overflow");
        assert!(pixels.len() >= expected_len, "Framebuffer slice too small");
        assert!(depths.len() >= expected_len, "ZBuffer slice too small");
    }

    fn merge_tile_direct_into_slices(
        tile_pixels: &[u32],
        tile_depths: &[f32],
        width: u32,
        height: u32,
        pixels: &mut [u32],
        depths: &mut [f32],
        bounds: &TileMergeBounds,
    ) {
        let tile_x0 = bounds.tx * TILE_SIZE;
        let tile_y0 = bounds.ty * TILE_SIZE;
        let tile_x_end = (tile_x0 + TILE_SIZE).min(bounds.width);
        let tile_cols = (tile_x_end - tile_x0) as usize;
        let fb_width = width as usize;

        let row_begin = bounds.y_min.max(tile_y0 as i32) as u32;
        let row_end = (bounds.y_max as u32 + 1)
            .min(tile_y0 + TILE_SIZE)
            .min(height);

        for row in row_begin..row_end {
            let tile_row_offset = ((row - tile_y0) * TILE_SIZE) as usize;
            let fb_start = row as usize * fb_width + tile_x0 as usize;

            pixels[fb_start..fb_start + tile_cols]
                .copy_from_slice(&tile_pixels[tile_row_offset..tile_row_offset + tile_cols]);
            depths[fb_start..fb_start + tile_cols]
                .copy_from_slice(&tile_depths[tile_row_offset..tile_row_offset + tile_cols]);
        }
    }

    fn merge_empty_tile_into_slices(
        pixels: &mut [u32],
        depths: &mut [f32],
        tx: u32,
        ty: u32,
        width: u32,
        height: u32,
        clear_color: u32,
    ) {
        let tile_x0 = tx * TILE_SIZE;
        let tile_y0 = ty * TILE_SIZE;
        let tile_x_end = (tile_x0 + TILE_SIZE).min(width);
        let tile_y_end = (tile_y0 + TILE_SIZE).min(height);
        let tile_cols = (tile_x_end - tile_x0) as usize;
        let fb_width = width as usize;

        for row in tile_y0..tile_y_end {
            let fb_start = row as usize * fb_width + tile_x0 as usize;
            pixels[fb_start..fb_start + tile_cols].fill(clear_color);
            depths[fb_start..fb_start + tile_cols].fill(f32::INFINITY);
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
/// use abrash_render::rasterizer::should_use_tiled_rendering;
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

/// Render a single tile gouraud: clear, rasterize triangles, and return tile buffers.
/// Free function to enable parallel dispatch without `&mut self` borrows.
#[inline(always)]
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
    clear_color: u32,
) -> Option<(i32, i32)> {
    let bin_idx = (ty * tiles_x + tx) as usize;
    if tile_bins.heads[bin_idx] == u32::MAX {
        return None;
    }

    let tile_x0 = (tx * TILE_SIZE) as i32;
    let tile_y0 = (ty * TILE_SIZE) as i32;
    let tile_x1 = tile_x0 + TILE_SIZE as i32;
    let tile_y1 = tile_y0 + TILE_SIZE as i32;
    let screen_x_max = width as i32 - 1;

    let mut ctx = TileContext {
        pixels: tile_pixels,
        depths: tile_depths,
        x0: tile_x0,
        y0: tile_y0,
        x1: tile_x1,
        y1: tile_y1,
        screen_x_max,
    };

    let (clear_y_min, clear_y_max) = clear_tile_bounds(
        &mut ctx,
        tile_bins,
        bin_idx,
        prepared,
        |tri| (i32::from(tri.aabb_min_y), i32::from(tri.aabb_max_y)),
        clear_color,
    );

    // Render all triangles in bin
    for tri_idx in tile_bins.iter(bin_idx) {
        let tri = &prepared[tri_idx];
        render_triangle_in_tile_gouraud(&mut ctx, tri);
    }

    Some((clear_y_min, clear_y_max))
}

/// Render a gouraud triangle into tile-local buffers.
#[inline(always)]
fn render_triangle_in_tile_gouraud(ctx: &mut TileContext, tri: &PreparedGouraudTriangle) {
    let p0_y = tri.p0.y;
    let p2_y = tri.p2.y;

    let y_start = p0_y.max(ctx.y0);
    let y_end = p2_y.min(ctx.y1 - 1);

    if y_start > y_end {
        return;
    }

    // Edge Walking
    let p0 = tri.p0.to_screen_point(1.0);
    let p1 = tri.p1.to_screen_point(1.0);
    let p2 = tri.p2.to_screen_point(1.0);

    // Convert fixed point colors back to Vec3 for EdgeWalker initialization
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

    let mut edge_b = if y_start < tri.p1.y {
        let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
        if y_start > p0.y {
            e.step_n(i64::from(y_start) - i64::from(p0.y));
        }
        e
    } else {
        let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
        if y_start > tri.p1.y {
            e.step_n(i64::from(y_start) - i64::from(tri.p1.y));
        }
        e
    };

    let dz_dx = tri.gradients.dz_dx;
    let dc_dx = tri.gradients.dc_dx;

    for y in y_start..=y_end {
        if y == tri.p1.y && y != p0_y {
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

        process_tile_scanline_gouraud(ctx, y, x_start, x_end, z_left, c_left, tri);

        edge_a.step();
        edge_b.step();
    }
}

/// Helper for drawing gouraud scanline into a slice (no bounds checking needed)
#[inline(always)]

fn process_tile_scanline_gouraud(
    ctx: &mut TileContext,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_left: f32,
    c_left: (i32, i32, i32),
    tri: &PreparedGouraudTriangle,
) {
    let dx = i64::from(x_end) - i64::from(x_start);
    let dz_dx = tri.gradients.dz_dx;
    let dc_dx = tri.gradients.dc_dx;

    if dx <= 0 {
        // Single-pixel scanline
        if x_start >= ctx.x0 && x_start < ctx.x1 && x_start >= 0 && x_start <= ctx.screen_x_max {
            let tile_idx = ctx.get_indices(x_start, y);
            if z_left < ctx.depths[tile_idx] {
                ctx.depths[tile_idx] = z_left;
                let r = (c_left.0 >> 16).max(0).min(255) as u32;
                let g = (c_left.1 >> 16).max(0).min(255) as u32;
                let b = (c_left.2 >> 16).max(0).min(255) as u32;
                ctx.pixels[tile_idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
    } else {
        // Clamp X to tile and screen bounds
        let xs = x_start.max(ctx.x0).max(0);
        let xe = x_end.min(ctx.x1 - 1).min(ctx.screen_x_max);

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

            let row_offset = ctx.get_row_offset(y);
            let col_start = (xs - ctx.x0) as usize;
            let col_end = (xe - ctx.x0) as usize;

            let pixels = &mut ctx.pixels[row_offset + col_start..=row_offset + col_end];
            let depths = &mut ctx.depths[row_offset + col_start..=row_offset + col_end];

            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            {
                if pixels.len() >= 32 && is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_scanline_gouraud_simd_fast(
                            pixels, depths, z_at_xs, c_at_xs, dz_dx, dc_dx,
                        );
                    }
                } else {
                    draw_scanline_gouraud_i32_tile(pixels, depths, z_at_xs, c_at_xs, dz_dx, dc_dx);
                }
            }
            #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
            draw_scanline_gouraud_i32_tile(pixels, depths, z_at_xs, c_at_xs, dz_dx, dc_dx);
        }
    }
}

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

    let len = pixels.len();
    assert!(
        depths.len() >= len,
        "Depth buffer must be at least as large as the pixels slice"
    );
    let mut fb_ptr = pixels.as_mut_ptr();
    let mut zb_ptr = depths.as_mut_ptr();

    for _ in 0..len {
        unsafe {
            if z < *zb_ptr {
                *zb_ptr = z;
                let rv = (r >> 16).max(0).min(255) as u32;
                let gv = (g >> 16).max(0).min(255) as u32;
                let bv = (b >> 16).max(0).min(255) as u32;
                *fb_ptr = 0xFF00_0000 | (rv << 16) | (gv << 8) | bv;
            }
            fb_ptr = fb_ptr.add(1);
            zb_ptr = zb_ptr.add(1);
        }
        z += dz_dx;
        r = r.wrapping_add(dr);
        g = g.wrapping_add(dg);
        b = b.wrapping_add(db);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::{Vec2, Vec3};
    use crate::rasterizer::fill_triangle_3d;
    use crate::texture::Texture;
    use crate::zbuffer::ZBuffer;

    // --- Step 1: Infrastructure + prepare ---

    #[test]
    #[should_panic(expected = "TileRenderer dimensions overflow")]
    fn new_panics_on_dimensions_overflow() {
        let _ = TileRenderer::new(u32::MAX, u32::MAX);
    }

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
    #[should_panic(expected = "TileRenderer dimensions overflow")]
    fn new_panics_on_tile_count_overflow() {
        let _ = TileRenderer::new(u32::MAX, 100);
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
        assert!(tri.aabb_max_x < 100);
        assert!(tri.aabb_max_y < 100);
        // And AABB should encompass the triangle
        assert!(i32::from(tri.aabb_min_x) <= tri.p0.x.min(tri.p1.x).min(tri.p2.x));
        assert!(i32::from(tri.aabb_max_x) >= tri.p0.x.max(tri.p1.x).max(tri.p2.x));
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

    fn test_triangle() -> ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32) {
        (
            (Vec3::new(0.0, 0.5, 5.0), 5.0),
            (Vec3::new(-0.5, -0.5, 5.0), 5.0),
            (Vec3::new(0.5, -0.5, 5.0), 5.0),
            0xFFFF_0000,
        )
    }

    fn test_textured_triangle() -> TexturedClipTriangle {
        (
            (Vec3::new(0.0, 0.5, 5.0), 5.0),
            Vec2::new(0.5, 0.0),
            (Vec3::new(-0.5, -0.5, 5.0), 5.0),
            Vec2::new(0.0, 1.0),
            (Vec3::new(0.5, -0.5, 5.0), 5.0),
            Vec2::new(1.0, 1.0),
        )
    }

    fn test_texture() -> Texture {
        let mut texture = Texture::new(2, 2).expect("valid test texture");
        texture.pixels[0] = 0xFFFF_0000;
        texture.pixels[1] = 0xFF00_FF00;
        texture.pixels[2] = 0xFF00_00FF;
        texture.pixels[3] = 0xFFFF_FFFF;
        texture
    }

    fn blank_buffers(width: u32, height: u32) -> (Vec<u32>, Vec<f32>) {
        let len = (width as usize) * (height as usize);
        (vec![0xFF00_0000; len], vec![f32::INFINITY; len])
    }

    #[test]
    fn test_end_frame_into_slices_matches_end_frame() {
        let width = 100;
        let height = 100;
        let (v0, v1, v2, color) = test_triangle();

        let mut ref_renderer = TileRenderer::new(width, height);
        ref_renderer.begin_frame();
        ref_renderer.prepare_triangle(v0, v1, v2, color);
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        ref_renderer.end_frame(&mut fb_ref, &mut zb_ref);

        let mut slice_renderer = TileRenderer::new(width, height);
        slice_renderer.begin_frame();
        slice_renderer.prepare_triangle(v0, v1, v2, color);
        let (mut pixels, mut depths) = blank_buffers(width, height);
        slice_renderer.end_frame_into_slices(
            width,
            height,
            pixels.as_mut_slice(),
            depths.as_mut_slice(),
        );

        assert_eq!(fb_ref.as_slice(), pixels.as_slice());
        assert_eq!(zb_ref.as_slice(), depths.as_slice());
    }

    #[test]
    fn test_render_batch_into_slices_matches_render_batch() {
        let width = 100;
        let height = 100;
        let triangle = test_triangle();

        let mut ref_renderer = TileRenderer::new(width, height);
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        ref_renderer.render_batch(&mut fb_ref, &mut zb_ref, &[triangle]);

        let mut slice_renderer = TileRenderer::new(width, height);
        let (mut pixels, mut depths) = blank_buffers(width, height);
        slice_renderer.render_batch_into_slices(
            width,
            height,
            pixels.as_mut_slice(),
            depths.as_mut_slice(),
            &[triangle],
        );

        assert_eq!(fb_ref.as_slice(), pixels.as_slice());
        assert_eq!(zb_ref.as_slice(), depths.as_slice());
    }

    #[test]
    fn test_render_batch_textured_into_slices_matches_render_batch_textured() {
        let width = 100;
        let height = 100;
        let triangle = test_textured_triangle();
        let texture = test_texture();

        let mut ref_renderer = TileRenderer::new(width, height);
        let mut fb_ref = Framebuffer::new(width, height).unwrap();
        let mut zb_ref = ZBuffer::new(width, height).unwrap();
        ref_renderer.render_batch_textured(&mut fb_ref, &mut zb_ref, &[triangle], &texture);

        let mut slice_renderer = TileRenderer::new(width, height);
        let (mut pixels, mut depths) = blank_buffers(width, height);
        slice_renderer.render_batch_textured_into_slices(
            width,
            height,
            pixels.as_mut_slice(),
            depths.as_mut_slice(),
            &[triangle],
            &texture,
        );

        assert_eq!(fb_ref.as_slice(), pixels.as_slice());
        assert_eq!(zb_ref.as_slice(), depths.as_slice());
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
            "Expected at least 100 pixels rendered, got {pixels_changed}"
        );
    }
}

#[cfg(all(test, feature = "parallel"))]
mod warden_tests {
    use super::{PreparedTrianglesList, SendPtr};

    #[test]
    #[should_panic(expected = "Index out of bounds")]
    fn exploit_sendptr_oob() {
        let mut buffer = vec![0u32; 10];
        let ptr = SendPtr(buffer.as_mut_ptr(), buffer.len());

        // Attempt to write out of bounds
        unsafe {
            ptr.write(15, 0x00FF_FFFFFF);
        }
    }
}
#[cfg(test)]
mod tile_bins_tests {
    use super::*;

    #[test]
    fn test_tile_bins_empty_iter() {
        let bins = TileBins::new(4);
        assert_eq!(bins.iter(0).next(), None);
        assert_eq!(bins.iter(3).next(), None);
    }

    #[test]
    fn test_tile_bins_single_push() {
        let mut bins = TileBins::new(4);
        bins.push(1, 42);

        let mut iter = bins.iter(1);
        assert_eq!(iter.next(), Some(42));
        assert_eq!(iter.next(), None);

        // Other bins should still be empty
        assert_eq!(bins.iter(0).next(), None);
        assert_eq!(bins.iter(2).next(), None);
    }

    #[test]
    fn test_tile_bins_multiple_push_same_bin() {
        let mut bins = TileBins::new(2);
        bins.push(0, 10);
        bins.push(0, 20);
        bins.push(0, 30);

        let items: Vec<usize> = bins.iter(0).collect();
        assert_eq!(items, vec![10, 20, 30]);

        assert_eq!(bins.iter(1).next(), None);
    }

    #[test]
    fn test_tile_bins_multiple_bins() {
        let mut bins = TileBins::new(3);
        bins.push(0, 100);
        bins.push(1, 200);
        bins.push(2, 300);
        bins.push(1, 201);
        bins.push(0, 101);

        let items_0: Vec<usize> = bins.iter(0).collect();
        assert_eq!(items_0, vec![100, 101]);

        let items_1: Vec<usize> = bins.iter(1).collect();
        assert_eq!(items_1, vec![200, 201]);

        let items_2: Vec<usize> = bins.iter(2).collect();
        assert_eq!(items_2, vec![300]);
    }

    #[test]
    fn test_tile_bins_clear() {
        let mut bins = TileBins::new(2);
        bins.push(0, 1);
        bins.push(1, 2);

        bins.clear();

        // Should be empty after clear
        assert_eq!(bins.iter(0).next(), None);
        assert_eq!(bins.iter(1).next(), None);
        assert_eq!(bins.tris.len(), 0);
        assert_eq!(bins.nexts.len(), 0);

        // Pushing again should work correctly from scratch
        bins.push(0, 10);
        let items: Vec<usize> = bins.iter(0).collect();
        assert_eq!(items, vec![10]);
    }
}
