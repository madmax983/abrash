//! 3D Mesh representation.
//!
//! A mesh is a collection of vertices, indices (forming triangles), and optional UV coordinates.
//!
//! # Examples
//!
//! ```
//! use abrash::mesh::Mesh;
//! use abrash::math::Vec3;
//!
//! let mut mesh = Mesh::new();
//! mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
//! mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
//! mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
//! mesh.indices.push([0, 1, 2]);
//! ```

use crate::math::{Vec2, Vec3};

/// A 3D mesh with vertices and triangle indices
#[derive(Debug, Clone)]
pub struct Mesh {
    /// List of 3D vertices (x, y, z).
    pub vertices: Vec<Vec3>,
    /// List of triangles, each defined by 3 indices into `vertices`.
    pub indices: Vec<[usize; 3]>,
    /// List of texture coordinates (u, v) for each vertex.
    /// If present, must have same length as `vertices`.
    pub uvs: Vec<Vec2>,
}

impl Mesh {
    /// Creates a new empty mesh.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::mesh::Mesh;
    /// let mesh = Mesh::new();
    /// assert!(mesh.vertices.is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
        }
    }

    /// Create a cube centered at origin with side length `size`.
    ///
    /// The cube has 8 vertices and 12 triangles (2 per face).
    /// Does not include UV coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::mesh::Mesh;
    /// let cube = Mesh::cube(2.0);
    /// assert_eq!(cube.vertices.len(), 8);
    /// assert_eq!(cube.indices.len(), 12);
    /// ```
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

        Self {
            vertices,
            indices,
            uvs,
        }
    }

    /// Compute face normal for each triangle.
    ///
    /// Returns a vector of normals, one per triangle (in `indices` order).
    /// Normals are normalized.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::mesh::Mesh;
    /// use abrash::math::Vec3;
    ///
    /// let mut mesh = Mesh::new();
    /// mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    /// mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    /// mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
    /// mesh.indices.push([0, 1, 2]);
    ///
    /// let normals = mesh.compute_face_normals();
    /// assert_eq!(normals.len(), 1);
    /// assert_eq!(normals[0], Vec3::new(0.0, 0.0, 1.0));
    /// ```
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
