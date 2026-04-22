//! High-level Scene management with Object Culling.
//!
//! This module provides a scene graph structure that organizes objects and performs
//! frustum culling before submitting visible geometry to the rasterizer.
//!
//! # The Scene Graph
//!
//! In Abrash, a [`Scene`] is a collection of [`SceneObject`]s and a [`Camera`].
//! The scene is responsible for:
//!
//! 1.  **Spatial Organization**: Managing objects and their transforms.
//! 2.  **Culling**: Determining which objects are visible to the camera (Frustum Culling).
//! 3.  **Rendering**: Submitting visible geometry to a [`TileRenderer`] or other rasterizer.
//!
//! # Coordinate Spaces
//!
//! *   **Model Space**: The local coordinates of the mesh vertices.
//! *   **World Space**: The coordinates of the object in the scene (after applying `object.transform`).
//! *   **View Space**: The coordinates relative to the camera.
//! *   **Clip Space**: The coordinates after projection (before perspective division).
//!
//! The [`Scene::render`] method transforms vertices from Model Space directly to Clip Space
//! using a combined Model-View-Projection (MVP) matrix for efficiency.
//!
//! # Usage
//!
//! ```
//! use abrash_render::scene::{Scene, SceneObject, Camera};
//! use abrash_core::mesh::Mesh;
//! use abrash_core::math::{Mat4, Vec3};
//! use abrash_render::rasterizer::TileRenderer;
//! use abrash_core::framebuffer::Framebuffer;
//! use abrash_core::zbuffer::ZBuffer;
//! use std::sync::Arc;
//!
//! // 1. Setup Renderer and Buffers
//! let width = 800;
//! let height = 600;
//! let mut fb = Framebuffer::new(width, height).unwrap();
//! let mut zb = ZBuffer::new(width, height).unwrap();
//! let mut renderer = TileRenderer::new(width, height);
//!
//! // 2. Setup Camera
//! let eye = Vec3::new(0.0, 5.0, 10.0);
//! let target = Vec3::new(0.0, 0.0, 0.0);
//! let up = Vec3::new(0.0, 1.0, 0.0);
//!
//! let view = Mat4::look_at(eye, target, up);
//! let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
//! let camera = Camera::new(view, proj);
//!
//! // 3. Create Scene
//! let mut scene = Scene::new(camera);
//!
//! // 4. Add Objects
//! let mesh = Arc::new(Mesh::cube(1.0));
//! let transform = Mat4::translation(0.0, 0.0, 0.0);
//! let object = SceneObject::new(mesh, transform, 0xFFFF0000); // Red Cube
//! scene.add_object(object);
//!
//! // 5. Render
//! scene.render(&mut renderer, &mut fb, &mut zb);
//! ```

use crate::culling::Frustum;
use crate::framebuffer::Framebuffer;
use crate::geometry::AABB;
use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;
use crate::rasterizer::TileRenderer;
use crate::render_api::draw_list::{DrawBatch, DrawList};
use crate::render_api::frame::FrameCamera;
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;
use std::sync::Arc;

/// A single object in the scene.
///
/// An object consists of a geometric [`Mesh`], a transformation [`Mat4`],
/// and a base color.
///
/// # Axis-Aligned Bounding Box (AABB)
///
/// Each object maintains a pre-calculated Local AABB. When rendering, the Scene
/// transforms this AABB to World Space to perform fast Frustum Culling.
pub struct SceneObject {
    /// The geometric mesh data.
    pub mesh: Arc<Mesh>,
    /// The object's transformation matrix (Model Matrix).
    pub transform: Mat4,
    /// The object's Axis-Aligned Bounding Box in Local Space.
    /// This is pre-calculated from the mesh.
    pub local_aabb: AABB,
    /// The color of the object (if not using textures/materials).
    pub color: u32,
    /// Shared copy of the indices for efficient batch creation.
    pub shared_indices: std::sync::Arc<[[usize; 3]]>,
}

impl SceneObject {
    /// Create a new scene object from a mesh and transform.
    /// Automatically calculates the local AABB.
    #[must_use]
    pub fn new(mesh: Arc<Mesh>, transform: Mat4, color: u32) -> Self {
        let local_aabb = AABB::from_points(&mesh.vertices);
        let shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
        Self {
            mesh,
            transform,
            local_aabb,
            color,
            shared_indices,
        }
    }

    /// Calculate the World Space AABB by transforming the Local AABB.
    #[must_use]
    pub fn calculate_world_aabb(&self) -> AABB {
        self.local_aabb.transform(&self.transform)
    }
}

/// A Camera defined by View and Projection matrices.
///
/// The camera also maintains a [`Frustum`] derived from the View-Projection matrix,
/// which is used for culling objects that are outside the field of view.
pub struct Camera {
    /// The View Matrix (World Space -> View Space).
    pub view: Mat4,
    /// The Projection Matrix (View Space -> Clip Space).
    pub proj: Mat4,
    /// The View Frustum extracted from `view * proj`.
    pub frustum: Frustum,
}

impl Camera {
    /// Create a new camera.
    ///
    /// Automatically calculates the View Frustum.
    #[must_use]
    pub fn new(view: Mat4, proj: Mat4) -> Self {
        let view_proj = view * proj;
        let frustum = Frustum::from_matrix(view_proj);
        Self {
            view,
            proj,
            frustum,
        }
    }

    /// Update the view and projection matrices, and recalculate the frustum.
    pub fn update(&mut self, view: Mat4, proj: Mat4) {
        self.view = view;
        self.proj = proj;
        let view_proj = view * proj;
        self.frustum = Frustum::from_matrix(view_proj);
    }
}

thread_local! {
    static RENDER_CONTEXT: RefCell<SceneRenderContext> = RefCell::new(SceneRenderContext::default());
}

#[derive(Default)]
struct SceneRenderContext {
    world_aabbs: Vec<AABB>,
    cull_results: Vec<bool>,
}

/// The Scene containing objects and the camera.
///
/// See the [module-level documentation](self) for usage examples.
pub struct Scene {
    /// The collection of renderable objects in the scene.
    pub objects: Vec<SceneObject>,
    /// The camera used to view the scene.
    pub camera: Camera,
}

impl Scene {
    /// Create a new scene.
    #[must_use]
    pub const fn new(camera: Camera) -> Self {
        Self {
            objects: Vec::new(),
            camera,
        }
    }

    /// ⚡ Bolt: Create a new scene with pre-allocated capacity for objects.
    /// This eliminates heap reallocations when registering initial scene assets.
    #[must_use]
    pub fn with_capacity(camera: Camera, capacity: usize) -> Self {
        Self {
            objects: Vec::with_capacity(capacity),
            camera,
        }
    }

    /// Add an object to the scene.
    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.push(object);
    }

    /// Extract the scene into a backend-agnostic [`DrawList`].
    ///
    /// Performs frustum culling and transforms all visible object vertices to clip-space.
    /// The returned `DrawList` is ready for rasterization by any backend — the caller
    /// does not need scene-level concepts (cameras, transforms) to execute it.
    ///
    /// Use [`render`](Self::render) for a one-shot path that executes immediately.
    /// Use `extract` when you need to inspect, sort, or route the draw list first.
    ///
    /// # Performance
    ///
    /// Each visible object allocates a `Vec<(Vec3, f32)>` for its clip-space vertices.
    /// The culling step still uses thread-local scratch buffers to stay allocation-free.
    #[must_use]
    pub fn extract(&self) -> DrawList {
        let view_proj = self.camera.view * self.camera.proj;
        let camera = FrameCamera::new(self.camera.view, self.camera.proj);

        RENDER_CONTEXT.with(|ctx_cell| {
            let mut ctx_guard = ctx_cell.borrow_mut();
            let ctx = &mut *ctx_guard;

            let world_aabbs = &mut ctx.world_aabbs;
            let cull_results = &mut ctx.cull_results;

            let num_objects = self.objects.len();
            world_aabbs.clear();
            world_aabbs.reserve(num_objects);
            cull_results.clear();
            cull_results.resize(num_objects, false);

            for obj in &self.objects {
                world_aabbs.push(obj.local_aabb.transform(&obj.transform));
            }

            self.camera
                .frustum
                .cull_aabbs_prealloc(world_aabbs, cull_results);

            // Pre-calculate visible objects and total required vertices to avoid dynamic reallocations
            let mut total_vertices = 0;
            let mut visible_count = 0;
            for (obj, &is_visible) in self.objects.iter().zip(cull_results.iter()) {
                if is_visible {
                    total_vertices += obj.mesh.vertices.len();
                    visible_count += 1;
                }
            }

            let mut draw_list = DrawList::with_capacity(camera, visible_count, total_vertices, 0);

            for (obj, &is_visible) in self.objects.iter().zip(cull_results.iter()) {
                if !is_visible {
                    continue;
                }

                let mvp = obj.transform * view_proj;
                let mesh = &obj.mesh;

                let start_idx = draw_list.vertices.len();
                let end_idx = start_idx + mesh.vertices.len();

                // Extract uninitialized slice from the reserved capacity
                let uninit_slice = draw_list.vertices.spare_capacity_mut();
                let uninit_slice = &mut uninit_slice[..mesh.vertices.len()];

                #[cfg(feature = "parallel")]
                mvp.transform_points_uninit_parallel(&mesh.vertices, uninit_slice);

                #[cfg(not(feature = "parallel"))]
                mvp.transform_points_uninit(&mesh.vertices, uninit_slice);

                // SAFETY: We have initialized `mesh.vertices.len()` elements via `transform_points_uninit*`.
                unsafe {
                    draw_list.vertices.set_len(end_idx);
                }

                draw_list.push(DrawBatch::new(
                    start_idx..end_idx,
                    std::sync::Arc::clone(&obj.shared_indices),
                    obj.color,
                ));
            }

            draw_list
        })
    }

    /// Render the scene using the provided renderer.
    ///
    /// Internally calls [`extract`](Self::extract) to build a [`DrawList`], then executes
    /// it through the tile renderer. Use `extract` directly when you need to inspect
    /// or manipulate the draw list before rasterization.
    ///
    /// # Performance
    ///
    /// *   **Culling**: Objects completely outside the frustum are skipped entirely.
    /// *   **Batching**: Vertex transformations use thread-local scratch buffers.
    pub fn render(&self, renderer: &mut TileRenderer, fb: &mut Framebuffer, zb: &mut ZBuffer) {
        let draw_list = self.extract();

        renderer.begin_frame();
        for batch in &draw_list.batches {
            renderer.submit_mesh(
                &batch.indices,
                &draw_list.vertices[batch.vertex_range.start..batch.vertex_range.end],
                batch.color,
            );
        }
        renderer.end_frame(fb, zb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn test_calculate_world_aabb_matches_naive() {
        // Create a mesh with known bounds (e.g., unit cube centered at origin)
        // Local AABB: [-0.5, -0.5, -0.5] to [0.5, 0.5, 0.5]
        let mesh = Arc::new(Mesh::cube(1.0));

        // Create a transform: Rotate 45 deg around Y, Translate (10, 0, 0)
        let rotation = Mat4::rotation_y(std::f32::consts::FRAC_PI_4);
        let translation = Mat4::translation(10.0, 0.0, 0.0);
        let transform = rotation * translation; // Translate then Rotate? No, row vector v*R*T

        // In row-vector convention: v' = v * R * T.
        // First rotate, then translate.

        let obj = SceneObject::new(mesh, transform, 0xFFFFFFFF);

        let calculated_aabb = obj.local_aabb.transform(&obj.transform);

        // Naive calculation for verification
        let min = obj.local_aabb.min;
        let max = obj.local_aabb.max;
        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
        ];

        let mut expected_min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut expected_max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for corner in corners {
            let (p, _) = transform.transform_point(corner);
            if p.x < expected_min.x {
                expected_min.x = p.x;
            }
            if p.y < expected_min.y {
                expected_min.y = p.y;
            }
            if p.z < expected_min.z {
                expected_min.z = p.z;
            }

            if p.x > expected_max.x {
                expected_max.x = p.x;
            }
            if p.y > expected_max.y {
                expected_max.y = p.y;
            }
            if p.z > expected_max.z {
                expected_max.z = p.z;
            }
        }

        // Check with epsilon
        let diff_min = calculated_aabb.min - expected_min;
        let diff_max = calculated_aabb.max - expected_max;

        assert!(
            diff_min.length() < 0.0001,
            "Min bounds mismatch: {:?} vs {:?}",
            calculated_aabb.min,
            expected_min
        );
        assert!(
            diff_max.length() < 0.0001,
            "Max bounds mismatch: {:?} vs {:?}",
            calculated_aabb.max,
            expected_max
        );
    }

    fn test_scene() -> (Scene, Camera) {
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let camera = Camera::new(view, proj);
        let scene = Scene::new(camera);
        let camera2 = Camera::new(view, proj);
        (scene, camera2)
    }

    #[test]
    fn test_extract_empty_scene() {
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let scene = Scene::new(Camera::new(view, proj));
        let dl = scene.extract();
        assert!(dl.batches.is_empty());
        assert_eq!(dl.triangle_count(), 0);
    }

    #[test]
    fn test_extract_visible_object_produces_batch() {
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let mut scene = Scene::new(Camera::new(view, proj));

        let mesh = Arc::new(Mesh::cube(1.0));
        scene.add_object(SceneObject::new(
            mesh.clone(),
            Mat4::identity(),
            0xFFFF_0000,
        ));

        let dl = scene.extract();
        assert_eq!(dl.batches.len(), 1);
        assert_eq!(dl.batches[0].color, 0xFFFF_0000);
        assert_eq!(dl.batches[0].indices.len(), mesh.indices.len());
        assert_eq!(
            dl.vertices[dl.batches[0].vertex_range.start..dl.batches[0].vertex_range.end].len(),
            mesh.vertices.len()
        );
    }

    #[test]
    fn test_extract_culls_objects_behind_camera() {
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let mut scene = Scene::new(Camera::new(view, proj));

        // Object far behind the camera (z = +500, camera looks toward -z)
        let mesh = Arc::new(Mesh::cube(1.0));
        scene.add_object(SceneObject::new(
            mesh,
            Mat4::translation(0.0, 0.0, 500.0),
            0xFFFF_0000,
        ));

        let dl = scene.extract();
        assert!(
            dl.batches.is_empty(),
            "Object behind camera should be culled"
        );
    }

    #[test]
    fn test_extract_multiple_objects() {
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 200.0);
        let mut scene = Scene::new(Camera::new(view, proj));

        let mesh = Arc::new(Mesh::cube(1.0));
        scene.add_object(SceneObject::new(
            mesh.clone(),
            Mat4::translation(-2.0, 0.0, 0.0),
            0xFFFF_0000,
        ));
        scene.add_object(SceneObject::new(
            mesh,
            Mat4::translation(2.0, 0.0, 0.0),
            0xFF00_FF00,
        ));

        let dl = scene.extract();
        assert_eq!(dl.batches.len(), 2);
    }
}
