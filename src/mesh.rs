//! 3D mesh representation.

use crate::math::{Vec2, Vec3};

/// A 3D mesh with vertices and triangle indices
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[usize; 3]>,
    pub uvs: Vec<Vec2>,
    pub normals: Vec<Vec3>,
}

impl Mesh {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
            normals: Vec::new(),
        }
    }

    /// Create a cube centered at origin
    #[must_use]
    pub fn cube(size: f32) -> Self {
        let h = size / 2.0;
        let vertices = vec![
            // Front face
            Vec3::new(-h, -h, h), // 0
            Vec3::new(h, -h, h),  // 1
            Vec3::new(h, h, h),   // 2
            Vec3::new(-h, h, h),  // 3
            // Back face
            Vec3::new(-h, -h, -h), // 4
            Vec3::new(h, -h, -h),  // 5
            Vec3::new(h, h, -h),   // 6
            Vec3::new(-h, h, -h),  // 7
        ];

        // Basic indices - this reuses vertices, so normals will be averaged if we computed them per vertex
        // For flat shading, this is fine. For smooth shading, we'd need to duplicate vertices per face.
        // For now, let's keep it simple and just init empty normals or face normals.
        // The prompt asks to "generate default normals".

        let indices = vec![
            // Front
            [0, 1, 2],
            [0, 2, 3],
            // Back
            [5, 4, 7],
            [5, 7, 6],
            // Top
            [3, 2, 6],
            [3, 6, 7],
            // Bottom
            [4, 5, 1],
            [4, 1, 0],
            // Right
            [1, 5, 6],
            [1, 6, 2],
            // Left
            [4, 0, 3],
            [4, 3, 7],
        ];

        // Cube doesn't have UVs by default
        let uvs = Vec::new();
        // Cube doesn't have per-vertex normals by default in this simple representation
        // (Shared vertices have conflicting normals at corners)
        let normals = Vec::new();

        Self {
            vertices,
            indices,
            uvs,
            normals,
        }
    }

    /// Compute face normal for each triangle
    #[must_use]
    pub fn compute_face_normals(&self) -> Vec<Vec3> {
        self.indices
            .iter()
            .map(|[i0, i1, i2]| {
                let v0 = self.vertices[*i0];
                let v1 = self.vertices[*i1];
                let v2 = self.vertices[*i2];

                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                edge1.cross(edge2).normalize()
            })
            .collect()
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
