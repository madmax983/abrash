//! 3D Mesh representation.
//!
//! A mesh is a collection of vertices, indices (forming triangles), and optional UV coordinates.
//!
//! # Examples
//!
//! ```
//! use abrash::geometry::mesh::Mesh;
//! use abrash::math::Vec3;
//!
//! let mut mesh = Mesh::new();
//! mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
//! mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
//! mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
//! mesh.indices.push([0, 1, 2]);
//! ```

use crate::math::{Vec2, Vec3, Vec4};

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
    /// List of vertex normals.
    pub normals: Vec<Vec3>,
    /// List of vertex tangents (xyz + handedness w).
    pub tangents: Vec<Vec4>,
}

/// A Bounding Sphere for object-level culling.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// Axis-Aligned Bounding Box (AABB) for object-level culling.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub pad0: f32, // Padding to align max to 16 bytes offset
    pub max: Vec3,
    pub pad1: f32, // Padding to make total size 32 bytes
}

impl AABB {
    /// Create a new AABB from min and max points.
    #[must_use]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            pad0: 0.0,
            max,
            pad1: 0.0,
        }
    }

    /// Calculate AABB from a list of points.
    #[must_use]
    pub fn from_points(points: &[Vec3]) -> Self {
        if points.is_empty() {
            return Self {
                min: Vec3::default(),
                pad0: 0.0,
                max: Vec3::default(),
                pad1: 0.0,
            };
        }

        let mut min = points[0];
        let mut max = points[0];

        for &p in points.iter().skip(1) {
            if p.x < min.x {
                min.x = p.x;
            }
            if p.y < min.y {
                min.y = p.y;
            }
            if p.z < min.z {
                min.z = p.z;
            }

            if p.x > max.x {
                max.x = p.x;
            }
            if p.y > max.y {
                max.y = p.y;
            }
            if p.z > max.z {
                max.z = p.z;
            }
        }

        Self {
            min,
            pad0: 0.0,
            max,
            pad1: 0.0,
        }
    }

    /// Get the center of the AABB.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the extents (half-size) of the AABB.
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }
}

impl Mesh {
    /// Creates a new empty mesh.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::geometry::mesh::Mesh;
    /// let mesh = Mesh::new();
    /// assert!(mesh.vertices.is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
            normals: Vec::new(),
            tangents: Vec::new(),
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
    /// use abrash::geometry::mesh::Mesh;
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
        let normals = Vec::new();
        let tangents = Vec::new();

        Self {
            vertices,
            indices,
            uvs,
            normals,
            tangents,
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
    /// use abrash::geometry::mesh::Mesh;
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

    /// Calculates the bounding sphere of the mesh.
    ///
    /// Uses a simple algorithm: Center is the average of min/max bounds (AABB center),
    /// and radius is the distance to the furthest vertex.
    #[must_use]
    pub fn calculate_bounding_sphere(&self) -> BoundingSphere {
        if self.vertices.is_empty() {
            return BoundingSphere {
                center: Vec3::default(),
                radius: 0.0,
            };
        }

        let mut min = self.vertices[0];
        let mut max = self.vertices[0];

        for v in &self.vertices {
            if v.x < min.x {
                min.x = v.x;
            }
            if v.y < min.y {
                min.y = v.y;
            }
            if v.z < min.z {
                min.z = v.z;
            }
            if v.x > max.x {
                max.x = v.x;
            }
            if v.y > max.y {
                max.y = v.y;
            }
            if v.z > max.z {
                max.z = v.z;
            }
        }

        let center = (min + max) * 0.5;
        let mut max_dist_sq = 0.0;

        for v in &self.vertices {
            let d = *v - center;
            let dist_sq = d.x * d.x + d.y * d.y + d.z * d.z;
            if dist_sq > max_dist_sq {
                max_dist_sq = dist_sq;
            }
        }

        BoundingSphere {
            center,
            radius: max_dist_sq.sqrt(),
        }
    }

    /// Compute vertex tangents for normal mapping.
    ///
    /// Requires `vertices`, `uvs`, and `normals` to be populated.
    /// Populates `self.tangents`.
    pub fn compute_tangents(&mut self) {
        if self.uvs.is_empty() || self.normals.is_empty() {
            return;
        }

        let mut tan1 = vec![Vec3::default(); self.vertices.len()];
        let mut tan2 = vec![Vec3::default(); self.vertices.len()];

        for &[i0, i1, i2] in &self.indices {
            let v0 = self.vertices[i0];
            let v1 = self.vertices[i1];
            let v2 = self.vertices[i2];

            let w0 = self.uvs[i0];
            let w1 = self.uvs[i1];
            let w2 = self.uvs[i2];

            let x1 = v1.x - v0.x;
            let x2 = v2.x - v0.x;
            let y1 = v1.y - v0.y;
            let y2 = v2.y - v0.y;
            let z1 = v1.z - v0.z;
            let z2 = v2.z - v0.z;

            let s1 = w1.x - w0.x;
            let s2 = w2.x - w0.x;
            let t1 = w1.y - w0.y;
            let t2 = w2.y - w0.y;

            let r = 1.0 / (s1 * t2 - s2 * t1);
            let sdir = Vec3::new(
                (t2 * x1 - t1 * x2) * r,
                (t2 * y1 - t1 * y2) * r,
                (t2 * z1 - t1 * z2) * r,
            );
            let tdir = Vec3::new(
                (s1 * x2 - s2 * x1) * r,
                (s1 * y2 - s2 * y1) * r,
                (s1 * z2 - s2 * z1) * r,
            );

            tan1[i0] = tan1[i0] + sdir;
            tan1[i1] = tan1[i1] + sdir;
            tan1[i2] = tan1[i2] + sdir;

            tan2[i0] = tan2[i0] + tdir;
            tan2[i1] = tan2[i1] + tdir;
            tan2[i2] = tan2[i2] + tdir;
        }

        self.tangents = vec![Vec4::default(); self.vertices.len()];
        for i in 0..self.vertices.len() {
            let n = self.normals[i];
            let t = tan1[i];

            // Gram-Schmidt orthogonalize
            let tangent_xyz = (t - n * n.dot(t)).normalize();

            // Calculate handedness
            let w = if n.cross(t).dot(tan2[i]) < 0.0 {
                -1.0
            } else {
                1.0
            };

            self.tangents[i] = Vec4::new(tangent_xyz.x, tangent_xyz.y, tangent_xyz.z, w);
        }
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
