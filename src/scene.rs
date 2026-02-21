//! High-level Scene management with Object Culling.
//!
//! This module provides a scene graph structure that organizes objects and performs
//! frustum culling before submitting visible geometry to the rasterizer.

use crate::culling::Frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3};
use crate::mesh::{AABB, Mesh};
use crate::tile_renderer::TileRenderer;
use crate::zbuffer::ZBuffer;
use std::sync::Arc;

/// A single object in the scene.
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
    pub fn calculate_world_aabb(&self) -> AABB {
        let min = self.local_aabb.min;
        let max = self.local_aabb.max;

        // The 8 corners of the local AABB
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

        let mut world_min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut world_max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for &corner in &corners {
            // Transform point (w=1.0)
            let (p, _) = self.transform.transform_point(corner);

            if p.x < world_min.x {
                world_min.x = p.x;
            }
            if p.y < world_min.y {
                world_min.y = p.y;
            }
            if p.z < world_min.z {
                world_min.z = p.z;
            }

            if p.x > world_max.x {
                world_max.x = p.x;
            }
            if p.y > world_max.y {
                world_max.y = p.y;
            }
            if p.z > world_max.z {
                world_max.z = p.z;
            }
        }

        AABB::new(world_min, world_max)
    }
}

/// A Camera defined by View and Projection matrices.
pub struct Camera {
    pub view: Mat4,
    pub proj: Mat4,
    pub frustum: Frustum,
}

impl Camera {
    /// Create a new camera.
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

/// The Scene containing objects and the camera.
pub struct Scene {
    pub objects: Vec<SceneObject>,
    pub camera: Camera,
}

impl Scene {
    /// Create a new scene.
    pub fn new(camera: Camera) -> Self {
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
    pub fn render(&self, renderer: &mut TileRenderer, fb: &mut Framebuffer, zb: &mut ZBuffer) {
        let view_proj = self.camera.view * self.camera.proj;
        let mut triangle_batch = Vec::new();

        // Reserve capacity to avoid frequent reallocs
        // Heuristic: visible objects * average triangles per object
        // For now, just a safe guess or leave it dynamic.

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
            // Note: This naive iteration transforms all triangles.
            // Further optimization could use backface culling here too, but
            // rasterizer handles that (and needs projected coords).
            for indices in &mesh.indices {
                let v0_local = mesh.vertices[indices[0]];
                let v1_local = mesh.vertices[indices[1]];
                let v2_local = mesh.vertices[indices[2]];

                let (v0_clip, w0) = mvp.transform_point(v0_local);
                let (v1_clip, w1) = mvp.transform_point(v1_local);
                let (v2_clip, w2) = mvp.transform_point(v2_local);

                triangle_batch.push(((v0_clip, w0), (v1_clip, w1), (v2_clip, w2), obj.color));
            }
        }

        // 4. Submit Batch
        if !triangle_batch.is_empty() {
            renderer.render_batch(fb, zb, &triangle_batch);
        }
    }
}
