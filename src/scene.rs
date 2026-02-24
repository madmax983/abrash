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
//! use abrash::scene::{Scene, SceneObject, Camera};
//! use abrash::mesh::Mesh;
//! use abrash::math::{Mat4, Vec3};
//! use abrash::tile_renderer::TileRenderer;
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
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
use crate::math::{Mat4, Vec3};
use crate::mesh::{AABB, Mesh};
use crate::tile_renderer::{ClipTriangle, TileRenderer};
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
}

impl SceneObject {
    /// Create a new scene object from a mesh and transform.
    /// Automatically calculates the local AABB.
    #[must_use]
    pub fn new(mesh: Arc<Mesh>, transform: Mat4, color: u32) -> Self {
        let local_aabb = AABB::from_points(&mesh.vertices);
        Self {
            mesh,
            transform,
            local_aabb,
            color,
        }
    }

    /// Calculate the World Space AABB by transforming the local AABB corners.
    /// Note: This results in a loose-fitting AABB (AABB of the OBB).
    ///
    /// Optimized using Arvo's algorithm (Transforming Center & Extents) to avoid
    /// transforming all 8 corners explicitly.
    #[must_use]
    pub fn calculate_world_aabb(&self) -> AABB {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if is_x86_feature_detected!("sse2") {
            // SAFETY: We checked feature detection.
            unsafe {
                return self.calculate_world_aabb_simd();
            }
        }

        let min = self.local_aabb.min;
        let max = self.local_aabb.max;
        let m = &self.transform.m;

        let right = Vec3::new(m[0][0], m[0][1], m[0][2]);
        let up = Vec3::new(m[1][0], m[1][1], m[1][2]);
        let back = Vec3::new(m[2][0], m[2][1], m[2][2]);
        let translation = Vec3::new(m[3][0], m[3][1], m[3][2]);

        let xa = right * min.x;
        let xb = right * max.x;

        let ya = up * min.y;
        let yb = up * max.y;

        let za = back * min.z;
        let zb = back * max.z;

        // Arvo's algorithm:
        // NewMin = Translation + sum(min(a, b))
        // NewMax = Translation + sum(max(a, b))
        // where a = M * min, b = M * max (component-wise)

        let world_min = translation + xa.min(xb) + ya.min(yb) + za.min(zb);
        let world_max = translation + xa.max(xb) + ya.max(yb) + za.max(zb);

        AABB::new(world_min, world_max)
    }

    /// SIMD-optimized implementation of Arvo's algorithm using SSE.
    ///
    /// This vectorized version processes x, y, and z components of the result simultaneously,
    /// avoiding the overhead of scalar component-wise operations.
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[target_feature(enable = "sse2")]
    unsafe fn calculate_world_aabb_simd(&self) -> AABB {
        use std::arch::x86_64::{
            _mm_add_ps, _mm_load_ps, _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_set_ps,
            _mm_shuffle_ps, _mm_storeu_ps,
        };

        // SAFETY:
        // 1. SSE2 intrinsics are safe if feature is detected (checked by caller or cfg).
        // 2. `self.transform.m` is `Mat4` which is `#[repr(align(16))]`, ensuring 16-byte alignment
        //    required by `_mm_load_ps`.
        unsafe {
            let m = &self.transform.m;
            // Load rows. Mat4 is 16-byte aligned.
            let r = _mm_load_ps(m[0].as_ptr());
            let u = _mm_load_ps(m[1].as_ptr());
            let b = _mm_load_ps(m[2].as_ptr());
            let t = _mm_load_ps(m[3].as_ptr());

            let min = &self.local_aabb.min;
            let max = &self.local_aabb.max;

            // Load min/max safely. Vec3 is x, y, z.
            // We set w to 0.0 to avoid affecting translation (which has w=1.0).
            let min_v = _mm_set_ps(0.0, min.z, min.y, min.x);
            let max_v = _mm_set_ps(0.0, max.z, max.y, max.x);

            // Broadcast components
            // x
            let min_x = _mm_shuffle_ps(min_v, min_v, 0x00); // 0,0,0,0
            let max_x = _mm_shuffle_ps(max_v, max_v, 0x00);

            // y
            let min_y = _mm_shuffle_ps(min_v, min_v, 0x55); // 1,1,1,1
            let max_y = _mm_shuffle_ps(max_v, max_v, 0x55);

            // z
            let min_z = _mm_shuffle_ps(min_v, min_v, 0xAA); // 2,2,2,2
            let max_z = _mm_shuffle_ps(max_v, max_v, 0xAA);

            // X axis terms (Row 0)
            let xa = _mm_mul_ps(r, min_x);
            let xb = _mm_mul_ps(r, max_x);
            let min_term_x = _mm_min_ps(xa, xb);
            let max_term_x = _mm_max_ps(xa, xb);

            // Y axis terms (Row 1)
            let ya = _mm_mul_ps(u, min_y);
            let yb = _mm_mul_ps(u, max_y);
            let min_term_y = _mm_min_ps(ya, yb);
            let max_term_y = _mm_max_ps(ya, yb);

            // Z axis terms (Row 2)
            let za = _mm_mul_ps(b, min_z);
            let zb = _mm_mul_ps(b, max_z);
            let min_term_z = _mm_min_ps(za, zb);
            let max_term_z = _mm_max_ps(za, zb);

            // Sum everything
            let sum_min = _mm_add_ps(
                min_term_x,
                _mm_add_ps(min_term_y, _mm_add_ps(min_term_z, t)),
            );
            let sum_max = _mm_add_ps(
                max_term_x,
                _mm_add_ps(max_term_y, _mm_add_ps(max_term_z, t)),
            );

            // Store back to Vec3
            // We can extract via storeu
            let mut min_arr = [0.0; 4];
            let mut max_arr = [0.0; 4];
            _mm_storeu_ps(min_arr.as_mut_ptr(), sum_min);
            _mm_storeu_ps(max_arr.as_mut_ptr(), sum_max);

            AABB::new(
                Vec3::new(min_arr[0], min_arr[1], min_arr[2]),
                Vec3::new(max_arr[0], max_arr[1], max_arr[2]),
            )
        }
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
    transformed_verts: Vec<(Vec3, f32)>,
    triangle_batch: Vec<ClipTriangle>,
}

/// The Scene containing objects and the camera.
///
/// See the [module-level documentation](self) for usage examples.
pub struct Scene {
    pub objects: Vec<SceneObject>,
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

    /// Add an object to the scene.
    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.push(object);
    }

    /// Render the scene using the provided renderer.
    ///
    /// This method performs Object Culling (Frustum Culling) before processing vertices.
    /// Visible objects are transformed to Clip Space and submitted to the `renderer`.
    ///
    /// # Performance
    ///
    /// *   **Culling**: Objects completely outside the frustum are skipped entirely.
    /// *   **Batching**: Vertex transformations are batched and (optionally) parallelized.
    pub fn render(&self, renderer: &mut TileRenderer, fb: &mut Framebuffer, zb: &mut ZBuffer) {
        let view_proj = self.camera.view * self.camera.proj;

        // Use thread-local scratch buffers to avoid per-frame allocations
        RENDER_CONTEXT.with(|ctx_cell| {
            let mut ctx_guard = ctx_cell.borrow_mut();
            let ctx = &mut *ctx_guard;
            let triangle_batch = &mut ctx.triangle_batch;
            let transformed_verts = &mut ctx.transformed_verts;

            triangle_batch.clear();

            for obj in &self.objects {
                // 1. Calculate World AABB
                let world_aabb = obj.calculate_world_aabb();

                // 2. Frustum Cull
                if !self.camera.frustum.intersects_aabb(&world_aabb) {
                    continue;
                }

                // 3. Process Visible Object
                let mvp = obj.transform * view_proj;
                let mesh = &obj.mesh;

                // Transform vertices and append to batch
                // Optimization: Batch transform vertices to reuse calculations for shared vertices.
                // We reuse the scratch buffer to eliminate per-object allocations.
                transformed_verts.clear();
                transformed_verts.reserve(mesh.vertices.len());

                // Use spare_capacity_mut to get uninitialized memory safely
                let uninit_slice = transformed_verts.spare_capacity_mut();
                let uninit_slice = &mut uninit_slice[..mesh.vertices.len()];

                #[cfg(feature = "parallel")]
                mvp.transform_points_uninit_parallel(&mesh.vertices, uninit_slice);

                #[cfg(not(feature = "parallel"))]
                mvp.transform_points_uninit(&mesh.vertices, uninit_slice);

                // SAFETY: We have initialized `len` elements via `transform_points_uninit`.
                unsafe {
                    transformed_verts.set_len(mesh.vertices.len());
                }

                for indices in &mesh.indices {
                    let v0 = transformed_verts[indices[0]];
                    let v1 = transformed_verts[indices[1]];
                    let v2 = transformed_verts[indices[2]];

                    triangle_batch.push((v0, v1, v2, obj.color));
                }
            }

            // 4. Submit Batch
            if !triangle_batch.is_empty() {
                renderer.render_batch(fb, zb, triangle_batch);
            }
        });
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

        let calculated_aabb = obj.calculate_world_aabb();

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
}
