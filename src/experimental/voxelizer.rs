//! Voxelizer Module
//!
//! Converts a mesh into a voxel grid using stochastic sampling.
//! Can also generate a mesh representation of the voxel grid (cubes).

use crate::geometry::AABB;
use crate::math::Vec3;
use crate::mesh::Mesh;
use crate::utils::XorShift32;

/// A grid of voxels representing a 3D volume.
#[derive(Debug, Clone)]
pub struct VoxelGrid {
    /// Width of the grid (X axis).
    pub width: usize,
    /// Height of the grid (Y axis).
    pub height: usize,
    /// Depth of the grid (Z axis).
    pub depth: usize,
    /// Size of each voxel in world units.
    pub voxel_size: f32,
    /// Origin of the grid (min corner).
    pub origin: Vec3,
    /// Flat array of voxel data (true = set, false = empty).
    /// Index = z * width * height + y * width + x.
    pub data: Vec<bool>,
}

impl VoxelGrid {
    /// Creates a new empty voxel grid.
    #[must_use]
    pub fn new(width: usize, height: usize, depth: usize, voxel_size: f32, origin: Vec3) -> Self {
        let size = width * height * depth;
        Self {
            width,
            height,
            depth,
            voxel_size,
            origin,
            data: vec![false; size],
        }
    }

    /// Sets a voxel at the given grid coordinates.
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: bool) {
        if x < self.width && y < self.height && z < self.depth {
            let index = z * self.width * self.height + y * self.width + x;
            self.data[index] = value;
        }
    }

    /// Gets a voxel at the given grid coordinates.
    #[must_use]
    pub fn get(&self, x: usize, y: usize, z: usize) -> bool {
        if x < self.width && y < self.height && z < self.depth {
            let index = z * self.width * self.height + y * self.width + x;
            self.data[index]
        } else {
            false
        }
    }

    /// Converts the voxel grid back into a mesh where each set voxel is a cube.
    #[must_use]
    pub fn to_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new();
        let half_size = self.voxel_size * 0.5;

        for z in 0..self.depth {
            for y in 0..self.height {
                for x in 0..self.width {
                    if self.get(x, y, z) {
                        let center = self.origin
                            + Vec3::new(
                                (x as f32 + 0.5) * self.voxel_size,
                                (y as f32 + 0.5) * self.voxel_size,
                                (z as f32 + 0.5) * self.voxel_size,
                            );

                        // Generate a cube at `center` with size `voxel_size`
                        // We can optimize this by only generating visible faces, but full cubes for now.
                        Self::add_cube(&mut mesh, center, half_size);
                    }
                }
            }
        }

        let _ = mesh.compute_face_normals(); // Or compute vertex normals if needed
        mesh
    }

    fn add_cube(mesh: &mut Mesh, center: Vec3, h: f32) {
        let base_idx = mesh.vertices.len();

        // Vertices
        mesh.vertices.push(center + Vec3::new(-h, -h, h)); // 0
        mesh.vertices.push(center + Vec3::new(h, -h, h)); // 1
        mesh.vertices.push(center + Vec3::new(h, h, h)); // 2
        mesh.vertices.push(center + Vec3::new(-h, h, h)); // 3
        mesh.vertices.push(center + Vec3::new(-h, -h, -h)); // 4
        mesh.vertices.push(center + Vec3::new(h, -h, -h)); // 5
        mesh.vertices.push(center + Vec3::new(h, h, -h)); // 6
        mesh.vertices.push(center + Vec3::new(-h, h, -h)); // 7

        // Indices
        let indices = [
            [0, 1, 2],
            [0, 2, 3], // Front
            [5, 4, 7],
            [5, 7, 6], // Back
            [3, 2, 6],
            [3, 6, 7], // Top
            [4, 5, 1],
            [4, 1, 0], // Bottom
            [1, 5, 6],
            [1, 6, 2], // Right
            [4, 0, 3],
            [4, 3, 7], // Left
        ];

        for tri in &indices {
            mesh.indices
                .push([base_idx + tri[0], base_idx + tri[1], base_idx + tri[2]]);
        }
    }
}

/// Utility to voxelize meshes.
pub struct Voxelizer;

impl Voxelizer {
    /// Converts a mesh into a voxel grid using stochastic surface sampling.
    ///
    /// # Arguments
    ///
    /// * `mesh` - The source mesh.
    /// * `resolution` - The number of voxels along the longest axis.
    #[must_use]
    pub fn voxelize(mesh: &Mesh, resolution: usize) -> VoxelGrid {
        if mesh.vertices.is_empty() {
            return VoxelGrid::new(1, 1, 1, 1.0, Vec3::default());
        }

        // 1. Calculate AABB
        let aabb = AABB::from_points(&mesh.vertices);
        let size = aabb.max - aabb.min;
        let max_dim = size.x.max(size.y).max(size.z);

        // Ensure non-zero size for flat meshes
        let max_dim = if max_dim < 1e-6 { 1.0 } else { max_dim };

        let voxel_size = max_dim / resolution as f32;

        let width = (size.x / voxel_size).ceil() as usize + 1;
        let height = (size.y / voxel_size).ceil() as usize + 1;
        let depth = (size.z / voxel_size).ceil() as usize + 1;

        let mut grid = VoxelGrid::new(width, height, depth, voxel_size, aabb.min);
        let mut rng = XorShift32::new(12345);

        // 2. Iterate triangles and sample
        for tri in &mesh.indices {
            let v0 = mesh.vertices[tri[0]];
            let v1 = mesh.vertices[tri[1]];
            let v2 = mesh.vertices[tri[2]];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let area = edge1.cross(edge2).length() * 0.5;

            // Density: samples per voxel area.
            // Voxel face area = voxel_size^2.
            // We want at least ~5-10 samples per voxel to ensure coverage.
            let samples_per_unit_area = 10.0 / (voxel_size * voxel_size);
            let num_samples = (area * samples_per_unit_area).ceil() as u32;

            for _ in 0..num_samples {
                // Random barycentric coordinates
                let r1 = rng.next_f32();
                let r2 = rng.next_f32();

                // Uniform sampling on triangle:
                // p = (1 - sqrt(r1)) * A + (sqrt(r1) * (1 - r2)) * B + (sqrt(r1) * r2) * C
                let sqrt_r1 = r1.sqrt();
                let u = 1.0 - sqrt_r1;
                let v = sqrt_r1 * (1.0 - r2);
                let w = sqrt_r1 * r2;

                let p = v0 * u + v1 * v + v2 * w;

                // Map to grid
                let local_p = p - aabb.min;
                let gx = (local_p.x / voxel_size).floor() as usize;
                let gy = (local_p.y / voxel_size).floor() as usize;
                let gz = (local_p.z / voxel_size).floor() as usize;

                grid.set(gx, gy, gz, true);
            }
        }

        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxelize_cube_manual() {
        let mut mesh = Mesh::new();
        let size = 2.0;
        let h = size / 2.0;

        // Add vertices for a simple cube manually
        mesh.vertices.push(Vec3::new(-h, -h, h)); // 0
        mesh.vertices.push(Vec3::new(h, -h, h)); // 1
        mesh.vertices.push(Vec3::new(h, h, h)); // 2
        mesh.vertices.push(Vec3::new(-h, h, h)); // 3
        mesh.vertices.push(Vec3::new(-h, -h, -h)); // 4
        mesh.vertices.push(Vec3::new(h, -h, -h)); // 5
        mesh.vertices.push(Vec3::new(h, h, -h)); // 6
        mesh.vertices.push(Vec3::new(-h, h, -h)); // 7

        // Just one face (Front) to test
        mesh.indices.push([0, 1, 2]);
        mesh.indices.push([0, 2, 3]);

        let resolution = 10;
        let grid = Voxelizer::voxelize(&mesh, resolution);

        // Check dimensions
        assert!(grid.width > 0);
        assert!(grid.height > 0);
        assert!(grid.depth > 0);

        // AABB min is -1,-1,-1. Max is 1,1,1. Size 2.
        // Voxel size = 2 / 10 = 0.2.
        // Front face is at Z=1.
        // In grid coords: (1 - (-1)) / 0.2 = 10.
        // So Z index should be around resolution.

        // Count set voxels
        let mut set_count = 0;
        for b in &grid.data {
            if *b {
                set_count += 1;
            }
        }
        assert!(set_count > 0, "Should have voxelized something");
    }

    #[test]
    fn test_to_mesh() {
        let mut grid = VoxelGrid::new(2, 2, 2, 1.0, Vec3::default());
        grid.set(0, 0, 0, true);

        let mesh = grid.to_mesh();
        // A cube has 8 vertices and 12 triangles.
        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.indices.len(), 12);
    }
}
