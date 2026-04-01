//! Volumetric Sculpting Module.
//!
//! Provides a `Volume` struct that wraps a `VoxelGrid` and supports real-time modification
//! (carving, depositing) and optimized mesh generation (face culling).

use crate::experimental::voxelizer::{VoxelGrid, Voxelizer};
use abrash_core::math::{Vec3, Vec4};
use abrash_core::mesh::Mesh;

/// A modifiable volumetric object.
pub struct Volume {
    pub grid: VoxelGrid,
}

impl Volume {
    /// Creates a new empty volume.
    #[must_use]
    pub fn new(width: usize, height: usize, depth: usize, voxel_size: f32, origin: Vec3) -> Self {
        Self {
            grid: VoxelGrid::new(width, height, depth, voxel_size, origin),
        }
    }

    /// Creates a volume from an existing mesh.
    #[must_use]
    pub fn from_mesh(mesh: &Mesh, resolution: usize) -> Self {
        Self {
            grid: Voxelizer::voxelize(mesh, resolution),
        }
    }

    /// Carves (removes) voxels within a spherical radius.
    pub fn carve(&mut self, center: Vec3, radius: f32) {
        self.modify_sphere(center, radius, false);
    }

    /// Deposits (adds) voxels within a spherical radius.
    pub fn deposit(&mut self, center: Vec3, radius: f32) {
        self.modify_sphere(center, radius, true);
    }

    fn modify_sphere(&mut self, center: Vec3, radius: f32, value: bool) {
        // Convert sphere bounds to grid coordinates
        let local_center = center - self.grid.origin;
        let r_grid = radius / self.grid.voxel_size;

        let cx = local_center.x / self.grid.voxel_size;
        let cy = local_center.y / self.grid.voxel_size;
        let cz = local_center.z / self.grid.voxel_size;

        // Bounding box in grid coords
        // Clamp to grid dimensions
        let min_x = ((cx - r_grid).floor() as isize).max(0) as usize;
        let max_x = ((cx + r_grid).ceil() as isize).min(self.grid.width as isize - 1) as usize;

        let min_y = ((cy - r_grid).floor() as isize).max(0) as usize;
        let max_y = ((cy + r_grid).ceil() as isize).min(self.grid.height as isize - 1) as usize;

        let min_z = ((cz - r_grid).floor() as isize).max(0) as usize;
        let max_z = ((cz + r_grid).ceil() as isize).min(self.grid.depth as isize - 1) as usize;

        let r2 = radius * radius;

        for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    // Check distance
                    let p = self.grid.origin
                        + Vec3::new(
                            (x as f32 + 0.5) * self.grid.voxel_size,
                            (y as f32 + 0.5) * self.grid.voxel_size,
                            (z as f32 + 0.5) * self.grid.voxel_size,
                        );

                    let diff = p - center;
                    if diff.dot(diff) <= r2 {
                        self.grid.set(x, y, z, value);
                    }
                }
            }
        }
    }

    /// Generates an optimized mesh (Hidden Face Removal).
    #[must_use]
    pub fn to_mesh_optimized(&self) -> Mesh {
        let mut mesh = Mesh::new();
        let h = self.grid.voxel_size * 0.5;

        for z in 0..self.grid.depth {
            for y in 0..self.grid.height {
                for x in 0..self.grid.width {
                    if !self.grid.get(x, y, z) {
                        continue;
                    }

                    let center = self.grid.origin
                        + Vec3::new(
                            (x as f32 + 0.5) * self.grid.voxel_size,
                            (y as f32 + 0.5) * self.grid.voxel_size,
                            (z as f32 + 0.5) * self.grid.voxel_size,
                        );

                    // Check neighbors
                    // Left (-X)
                    if x == 0 || !self.grid.get(x - 1, y, z) {
                        Self::add_face(&mut mesh, center, h, Face::Left);
                    }
                    // Right (+X)
                    if x == self.grid.width - 1 || !self.grid.get(x + 1, y, z) {
                        Self::add_face(&mut mesh, center, h, Face::Right);
                    }
                    // Bottom (-Y)
                    if y == 0 || !self.grid.get(x, y - 1, z) {
                        Self::add_face(&mut mesh, center, h, Face::Bottom);
                    }
                    // Top (+Y)
                    if y == self.grid.height - 1 || !self.grid.get(x, y + 1, z) {
                        Self::add_face(&mut mesh, center, h, Face::Top);
                    }
                    // Back (-Z)
                    if z == 0 || !self.grid.get(x, y, z - 1) {
                        Self::add_face(&mut mesh, center, h, Face::Back);
                    }
                    // Front (+Z)
                    if z == self.grid.depth - 1 || !self.grid.get(x, y, z + 1) {
                        Self::add_face(&mut mesh, center, h, Face::Front);
                    }
                }
            }
        }

        // Compute tangents for lighting
        mesh.compute_tangents();
        mesh
    }

    fn add_face(mesh: &mut Mesh, c: Vec3, h: f32, face: Face) {
        let base_idx = mesh.vertices.len();

        let (v0, v1, v2, v3, normal) = match face {
            Face::Left => (
                Vec3::new(-h, -h, -h), // BL
                Vec3::new(-h, -h, h),  // BR
                Vec3::new(-h, h, h),   // TR
                Vec3::new(-h, h, -h),  // TL
                Vec3::new(-1.0, 0.0, 0.0),
            ),
            Face::Right => (
                Vec3::new(h, -h, h),  // BL (relative to face normal looking at it)
                Vec3::new(h, -h, -h), // BR
                Vec3::new(h, h, -h),  // TR
                Vec3::new(h, h, h),   // TL
                Vec3::new(1.0, 0.0, 0.0),
            ),
            Face::Bottom => (
                Vec3::new(-h, -h, -h),
                Vec3::new(h, -h, -h),
                Vec3::new(h, -h, h),
                Vec3::new(-h, -h, h),
                Vec3::new(0.0, -1.0, 0.0),
            ),
            Face::Top => (
                Vec3::new(-h, h, h),
                Vec3::new(h, h, h),
                Vec3::new(h, h, -h),
                Vec3::new(-h, h, -h),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Face::Back => (
                Vec3::new(h, -h, -h),
                Vec3::new(-h, -h, -h),
                Vec3::new(-h, h, -h),
                Vec3::new(h, h, -h),
                Vec3::new(0.0, 0.0, -1.0),
            ),
            Face::Front => (
                Vec3::new(-h, -h, h),
                Vec3::new(h, -h, h),
                Vec3::new(h, h, h),
                Vec3::new(-h, h, h),
                Vec3::new(0.0, 0.0, 1.0),
            ),
        };

        mesh.vertices.push(c + v0);
        mesh.vertices.push(c + v1);
        mesh.vertices.push(c + v2);
        mesh.vertices.push(c + v3);

        for _ in 0..4 {
            mesh.normals.push(normal);
            mesh.tangents.push(Vec4::default());
        }

        // Correct UVs: (0,0), (1,0), (1,1), (0,1)
        mesh.uvs.push(abrash_core::math::Vec2::new(0.0, 0.0));
        mesh.uvs.push(abrash_core::math::Vec2::new(1.0, 0.0));
        mesh.uvs.push(abrash_core::math::Vec2::new(1.0, 1.0));
        mesh.uvs.push(abrash_core::math::Vec2::new(0.0, 1.0));

        // Triangle 1: 0-1-2
        mesh.indices.push([base_idx, base_idx + 1, base_idx + 2]);
        // Triangle 2: 0-2-3
        mesh.indices.push([base_idx, base_idx + 2, base_idx + 3]);
    }
}

#[derive(Clone, Copy)]
enum Face {
    Left,
    Right,
    Bottom,
    Top,
    Back,
    Front,
}
