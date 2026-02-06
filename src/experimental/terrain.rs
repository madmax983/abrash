//! Procedural terrain generation module.
//!
//! Generates heightmap-based meshes using fractal noise.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

/// Generates procedural terrain.
pub struct TerrainGenerator {
    seed: u32,
    roughness: f32,
}

impl TerrainGenerator {
    /// Creates a new generator with a seed.
    #[must_use]
    pub const fn new(seed: u32, roughness: f32) -> Self {
        Self { seed, roughness }
    }

    /// Generates a terrain mesh.
    ///
    /// * `width` - Number of cells along X axis.
    /// * `depth` - Number of cells along Z axis.
    /// * `scale` - Physical size of each cell.
    #[must_use]
    pub fn generate_mesh(&self, width: usize, depth: usize, scale: f32) -> Mesh {
        let mut mesh = Mesh::new();
        let w_verts = width + 1;
        let d_verts = depth + 1;

        // Offset to center the terrain
        let offset_x = (width as f32 * scale) / 2.0;
        let offset_z = (depth as f32 * scale) / 2.0;

        // Generate vertices
        for z in 0..d_verts {
            for x in 0..w_verts {
                let px = x as f32 * scale - offset_x;
                let pz = z as f32 * scale - offset_z;

                // Height generation
                // Use coordinates scaled down for noise to get larger features
                let nx = x as f32 * 0.1;
                let nz = z as f32 * 0.1;

                let h = self.fbm(nx, nz) * self.roughness;

                mesh.vertices.push(Vec3::new(px, h, pz));
                mesh.uvs
                    .push(Vec2::new(x as f32 / width as f32, z as f32 / depth as f32));
            }
        }

        // Generate indices
        for z in 0..depth {
            for x in 0..width {
                let top_left = z * w_verts + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * w_verts + x;
                let bottom_right = bottom_left + 1;

                // Triangle 1
                mesh.indices.push([top_left, bottom_left, top_right]);
                // Triangle 2
                mesh.indices.push([top_right, bottom_left, bottom_right]);
            }
        }

        mesh
    }

    fn noise(&self, x: f32, y: f32) -> f32 {
        let x = x + self.seed as f32;
        let y = y + self.seed as f32;

        let i_x = x.floor();
        let i_y = y.floor();
        let f_x = x.fract();
        let f_y = y.fract();

        let a = random_hash(i_x, i_y);
        let b = random_hash(i_x + 1.0, i_y);
        let c = random_hash(i_x, i_y + 1.0);
        let d = random_hash(i_x + 1.0, i_y + 1.0);

        let u_x = f_x * f_x * (3.0 - 2.0 * f_x);
        let u_y = f_y * f_y * (3.0 - 2.0 * f_y);

        let mix_a_b = a * (1.0 - u_x) + b * u_x;
        let mix_c_d = c * (1.0 - u_x) + d * u_x;

        mix_a_b * (1.0 - u_y) + mix_c_d * u_y
    }

    fn fbm(&self, x: f32, y: f32) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let octaves = 4;

        for _ in 0..octaves {
            value += self.noise(x * frequency, y * frequency) * amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        value
    }
}

/// Simple pseudo-random hash function based on sine.
/// Returns value in [0.0, 1.0].
fn random_hash(x: f32, y: f32) -> f32 {
    ((x * 12.9898 + y * 78.233).sin() * 43_758.547)
        .fract()
        .abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_dimensions() {
        let generator = TerrainGenerator::new(12345, 10.0);
        let width = 10;
        let depth = 10;
        let scale = 1.0;
        let mesh = generator.generate_mesh(width, depth, scale);

        let expected_verts = (width + 1) * (depth + 1);
        assert_eq!(mesh.vertices.len(), expected_verts);

        let expected_tris = width * depth * 2;
        assert_eq!(mesh.indices.len(), expected_tris);
    }
}
