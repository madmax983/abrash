//! 3D mesh representation.

use crate::math::{Vec2, Vec3};

/// A 3D mesh with vertices and triangle indices
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[usize; 3]>,
    pub uvs: Vec<Vec2>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
        }
    }

    /// Load a mesh from an OBJ file content string
    /// Supports `v` (vertices) and `f` (faces).
    /// Triangulates quads.
    /// Handles `v/vt/vn` format (ignoring vt and vn for now).
    pub fn from_obj(obj_content: &str) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let uvs = Vec::new(); // UV parsing not requested for now, but kept empty

        for line in obj_content.lines() {
            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("v") => {
                    let x: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let z: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    vertices.push(Vec3::new(x, y, z));
                }
                Some("f") => {
                    let mut face_indices = Vec::new();
                    for part in parts {
                        // Handle v/vt/vn or v//vn or v
                        let index_str = part.split('/').next().unwrap_or("0");
                        if let Ok(idx) = index_str.parse::<isize>() {
                            // OBJ uses 1-based indexing.
                            let target_idx = if idx > 0 {
                                (idx as usize).checked_sub(1)
                            } else {
                                // Handle negative indices (relative to end of vertex list)
                                let len = vertices.len() as isize;
                                let abs_idx = len + idx;
                                if abs_idx >= 0 {
                                    Some(abs_idx as usize)
                                } else {
                                    None
                                }
                            };

                            if let Some(valid_idx) = target_idx.filter(|&i| i < vertices.len()) {
                                face_indices.push(valid_idx);
                            }
                        }
                    }

                    if face_indices.len() >= 3 {
                        // Triangulate fan (0, 1, 2), (0, 2, 3), ...
                        for i in 1..face_indices.len() - 1 {
                            indices.push([face_indices[0], face_indices[i], face_indices[i + 1]]);
                        }
                    }
                }
                _ => {} // Ignore comments and other unknown lines
            }
        }

        Self {
            vertices,
            indices,
            uvs,
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

        // Cube doesn't have UVs by default
        let uvs = Vec::new();

        Self {
            vertices,
            indices,
            uvs,
        }
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
        let mut vertex_normals = vec![Vec3::default(); self.vertices.len()];

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

        let uvs = Vec::new();

        Self {
            vertices,
            indices,
            uvs,
        }
    }

    /// Create a textured cube with duplicated vertices for proper UV mapping
    pub fn textured_cube(size: f32) -> Self {
        let h = size / 2.0;
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Helper to add a quad
        // v0: BL, v1: BR, v2: TR, v3: TL
        let mut add_quad = |v0: Vec3, v1: Vec3, v2: Vec3, v3: Vec3| {
            let base = vertices.len();
            vertices.push(v0);
            vertices.push(v1);
            vertices.push(v2);
            vertices.push(v3);

            // Standard UV mapping (0,0 bottom-left, 1,1 top-right)
            uvs.push(Vec2::new(0.0, 0.0));
            uvs.push(Vec2::new(1.0, 0.0));
            uvs.push(Vec2::new(1.0, 1.0));
            uvs.push(Vec2::new(0.0, 1.0));

            indices.push([base, base + 1, base + 2]);
            indices.push([base, base + 2, base + 3]);
        };

        // Front Face (+Z)
        add_quad(
            Vec3::new(-h, -h, h),
            Vec3::new(h, -h, h),
            Vec3::new(h, h, h),
            Vec3::new(-h, h, h),
        );

        // Back Face (-Z)
        add_quad(
            Vec3::new(h, -h, -h),  // 5: BL (from back view)
            Vec3::new(-h, -h, -h), // 4: BR
            Vec3::new(-h, h, -h),  // 7: TR
            Vec3::new(h, h, -h),   // 6: TL
        );

        // Top Face (+Y)
        add_quad(
            Vec3::new(-h, h, h),  // 3: BL
            Vec3::new(h, h, h),   // 2: BR
            Vec3::new(h, h, -h),  // 6: TR
            Vec3::new(-h, h, -h), // 7: TL
        );

        // Bottom Face (-Y)
        add_quad(
            Vec3::new(-h, -h, -h), // 4: BL
            Vec3::new(h, -h, -h),  // 5: BR
            Vec3::new(h, -h, h),   // 1: TR
            Vec3::new(-h, -h, h),  // 0: TL
        );

        // Right Face (+X)
        add_quad(
            Vec3::new(h, -h, h),  // 1: BL
            Vec3::new(h, -h, -h), // 5: BR
            Vec3::new(h, h, -h),  // 6: TR
            Vec3::new(h, h, h),   // 2: TL
        );

        // Left Face (-X)
        add_quad(
            Vec3::new(-h, -h, -h), // 4: BL
            Vec3::new(-h, -h, h),  // 0: BR
            Vec3::new(-h, h, h),   // 3: TR
            Vec3::new(-h, h, -h),  // 7: TL
        );

        Self {
            vertices,
            indices,
            uvs,
        }
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
