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
            Vec3::new(-h, -h,  h), // 0
            Vec3::new( h, -h,  h), // 1
            Vec3::new( h,  h,  h), // 2
            Vec3::new(-h,  h,  h), // 3
            // Back face
            Vec3::new(-h, -h, -h), // 4
            Vec3::new( h, -h, -h), // 5
            Vec3::new( h,  h, -h), // 6
            Vec3::new(-h,  h, -h), // 7
        ];

        let indices = vec![
            // Front
            [0, 1, 2], [0, 2, 3],
            // Back
            [5, 4, 7], [5, 7, 6],
            // Top
            [3, 2, 6], [3, 6, 7],
            // Bottom
            [4, 5, 1], [4, 1, 0],
            // Right
            [1, 5, 6], [1, 6, 2],
            // Left
            [4, 0, 3], [4, 3, 7],
        ];

        Self { vertices, indices }
    }

    /// Create a pyramid
    pub fn pyramid(base: f32, height: f32) -> Self {
        let h = base / 2.0;
        let vertices = vec![
            Vec3::new( 0.0, height, 0.0), // 0: apex
            Vec3::new(-h,   0.0,    h),   // 1
            Vec3::new( h,   0.0,    h),   // 2
            Vec3::new( h,   0.0,   -h),   // 3
            Vec3::new(-h,   0.0,   -h),   // 4
        ];

        let indices = vec![
            // Sides
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 4],
            [0, 4, 1],
            // Base
            [1, 4, 3], [1, 3, 2],
        ];

        Self { vertices, indices }
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
