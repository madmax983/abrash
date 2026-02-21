//! Procedural Mesh Generation
//!
//! Tools for generating 3D meshes algorithmically, such as terrains.

#![allow(warnings)]

use crate::geometry::mesh::Mesh;
use crate::math::{Vec2, Vec3, Vec4};
use crate::utils::XorShift32;

/// A generator for procedural terrain meshes.
pub struct TerrainGenerator;

impl TerrainGenerator {
    /// Generates a flat plane mesh centered at (0,0,0) on the XZ plane.
    ///
    /// # Arguments
    ///
    /// * `width` - Total width along the X axis.
    /// * `depth` - Total depth along the Z axis.
    /// * `subdivisions` - Number of grid cells along each axis (must be > 0).
    ///
    /// The resulting mesh has `(subdivisions + 1)^2` vertices and `subdivisions^2 * 2` triangles.
    /// UV coordinates are mapped from (0,0) to (1,1).
    #[must_use]
    pub fn generate_plane(width: f32, depth: f32, subdivisions: u32) -> Mesh {
        let vertex_count = (subdivisions + 1) * (subdivisions + 1);
        let mut vertices = Vec::with_capacity(vertex_count as usize);
        let mut uvs = Vec::with_capacity(vertex_count as usize);
        let mut indices = Vec::with_capacity((subdivisions * subdivisions * 2) as usize);

        let half_width = width * 0.5;
        let half_depth = depth * 0.5;

        let dx = width / subdivisions as f32;
        let dz = depth / subdivisions as f32;
        let du = 1.0 / subdivisions as f32;
        let dv = 1.0 / subdivisions as f32;

        for z in 0..=subdivisions {
            let z_pos = -half_depth + z as f32 * dz;
            let v_coord = z as f32 * dv;

            for x in 0..=subdivisions {
                let x_pos = -half_width + x as f32 * dx;
                let u_coord = x as f32 * du;

                vertices.push(Vec3::new(x_pos, 0.0, z_pos));
                uvs.push(Vec2::new(u_coord, v_coord));
            }
        }

        for z in 0..subdivisions {
            for x in 0..subdivisions {
                let row1 = z * (subdivisions + 1);
                let row2 = (z + 1) * (subdivisions + 1);

                // Triangle 1
                // v0 -- v1
                // |  /
                // v2
                // Triangle 2
                //    v1
                //  /  |
                // v2 -- v3

                let v0 = (row1 + x) as usize;
                let v1 = (row1 + x + 1) as usize;
                let v2 = (row2 + x) as usize;
                let v3 = (row2 + x + 1) as usize;

                indices.push([v0, v2, v1]);
                indices.push([v1, v2, v3]);
            }
        }

        // Default flat normals
        let normals = vec![Vec3::new(0.0, 1.0, 0.0); vertices.len()];
        let tangents = vec![Vec4::default(); vertices.len()];

        let mut mesh = Mesh {
            vertices,
            indices,
            uvs,
            normals,
            tangents,
        };

        // Compute initial tangents for the flat plane
        mesh.compute_tangents();
        mesh
    }

    /// Applies a heightmap function to modify the Y coordinate of the mesh vertices.
    ///
    /// This also recalculates vertex normals and tangents based on the new geometry.
    ///
    /// # Arguments
    ///
    /// * `mesh` - The mesh to modify.
    /// * `func` - A function that takes (x, z) coordinates and returns the height (y).
    pub fn apply_heightmap<F>(mesh: &mut Mesh, func: F)
    where
        F: Fn(f32, f32) -> f32,
    {
        // 1. Update positions
        for v in &mut mesh.vertices {
            v.y = func(v.x, v.z);
        }

        // 2. Recompute Normals (Weighted averaging of face normals)
        let mut new_normals = vec![Vec3::default(); mesh.vertices.len()];

        for tri in &mesh.indices {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            let v0 = mesh.vertices[i0];
            let v1 = mesh.vertices[i1];
            let v2 = mesh.vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            // Cross product order matters for winding.
            // Standard CCW: (v1-v0) x (v2-v0) should point UP.
            let normal = edge1.cross(edge2).normalize();

            new_normals[i0] = new_normals[i0] + normal;
            new_normals[i1] = new_normals[i1] + normal;
            new_normals[i2] = new_normals[i2] + normal;
        }

        for n in &mut new_normals {
            *n = n.normalize();
        }
        mesh.normals = new_normals;

        // 3. Recompute Tangents
        mesh.compute_tangents();
    }
}

/// A simple value noise function for generating terrain heights.
///
/// Uses `XorShift32` hashing to generate deterministic noise.
///
/// # Arguments
///
/// * `x` - X coordinate.
/// * `z` - Z coordinate.
/// * `seed` - Random seed.
///
/// # Returns
///
/// A float between 0.0 and 1.0.
#[must_use]
pub fn noise(x: f32, z: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;

    let xf = x - x.floor();
    let zf = z - z.floor();

    // Smoothstep interpolation curves
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = zf * zf * (3.0 - 2.0 * zf);

    let hash = |x: i32, z: i32| -> f32 {
        // Simple hash mixing
        let n = (x as u32).wrapping_mul(73856093) ^ (z as u32).wrapping_mul(19349663);
        let mut rng = XorShift32::new(seed.wrapping_add(n));
        rng.next_f32()
    };

    let bl = hash(xi, zi);
    let br = hash(xi + 1, zi);
    let tl = hash(xi, zi + 1);
    let tr = hash(xi + 1, zi + 1);

    // Bilinear interpolation
    let b = bl + (br - bl) * u;
    let t = tl + (tr - tl) * u;

    b + (t - b) * v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_generation() {
        let mesh = TerrainGenerator::generate_plane(10.0, 10.0, 2);

        // (2+1)^2 = 9 vertices
        assert_eq!(mesh.vertices.len(), 9);
        // 2*2*2 = 8 triangles
        assert_eq!(mesh.indices.len(), 8);

        // Check corners
        // Top-left (-5, -5)
        let v0 = mesh.vertices[0];
        assert_eq!(v0.x, -5.0);
        assert_eq!(v0.z, -5.0);

        // Normals should be up
        assert_eq!(mesh.normals[0], Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_noise_consistency() {
        let val1 = noise(1.5, 2.5, 12345);
        let val2 = noise(1.5, 2.5, 12345);
        assert_eq!(val1, val2);

        let val3 = noise(1.5, 2.5, 54321);
        assert_ne!(val1, val3);
    }

    #[test]
    fn test_heightmap_application() {
        let mut mesh = TerrainGenerator::generate_plane(10.0, 10.0, 2);
        TerrainGenerator::apply_heightmap(&mut mesh, |x, z| x + z);

        for v in &mesh.vertices {
            assert!((v.y - (v.x + v.z)).abs() < 1e-5);
        }

        // Normals should have changed from (0,1,0)
        // Center vertex
        let n = mesh.normals[4];
        assert!(n.y < 0.99); // It's no longer flat up
        assert!(n.y > 0.0); // But still generally up-ish
    }
}
