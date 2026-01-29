//! 3D mesh representation.

use crate::math::Vec3;

/// A 3D mesh with vertices and triangle indices
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[usize; 3]>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Create a cube centered at origin
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

        Self { vertices, indices }
    }

    /// Compute face normal for each triangle
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

    /// Compute smooth vertex normals by averaging adjacent face normals
    pub fn compute_vertex_normals(&self) -> Vec<Vec3> {
        let face_normals = self.compute_face_normals();
        let mut vertex_normals = vec![Vec3::zero(); self.vertices.len()];

        // Accumulate face normals at each vertex
        for (face_idx, [i0, i1, i2]) in self.indices.iter().enumerate() {
            let normal = face_normals[face_idx];
            vertex_normals[*i0] = vertex_normals[*i0] + normal;
            vertex_normals[*i1] = vertex_normals[*i1] + normal;
            vertex_normals[*i2] = vertex_normals[*i2] + normal;
        }

        // Normalize each vertex normal
        vertex_normals.iter().map(|n| n.normalize()).collect()
    }

    /// Create a pyramid
    pub fn pyramid(base: f32, height: f32) -> Self {
        let h = base / 2.0;
        let vertices = vec![
            Vec3::new(0.0, height, 0.0), // 0: apex
            Vec3::new(-h, 0.0, h),       // 1
            Vec3::new(h, 0.0, h),        // 2
            Vec3::new(h, 0.0, -h),       // 3
            Vec3::new(-h, 0.0, -h),      // 4
        ];

        let indices = vec![
            // Sides
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 4],
            [0, 4, 1],
            // Base
            [1, 4, 3],
            [1, 3, 2],
        ];

        Self { vertices, indices }
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
