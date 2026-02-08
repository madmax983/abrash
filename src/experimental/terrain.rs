//! Procedural Terrain Generation Module
//!
//! Generates 3D terrain meshes using value noise.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

/// Holds the generated terrain mesh and per-vertex normals.
pub struct Terrain {
    pub mesh: Mesh,
    pub normals: Vec<Vec3>,
}

/// A simple pseudo-random number generator.
struct Rng {
    state: u32,
}

impl Rng {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        // Xorshift32
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        // Map to 0.0..1.0
        (x as f32) / (u32::MAX as f32)
    }
}

/// Value Noise Generator
struct ValueNoise {
    width: usize,
    height: usize,
    values: Vec<f32>,
}

impl ValueNoise {
    fn new(width: usize, height: usize, seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let size = width * height;
        let mut values = Vec::with_capacity(size);
        for _ in 0..size {
            values.push(rng.next_f32());
        }
        Self {
            width,
            height,
            values,
        }
    }

    fn get(&self, x: usize, y: usize) -> f32 {
        if x >= self.width || y >= self.height {
            0.0
        } else {
            self.values[y * self.width + x]
        }
    }

    fn sample(&self, x: f32, y: f32) -> f32 {
        // Grid coordinates
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        // Fractional part
        let tx = x - x.floor();
        let ty = y - y.floor();

        // Smoothstep interpolation
        let sx = tx * tx * (3.0 - 2.0 * tx);
        let sy = ty * ty * (3.0 - 2.0 * ty);

        // Fetch corner values
        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        // Bilinear interpolation with smooth weights
        let top = v00 + (v10 - v00) * sx;
        let bottom = v01 + (v11 - v01) * sx;

        top + (bottom - top) * sy
    }
}

/// Generates a terrain mesh using fractal value noise.
///
/// # Arguments
///
/// * `width` - Number of vertices along X axis.
/// * `depth` - Number of vertices along Z axis.
/// * `scale` - Physical size of the terrain (e.g., 100.0).
/// * `height_scale` - Maximum height of the terrain.
/// * `seed` - Random seed.
///
/// # Returns
///
/// A `Terrain` struct containing the mesh and normals.
pub fn generate_terrain(
    width: u32,
    depth: u32,
    scale: f32,
    height_scale: f32,
    seed: u32,
) -> Terrain {
    let mut mesh = Mesh::new();
    let num_verts = (width * depth) as usize;
    mesh.vertices.reserve(num_verts);
    mesh.uvs.reserve(num_verts);

    // Noise setup
    // Use a fixed grid size for noise, independent of mesh resolution for consistency?
    // Or couple them. Let's couple them for simplicity, but maybe use a lower frequency.
    let noise_w = 32;
    let noise_h = 32;
    let noise = ValueNoise::new(noise_w, noise_h, seed);
    let noise2 = ValueNoise::new(noise_w * 2, noise_h * 2, seed + 123); // Octave 2

    // Calculate vertices
    for z in 0..depth {
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;
            let v = z as f32 / (depth - 1) as f32;

            // Sample noise (FBM - Fractal Brownian Motion)
            let nx = u * (noise_w - 1) as f32;
            let ny = v * (noise_h - 1) as f32;

            let mut h = noise.sample(nx, ny);
            h += noise2.sample(nx * 2.0, ny * 2.0) * 0.5;
            h /= 1.5; // Normalize roughly to 0..1

            // Map to physical coordinates centered at origin
            let px = (u - 0.5) * scale;
            let pz = (v - 0.5) * scale;
            let py = h * height_scale;

            mesh.vertices.push(Vec3::new(px, py, pz));
            mesh.uvs.push(Vec2::new(u, v));
        }
    }

    // Calculate indices (grid topology)
    for z in 0..(depth - 1) {
        for x in 0..(width - 1) {
            let i0 = (z * width + x) as usize;
            let i1 = i0 + 1;
            let i2 = ((z + 1) * width + x) as usize;
            let i3 = i2 + 1;

            // Two triangles per quad
            mesh.indices.push([i0, i2, i1]);
            mesh.indices.push([i1, i2, i3]);
        }
    }

    // Compute normals
    // Since Mesh doesn't store per-vertex normals, we compute them manually by averaging face normals.
    let mut normals = vec![Vec3::default(); num_verts];

    // First, accumulate face normals
    for tri in &mesh.indices {
        let i0 = tri[0];
        let i1 = tri[1];
        let i2 = tri[2];

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize();

        normals[i0] = normals[i0] + normal;
        normals[i1] = normals[i1] + normal;
        normals[i2] = normals[i2] + normal;
    }

    // Normalize accumulated normals
    for n in &mut normals {
        *n = n.normalize();
    }

    Terrain { mesh, normals }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_generation() {
        let width = 10;
        let depth = 10;
        let terrain = generate_terrain(width, depth, 10.0, 5.0, 42);

        assert_eq!(terrain.mesh.vertices.len(), (width * depth) as usize);
        assert_eq!(terrain.normals.len(), (width * depth) as usize);

        // Check indices count: (w-1)*(d-1) quads * 2 tris * 3 indices
        let expected_indices = (width - 1) * (depth - 1) * 2 * 3;
        // mesh.indices stores [usize; 3], so verify number of triangles
        let expected_tris = (width - 1) * (depth - 1) * 2;
        assert_eq!(terrain.mesh.indices.len(), expected_tris as usize);
    }
}
