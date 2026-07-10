//! 3D Mesh representation.
//!
//! A mesh is a collection of vertices, indices (forming triangles), and optional UV coordinates.
//!
//! # Examples
//!
//! ```
//! use abrash_core::mesh::Mesh;
//! use abrash_core::math::Vec3;
//!
//! let mut mesh = Mesh::new();
//! mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
//! mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
//! mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
//! mesh.indices.push([0, 1, 2]);
//! ```

use crate::geometry::BoundingSphere;
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

impl Mesh {
    /// Creates a new empty mesh.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::mesh::Mesh;
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

    /// Creates a new empty mesh with pre-allocated capacity.
    ///
    /// Pre-allocating capacity avoids reallocations during mesh construction
    /// which improves performance when building large meshes procedurally.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::mesh::Mesh;
    /// let mesh = Mesh::with_capacity(100, 200);
    /// assert_eq!(mesh.vertices.capacity(), 100);
    /// assert_eq!(mesh.indices.capacity(), 200);
    /// ```
    #[must_use]
    pub fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(index_capacity),
            uvs: Vec::with_capacity(vertex_capacity),
            normals: Vec::with_capacity(vertex_capacity),
            tangents: Vec::with_capacity(vertex_capacity),
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
    /// use abrash_core::mesh::Mesh;
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

    /// Create a UV sphere centered at origin with the given radius.
    ///
    /// Generates a sphere with `stacks` horizontal rings and `sectors` vertical slices.
    /// Includes normals (unit sphere positions) and UV coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::mesh::Mesh;
    /// let sphere = Mesh::sphere(1.0, 16, 32);
    /// assert!(sphere.vertices.len() > 100);
    /// assert_eq!(sphere.normals.len(), sphere.vertices.len());
    /// ```
    #[must_use]
    pub fn sphere(radius: f32, stacks: u32, sectors: u32) -> Self {
        use std::f32::consts::PI;

        let num_vertices = ((stacks + 1) * (sectors + 1)) as usize;
        let num_indices = (stacks * sectors * 6) as usize;

        let mut vertices = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        for i in 0..=stacks {
            let stack_angle = PI / 2.0 - (i as f32 / stacks as f32) * PI; // π/2 to -π/2
            let (z, xy) = stack_angle.sin_cos();

            for j in 0..=sectors {
                let sector_angle = (j as f32 / sectors as f32) * 2.0 * PI;

                let (sin_sector, cos_sector) = sector_angle.sin_cos();
                let x = xy * cos_sector;
                let y = xy * sin_sector;

                vertices.push(Vec3::new(x * radius, z * radius, y * radius));
                normals.push(Vec3::new(x, z, y));
                uvs.push(Vec2::new(
                    j as f32 / sectors as f32,
                    i as f32 / stacks as f32,
                ));
            }
        }

        let row = sectors + 1;
        for i in 0..stacks {
            for j in 0..sectors {
                let k1 = i * row + j;
                let k2 = k1 + row;

                if i != 0 {
                    indices.push([k1 as usize, k2 as usize, (k1 + 1) as usize]);
                }
                if i != stacks - 1 {
                    indices.push([(k1 + 1) as usize, k2 as usize, (k2 + 1) as usize]);
                }
            }
        }

        Self {
            vertices,
            indices,
            uvs,
            normals,
            tangents: Vec::new(),
        }
    }

    /// Create a flat plane centered at origin in the XZ plane.
    ///
    /// The plane is subdivided into `subdivisions × subdivisions` quads.
    /// Normals point up (+Y). Includes UV coordinates \[0,1\].
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::mesh::Mesh;
    /// let plane = Mesh::plane(10.0, 4);
    /// assert_eq!(plane.vertices.len(), 25); // 5×5 grid
    /// assert_eq!(plane.indices.len(), 32);  // 4×4×2 triangles
    /// ```
    #[must_use]
    pub fn plane(size: f32, subdivisions: u32) -> Self {
        let half = size / 2.0;
        let divs = subdivisions.max(1);
        let step = size / divs as f32;

        let num_vertices = ((divs + 1) * (divs + 1)) as usize;
        let num_indices = (divs * divs * 6) as usize;

        let mut vertices = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        for z in 0..=divs {
            for x in 0..=divs {
                let px = -half + x as f32 * step;
                let pz = -half + z as f32 * step;
                vertices.push(Vec3::new(px, 0.0, pz));
                normals.push(Vec3::new(0.0, 1.0, 0.0));
                uvs.push(Vec2::new(x as f32 / divs as f32, z as f32 / divs as f32));
            }
        }

        let row = divs + 1;
        for z in 0..divs {
            for x in 0..divs {
                let i0 = (z * row + x) as usize;
                let i1 = i0 + 1;
                let i2 = ((z + 1) * row + x) as usize;
                let i3 = i2 + 1;
                indices.push([i0, i2, i1]);
                indices.push([i1, i2, i3]);
            }
        }

        Self {
            vertices,
            indices,
            uvs,
            normals,
            tangents: Vec::new(),
        }
    }

    /// Create a cylinder centered at origin along the Y axis.
    ///
    /// The cylinder has `sectors` vertical slices and `stacks` horizontal rings.
    /// Includes normals and UV coordinates. End caps are generated.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::mesh::Mesh;
    /// let cyl = Mesh::cylinder(0.5, 2.0, 16, 1);
    /// assert!(cyl.vertices.len() > 30);
    /// assert_eq!(cyl.normals.len(), cyl.vertices.len());
    /// ```
    #[must_use]
    pub fn cylinder(radius: f32, height: f32, sectors: u32, stacks: u32) -> Self {
        use std::f32::consts::PI;

        let half_h = height / 2.0;
        let sectors = sectors.max(3);
        let stacks = stacks.max(1);

        let body_vertices = (stacks + 1) * (sectors + 1);
        let cap_vertices = (sectors + 1) * 2;
        let num_vertices = (body_vertices + cap_vertices) as usize;

        let body_indices = stacks * sectors * 6;
        let cap_indices = sectors * 6;
        let num_indices = (body_indices + cap_indices) as usize;

        let mut vertices = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        for i in 0..=stacks {
            let y = -half_h + (i as f32 / stacks as f32) * height;
            let v = i as f32 / stacks as f32;

            for j in 0..=sectors {
                let angle = (j as f32 / sectors as f32) * 2.0 * PI;
                let (z, x) = angle.sin_cos();

                vertices.push(Vec3::new(x * radius, y, z * radius));
                normals.push(Vec3::new(x, 0.0, z));
                uvs.push(Vec2::new(j as f32 / sectors as f32, v));
            }
        }

        // Side indices
        let row = sectors + 1;
        for i in 0..stacks {
            for j in 0..sectors {
                let k1 = i * row + j;
                let k2 = k1 + row;
                indices.push([k1 as usize, k2 as usize, (k1 + 1) as usize]);
                indices.push([(k1 + 1) as usize, k2 as usize, (k2 + 1) as usize]);
            }
        }

        // Top cap
        let top_center = vertices.len();
        vertices.push(Vec3::new(0.0, half_h, 0.0));
        normals.push(Vec3::new(0.0, 1.0, 0.0));
        uvs.push(Vec2::new(0.5, 0.5));

        for j in 0..sectors {
            let angle = (j as f32 / sectors as f32) * 2.0 * PI;
            let (sin_angle, cos_angle) = angle.sin_cos();
            let idx = vertices.len();
            vertices.push(Vec3::new(cos_angle * radius, half_h, sin_angle * radius));
            normals.push(Vec3::new(0.0, 1.0, 0.0));
            uvs.push(Vec2::new(cos_angle * 0.5 + 0.5, sin_angle * 0.5 + 0.5));

            let next = if j + 1 < sectors {
                idx + 1
            } else {
                top_center + 1
            };
            indices.push([top_center, idx, next]);
        }

        // Bottom cap
        let bot_center = vertices.len();
        vertices.push(Vec3::new(0.0, -half_h, 0.0));
        normals.push(Vec3::new(0.0, -1.0, 0.0));
        uvs.push(Vec2::new(0.5, 0.5));

        for j in 0..sectors {
            let angle = (j as f32 / sectors as f32) * 2.0 * PI;
            let (sin_angle, cos_angle) = angle.sin_cos();
            let idx = vertices.len();
            vertices.push(Vec3::new(cos_angle * radius, -half_h, sin_angle * radius));
            normals.push(Vec3::new(0.0, -1.0, 0.0));
            uvs.push(Vec2::new(cos_angle * 0.5 + 0.5, sin_angle * 0.5 + 0.5));

            let next = if j + 1 < sectors {
                idx + 1
            } else {
                bot_center + 1
            };
            indices.push([bot_center, next, idx]); // reversed winding for bottom
        }

        Self {
            vertices,
            indices,
            uvs,
            normals,
            tangents: Vec::new(),
        }
    }

    /// Create a torus centered at origin in the XZ plane.
    ///
    /// `major_radius` is the distance from the center to the tube center.
    /// `minor_radius` is the tube radius. `major_segments` controls the ring count,
    /// `minor_segments` controls the tube cross-section detail.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::mesh::Mesh;
    /// let torus = Mesh::torus(1.0, 0.3, 24, 12);
    /// assert!(torus.vertices.len() > 200);
    /// assert_eq!(torus.normals.len(), torus.vertices.len());
    /// ```
    #[must_use]
    pub fn torus(
        major_radius: f32,
        minor_radius: f32,
        major_segments: u32,
        minor_segments: u32,
    ) -> Self {
        use std::f32::consts::PI;

        let maj = major_segments.max(3);
        let min = minor_segments.max(3);

        let num_vertices = ((maj + 1) * (min + 1)) as usize;
        let num_indices = (maj * min * 6) as usize;

        let mut vertices = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        for i in 0..=maj {
            let theta = (i as f32 / maj as f32) * 2.0 * PI;
            let (sin_t, cos_t) = theta.sin_cos();

            for j in 0..=min {
                let phi = (j as f32 / min as f32) * 2.0 * PI;
                let (sin_p, cos_p) = phi.sin_cos();

                // Vertex position
                let x = (major_radius + minor_radius * cos_p) * cos_t;
                let y = minor_radius * sin_p;
                let z = (major_radius + minor_radius * cos_p) * sin_t;

                // Normal (points outward from tube surface)
                let nx = cos_p * cos_t;
                let ny = sin_p;
                let nz = cos_p * sin_t;

                vertices.push(Vec3::new(x, y, z));
                normals.push(Vec3::new(nx, ny, nz));
                uvs.push(Vec2::new(i as f32 / maj as f32, j as f32 / min as f32));
            }
        }

        // Indices
        let row = min + 1;
        for i in 0..maj {
            for j in 0..min {
                let k1 = (i * row + j) as usize;
                let k2 = ((i + 1) * row + j) as usize;
                indices.push([k1, k2, k1 + 1]);
                indices.push([k1 + 1, k2, k2 + 1]);
            }
        }

        Self {
            vertices,
            indices,
            uvs,
            normals,
            tangents: Vec::new(),
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
    /// use abrash_core::mesh::Mesh;
    /// use abrash_core::math::Vec3;
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
    /// Computes and returns the face normals for each triangle in the mesh.
    #[must_use]
    pub fn compute_face_normals(&self) -> Vec<Vec3> {
        // ⚡ Bolt: Pre-allocate vector to eliminate dynamic heap reallocations
        // caused by Iterator::collect::<Vec<_>>() optimization failures on chained iterators.
        let mut normals = Vec::with_capacity(self.indices.len());
        normals.extend(self.indices.iter().map(|&[i0, i1, i2]| {
            let v0 = self.vertices[i0];
            let v1 = self.vertices[i1];
            let v2 = self.vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            edge1.cross(edge2).normalize()
        }));
        normals
    }

    /// Calculates the bounding sphere of the mesh.
    ///
    /// Uses a simple algorithm: Center is the average of min/max bounds (AABB center),
    /// and radius is the distance to the furthest vertex.
    ///
    /// Optimization: Uses `Vec3::min` and `Vec3::max` to leverage underlying fast
    /// floating point operations (`minss`/`maxss`) instead of branchy component-wise checks.
    /// This provides a small but measurable speedup for bounding box calculations on large meshes.
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

        for v in self.vertices.iter().skip(1) {
            min = min.min(*v);
            max = max.max(*v);
        }

        let center = (min + max) * 0.5;
        // Optimization: `f32::max` avoids branchy component-wise checks and utilizes
        // underlying fast float max instructions.
        let max_dist_sq = self.vertices.iter().fold(0.0_f32, |max_sq, v| {
            let d = *v - center;
            let dist_sq = d.x * d.x + d.y * d.y + d.z * d.z;
            max_sq.max(dist_sq)
        });

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

        self.tangents.clear();
        self.tangents.resize(self.vertices.len(), Vec4::default());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_mesh_new_and_default() {
        let mesh = Mesh::new();
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
        assert!(mesh.uvs.is_empty());
        assert!(mesh.normals.is_empty());
        assert!(mesh.tangents.is_empty());

        let default_mesh = Mesh::default();
        assert!(default_mesh.vertices.is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_mesh_with_capacity() {
        let mesh = Mesh::with_capacity(10, 20);
        assert!(mesh.vertices.capacity() >= 10);
        assert!(mesh.indices.capacity() >= 20);
        assert!(mesh.uvs.capacity() >= 10);
        assert!(mesh.normals.capacity() >= 10);
        assert!(mesh.tangents.capacity() >= 10);
    }
}

#[cfg(test)]
mod tests_generated {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn cube() {
        let size = 2.0;
        let cube = Mesh::cube(size);

        // A cube has 6 faces * 4 vertices/face = 24 vertices
        assert_eq!(cube.vertices.len(), 8);
        assert_eq!(cube.normals.len(), 0);
        assert_eq!(cube.uvs.len(), 0);

        // 6 faces * 2 triangles/face = 12 triangles
        assert_eq!(cube.indices.len(), 12);

        // Check bounds
        let bounds = cube.calculate_bounding_sphere();
        assert!((bounds.center.x).abs() < 1e-5);
        assert!((bounds.center.y).abs() < 1e-5);
        assert!((bounds.center.z).abs() < 1e-5);

        // Expected distance from center (0,0,0) to corner (1,1,1) is sqrt(3) ~ 1.732
        let expected_radius = (size / 2.0) * 3.0_f32.sqrt();
        assert!((bounds.radius - expected_radius).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn sphere() {
        let sphere = Mesh::sphere(1.0, 10, 10);

        // Vertices = (stacks + 1) * (sectors + 1)
        assert_eq!(sphere.vertices.len(), 11 * 11);
        assert_eq!(sphere.normals.len(), 11 * 11);
        assert_eq!(sphere.uvs.len(), 11 * 11);

        // Triangles = stacks * sectors * 2
        assert_eq!(sphere.indices.len(), 180); // Polar caps have triangles, body has quads (2 triangles)

        let bounds = sphere.calculate_bounding_sphere();
        assert!((bounds.center.x).abs() < 1e-5);
        assert!((bounds.center.y).abs() < 1e-5);
        assert!((bounds.center.z).abs() < 1e-5);
        assert!((bounds.radius - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn plane() {
        let plane = Mesh::plane(10.0, 2);

        // Vertices = (subdivisions + 1) * (subdivisions + 1)
        assert_eq!(plane.vertices.len(), 3 * 3);
        assert_eq!(plane.normals.len(), 3 * 3);
        assert_eq!(plane.uvs.len(), 3 * 3);

        // Triangles = subdivisions * subdivisions * 2
        assert_eq!(plane.indices.len(), 2 * 2 * 2);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn cylinder() {
        let cylinder = Mesh::cylinder(1.0, 2.0, 10, 5);

        // Side vertices: (stacks + 1) * (sectors + 1) = 6 * 11 = 66
        // Top cap vertices: 1 (center) + sectors = 1 + 10 = 11
        // Bottom cap vertices: 1 (center) + sectors = 1 + 10 = 11
        // Total vertices = 66 + 11 + 11 = 88
        assert_eq!(cylinder.vertices.len(), 88);
        assert_eq!(cylinder.normals.len(), 88);
        assert_eq!(cylinder.uvs.len(), 88);

        // Side indices: stacks * sectors * 2 = 5 * 10 * 2 = 100
        // Top cap indices: sectors = 10
        // Bottom cap indices: sectors = 10
        // Total indices = 100 + 10 + 10 = 120
        assert_eq!(cylinder.indices.len(), 120);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn torus() {
        let torus = Mesh::torus(2.0, 0.5, 10, 10);

        // Vertices: (major_segments + 1) * (minor_segments + 1) = 11 * 11 = 121
        assert_eq!(torus.vertices.len(), 121);
        assert_eq!(torus.normals.len(), 121);
        assert_eq!(torus.uvs.len(), 121);

        // Indices: major_segments * minor_segments * 2 = 10 * 10 * 2 = 200
        assert_eq!(torus.indices.len(), 200);
    }
}

#[cfg(test)]
mod tests_normals {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_face_normals_simple_triangle() {
        let mut mesh = Mesh::new();
        // Counter-clockwise triangle should point towards +Z according to right hand rule
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        let normals = mesh.compute_face_normals();
        assert_eq!(normals.len(), 1);

        let n = normals[0];
        assert!((n.x).abs() < 1e-5);
        assert!((n.y).abs() < 1e-5);
        assert!((n.z - 1.0).abs() < 1e-5); // Points exactly along +Z
    }
}

#[cfg(test)]
mod tests_bounding_sphere {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn calculate_bounding_sphere_empty() {
        let mesh = Mesh::new();
        let bounds = mesh.calculate_bounding_sphere();
        assert_eq!(bounds.center, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(bounds.radius, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn calculate_bounding_sphere_single_vertex() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(1.0, 2.0, 3.0));
        let bounds = mesh.calculate_bounding_sphere();
        assert_eq!(bounds.center, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bounds.radius, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn calculate_bounding_sphere_two_vertices() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(-1.0, -1.0, -1.0));
        mesh.vertices.push(Vec3::new(1.0, 1.0, 1.0));
        let bounds = mesh.calculate_bounding_sphere();

        assert_eq!(bounds.center, Vec3::new(0.0, 0.0, 0.0));
        let expected_radius = 3.0_f32.sqrt();
        assert!((bounds.radius - expected_radius).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_tangents {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_tangents_missing_data_early_exit() {
        let mut mesh = Mesh::new();
        // Missing uvs
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.normals.push(Vec3::new(0.0, 0.0, 1.0));
        mesh.compute_tangents();
        assert!(mesh.tangents.is_empty());

        // Missing normals
        mesh.uvs.push(Vec2::new(0.0, 0.0));
        mesh.normals.clear();
        mesh.compute_tangents();
        assert!(mesh.tangents.is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_tangents_simple_quad() {
        let mut mesh = Mesh::new();

        // Quad mapping 0..1 in X,Y to 0..1 in U,V
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, 1.0, 0.0));
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));

        mesh.uvs.push(Vec2::new(0.0, 0.0));
        mesh.uvs.push(Vec2::new(1.0, 0.0));
        mesh.uvs.push(Vec2::new(1.0, 1.0));
        mesh.uvs.push(Vec2::new(0.0, 1.0));

        mesh.normals.push(Vec3::new(0.0, 0.0, 1.0));
        mesh.normals.push(Vec3::new(0.0, 0.0, 1.0));
        mesh.normals.push(Vec3::new(0.0, 0.0, 1.0));
        mesh.normals.push(Vec3::new(0.0, 0.0, 1.0));

        mesh.indices.push([0, 1, 2]);
        mesh.indices.push([0, 2, 3]);

        mesh.compute_tangents();

        assert_eq!(mesh.tangents.len(), 4);

        for tangent in mesh.tangents {
            // Because U aligns with X, tangent should be +X
            assert!((tangent.x - 1.0).abs() < 1e-5);
            assert!((tangent.y).abs() < 1e-5);
            assert!((tangent.z).abs() < 1e-5);
            // Handedness component should be valid (+1.0 or -1.0)
            assert!((tangent.w.abs() - 1.0).abs() < 1e-5);
        }
    }
}
