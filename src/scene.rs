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

    /// Calculate the World Space AABB by transforming the local AABB.
    ///
    /// Optimization: Uses "Transforming Axis-Aligned Bounding Boxes" by Jim Arvo (Graphics Gems)
    /// to avoid transforming all 8 corners.
    ///
    /// $$ C' = M \cdot C + T $$
    /// $$ E' = |M| \cdot E $$
    ///
    /// Note: This assumes the transform is affine (W=1.0).
    pub fn calculate_world_aabb(&self) -> AABB {
        let center = self.local_aabb.center();
        let extents = self.local_aabb.extents();

        // 1. Transform Center
        let (new_center, _) = self.transform.transform_point(center);

        // 2. Transform Extents (using absolute values of matrix elements)
        // Row-Vector convention: v' = v * M
        // x' = x*m00 + y*m10 + z*m20 + m30
        // So x depends on column 0.
        let m = self.transform.m;

        let ex = extents.x * m[0][0].abs() + extents.y * m[1][0].abs() + extents.z * m[2][0].abs();
        let ey = extents.x * m[0][1].abs() + extents.y * m[1][1].abs() + extents.z * m[2][1].abs();
        let ez = extents.x * m[0][2].abs() + extents.y * m[1][2].abs() + extents.z * m[2][2].abs();

        let new_extents = Vec3::new(ex, ey, ez);

        AABB::new(new_center - new_extents, new_center + new_extents)
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

        // Process in chunks to enable SIMD culling
        // Batch size of 32 is a good balance for AVX2 (processes 8 at a time)
        const CHUNK_SIZE: usize = 32;
        let mut aabb_buffer = [AABB {
            min: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            pad0: 0.0,
            max: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            pad1: 0.0,
        }; CHUNK_SIZE];
        let mut results_buffer = [false; CHUNK_SIZE];

        for chunk in self.objects.chunks(CHUNK_SIZE) {
            let count = chunk.len();

            // 1. Calculate World AABBs for the chunk
            for (i, obj) in chunk.iter().enumerate() {
                aabb_buffer[i] = obj.calculate_world_aabb();
            }

            // 2. Batch Cull (SIMD)
            self.camera
                .frustum
                .cull_aabbs_prealloc(&aabb_buffer[0..count], &mut results_buffer[0..count]);

            // 3. Process Visible Objects
            for (i, obj) in chunk.iter().enumerate() {
                if results_buffer[i] {
                    let mvp = obj.transform * view_proj;
                    let mesh = &obj.mesh;

                    for indices in &mesh.indices {
                        let v0_local = mesh.vertices[indices[0]];
                        let v1_local = mesh.vertices[indices[1]];
                        let v2_local = mesh.vertices[indices[2]];

                        let (v0_clip, w0) = mvp.transform_point(v0_local);
                        let (v1_clip, w1) = mvp.transform_point(v1_local);
                        let (v2_clip, w2) = mvp.transform_point(v2_local);

                        triangle_batch.push((
                            (v0_clip, w0),
                            (v1_clip, w1),
                            (v2_clip, w2),
                            obj.color,
                        ));
                    }
                }
            }
        }

        // 4. Submit Batch
        if !triangle_batch.is_empty() {
            renderer.render_batch(fb, zb, &triangle_batch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SceneObject;
    use crate::math::{Mat4, Vec3};
    use crate::mesh::Mesh;
    use std::sync::Arc;

    #[test]
    fn test_calculate_world_aabb_optimized() {
        // Setup a test object
        let min = Vec3::new(-1.0, -1.0, -1.0);
        let max = Vec3::new(1.0, 1.0, 1.0);

        let mut mesh = Mesh::new();
        mesh.vertices.push(min);
        mesh.vertices.push(max);

        let mesh_arc = Arc::new(mesh);

        // 1. Test Identity
        let obj_identity = SceneObject::new(mesh_arc.clone(), Mat4::identity(), 0);
        let aabb_id = obj_identity.calculate_world_aabb();

        assert!((aabb_id.min.x - min.x).abs() < 1e-5);
        assert!((aabb_id.max.x - max.x).abs() < 1e-5);

        // 2. Test Translation
        let obj_trans = SceneObject::new(mesh_arc.clone(), Mat4::translation(10.0, 0.0, 0.0), 0);
        let aabb_trans = obj_trans.calculate_world_aabb();
        assert!((aabb_trans.min.x - 9.0).abs() < 1e-5);
        assert!((aabb_trans.max.x - 11.0).abs() < 1e-5);

        // 3. Test Scale
        let obj_scale = SceneObject::new(mesh_arc.clone(), Mat4::scale(2.0, 2.0, 2.0), 0);
        let aabb_scale = obj_scale.calculate_world_aabb();
        assert!((aabb_scale.min.x - (-2.0)).abs() < 1e-5);
        assert!((aabb_scale.max.x - 2.0).abs() < 1e-5);

        // 4. Test Rotation (45 deg Y)
        // Local box is [-1, 1]x[-1, 1]x[-1, 1].
        // Rotated 45 deg Y, new extents should be sqrt(2) on X and Z.
        let rot = Mat4::rotation_y(std::f32::consts::FRAC_PI_4);
        let obj_rot = SceneObject::new(mesh_arc, rot, 0);
        let aabb_rot = obj_rot.calculate_world_aabb();

        let expected_extent = 2.0f32.sqrt();
        assert!(
            (aabb_rot.max.x - expected_extent).abs() < 1e-5,
            "Got {}",
            aabb_rot.max.x
        );
        assert!(
            (aabb_rot.max.z - expected_extent).abs() < 1e-5,
            "Got {}",
            aabb_rot.max.z
        );
        assert!((aabb_rot.min.x - -expected_extent).abs() < 1e-5);
    }
}
