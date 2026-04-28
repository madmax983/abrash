//! `DrawList` - backend-agnostic intermediate representation for a rendered frame.
//!
//! The [`DrawList`] is the seam between high-level scene description ([`Frame`], [`Scene`])
//! and low-level rasterization ([`TileRenderer`], GPU compute). Once geometry is in a
//! `DrawList`, the backend does not need to know where it came from.
//!
//! # Pipeline position
//!
//! ```text
//! Frame / Scene
//!     │
//!     ▼ (frustum cull, vertex transform to clip-space)
//! DrawList  ◄─── this module
//!     │
//!     ▼ (bin, rasterize, composite)
//! RenderTarget pixels
//! ```
//!
//! # Extension points
//!
//! `DrawBatch` currently stores a flat `color: u32` sufficient for the CPU path.
//! A GPU backend can carry additional material metadata alongside this (e.g. a GPU
//! buffer index) without changing the core DrawList/DrawBatch layout.
//!
//! [`Frame`]: crate::render_api::frame::Frame
//! [`Scene`]: crate::scene::Scene
//! [`TileRenderer`]: crate::rasterizer::TileRenderer

use crate::math::Vec3;
use crate::render_api::frame::{FrameCamera, Light};

/// A single pre-transformed draw call ready for rasterization.
///
/// Vertices are already in clip-space (post-MVP). The backend only needs to
/// perspective-divide and bin them — no further matrix math required.
#[derive(Debug, Clone)]
pub struct DrawBatch {
    /// Range of vertices inside the parent `DrawList::vertices` flat array.
    pub vertex_range: std::ops::Range<usize>,
    /// Triangle index buffer. Each `[v0, v1, v2]` triplet indexes into the slice defined by `vertex_range`.
    pub indices: std::sync::Arc<[[usize; 3]]>,
    /// Flat surface color (0xAARRGGBB).
    ///
    /// For the CPU path this is passed directly to `submit_mesh`. A GPU backend
    /// would use this as a fallback or resolve it to a GPU material handle instead.
    pub color: u32,
}

impl DrawBatch {
    /// Create a new draw batch.
    #[must_use]
    pub const fn new(
        vertex_range: std::ops::Range<usize>,
        indices: std::sync::Arc<[[usize; 3]]>,
        color: u32,
    ) -> Self {
        Self {
            vertex_range,
            indices,
            color,
        }
    }
}

/// Backend-agnostic draw list for a single frame.
///
/// Produced by extracting a [`Frame`] or [`Scene`] — culling, vertex transformation,
/// and draw-call sorting happen *before* this type is created. The rasterization
/// backend then consumes the `DrawList` without needing scene-level concepts.
///
/// # Example — building manually
///
/// ```
/// use abrash_render::render_api::draw_list::{DrawBatch, DrawList};
/// use abrash_render::render_api::frame::FrameCamera;
/// use abrash_core::math::{Mat4, Vec3};
///
/// let camera = FrameCamera::new(
///     Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
///     Mat4::perspective(1.57, 4.0 / 3.0, 0.1, 100.0),
/// );
/// let mut draw_list = DrawList::new(camera);
/// draw_list.clear_color = Some(0xFF00_0000);
///
/// // Normally produced by Frame/Scene extraction — here we stub it:
/// let batch = DrawBatch::new(0..0, vec![].into(), 0xFFFF_0000);
/// draw_list.push(batch);
/// assert_eq!(draw_list.batches.len(), 1);
/// ```
///
/// [`Frame`]: crate::render_api::frame::Frame
/// [`Scene`]: crate::scene::Scene
#[derive(Debug, Clone)]
pub struct DrawList {
    /// Camera for this frame (view + projection matrices).
    pub camera: FrameCamera,
    /// Active lights. Currently informational; used by backends that support per-pixel lighting.
    pub lights: Vec<Light>,
    /// Flat array of clip-space vertices: `(position, w)` where `w` is the perspective-divide factor.
    /// Batches store index ranges into this array to prevent O(N) allocations.
    pub vertices: Vec<(Vec3, f32)>,
    /// Pre-transformed draw batches in submission order.
    pub batches: Vec<DrawBatch>,
    /// If `Some`, clear the render target to this color before executing batches.
    pub clear_color: Option<u32>,
}

impl DrawList {
    /// Create an empty draw list for the given camera.
    #[must_use]
    pub const fn new(camera: FrameCamera) -> Self {
        Self {
            camera,
            lights: Vec::new(),
            vertices: Vec::new(),
            batches: Vec::new(),
            clear_color: Some(0xFF00_0000),
        }
    }

    /// ⚡ Bolt: Create an empty draw list, pre-allocating the underlying vectors.
    /// This drastically reduces heap reallocations per frame when drawing many objects.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_render::render_api::draw_list::DrawList;
    /// use abrash_render::render_api::frame::FrameCamera;
    /// use abrash_core::math::Mat4;
    /// let camera = FrameCamera::new(Mat4::identity(), Mat4::identity());
    /// let dl = DrawList::with_capacity(camera, 10, 100, 2);
    /// ```
    ///
    /// # Panics
    /// Panics if the requested capacity exceeds the maximum allowed allocation size (`isize::MAX` bytes).
    #[must_use]
    pub fn with_capacity(
        camera: FrameCamera,
        num_commands: usize,
        num_vertices: usize,
        num_lights: usize,
    ) -> Self {
        // WARDEN DEFENSE: Prevent capacity overflow panics
        assert!(num_lights <= (isize::MAX as usize) / std::mem::size_of::<Light>(), "capacity overflow");
        assert!(num_vertices <= (isize::MAX as usize) / std::mem::size_of::<(Vec3, f32)>(), "capacity overflow");
        assert!(num_commands <= (isize::MAX as usize) / std::mem::size_of::<DrawBatch>(), "capacity overflow");

        Self {
            camera,
            lights: Vec::with_capacity(num_lights),
            vertices: Vec::with_capacity(num_vertices),
            batches: Vec::with_capacity(num_commands),
            clear_color: Some(0xFF00_0000),
        }
    }

    /// Append a draw batch.
    pub fn push(&mut self, batch: DrawBatch) {
        self.batches.push(batch);
    }

    /// Clears the draw list while retaining its allocated capacity.
    pub fn clear(&mut self, camera: FrameCamera) {
        self.camera = camera;
        self.lights.clear();
        self.vertices.clear();
        self.batches.clear();
        self.clear_color = Some(0xFF00_0000);
    }

    /// Total number of triangles across all batches.
    ///
    /// Useful for debugging and performance metrics.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_render::render_api::draw_list::DrawList;
    /// use abrash_render::render_api::frame::FrameCamera;
    /// use abrash_core::math::Mat4;
    /// let camera = FrameCamera::new(Mat4::identity(), Mat4::identity());
    /// let dl = DrawList::new(camera);
    /// assert_eq!(dl.triangle_count(), 0);
    /// ```
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.batches.iter().map(|b| b.indices.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};

    fn test_camera() -> FrameCamera {
        FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
        )
    }

    #[test]
    fn test_draw_list_empty() {
        let dl = DrawList::new(test_camera());
        assert!(dl.batches.is_empty());
        assert!(dl.lights.is_empty());
        assert_eq!(dl.clear_color, Some(0xFF00_0000));
        assert_eq!(dl.triangle_count(), 0);
    }

    #[test]
    fn test_draw_list_push_batch() {
        let mut dl = DrawList::new(test_camera());
        dl.vertices.push((Vec3::new(0.0, 0.0, 0.0), 1.0));
        let batch = DrawBatch::new(0..1, vec![[0, 0, 0]].into(), 0xFFFF_0000);
        dl.push(batch);
        assert_eq!(dl.batches.len(), 1);
        assert_eq!(dl.triangle_count(), 1);
    }

    #[test]
    fn test_draw_list_clear() {
        let cam = test_camera();
        let mut dl = DrawList::new(cam);
        dl.vertices.push((Vec3::ZERO, 1.0));
        dl.push(DrawBatch::new(
            0..1,
            std::sync::Arc::new([[0, 1, 2]]),
            0xFF000000,
        ));

        dl.clear(cam);
        assert_eq!(dl.vertices.len(), 0);
        assert_eq!(dl.batches.len(), 0);
    }

    #[test]
    fn test_draw_list_triangle_count_multi_batch() {
        let mut dl = DrawList::new(test_camera());
        // Batch with 2 triangles
        dl.push(DrawBatch::new(
            0..0,
            vec![[0, 1, 2], [3, 4, 5]].into(),
            0xFFFF_0000,
        ));
        // Batch with 3 triangles
        dl.push(DrawBatch::new(
            0..0,
            vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]].into(),
            0xFF00_FF00,
        ));
        assert_eq!(dl.triangle_count(), 5);
    }

    #[test]
    fn test_draw_batch_new() {
        let indices = vec![[0usize, 0, 0]];
        let batch = DrawBatch::new(0..1, indices.into(), 0xFFFF_FFFF);
        assert_eq!(batch.vertex_range, 0..1);
        assert_eq!(batch.indices.len(), 1);
        assert_eq!(batch.color, 0xFFFF_FFFF);
    }
}
