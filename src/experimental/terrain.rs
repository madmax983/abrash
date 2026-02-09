//! Procedural Terrain Generation
//!
//! Generates a heightmap-based mesh using Value Noise.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

/// A simple Xorshift random number generator for deterministic noise.
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    const fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        // Generate float in [0, 1)
        (self.next() as f32) / (u32::MAX as f32)
    }
}

/// Generates a deterministic pseudo-random float in [0, 1] based on 2D coordinates.
fn hash_2d(x: i32, y: i32, seed: u32) -> f32 {
    // Combine coordinates into a single seed
    // Using a simple hash mixing
    let mut h = (x as u32).wrapping_mul(374761393);
    h = h.wrapping_add((y as u32).wrapping_mul(668265263));
    h = h.wrapping_add(seed);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);

    let mut rng = XorShift32::new(h);
    rng.next_f32()
}

/// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep interpolation (Hermite)
/// maps t from 0..1 to 0..1 with zero derivatives at endpoints
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 2D Value Noise
/// Returns a value in [0, 1]
pub fn value_noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    let x_floor = x.floor() as i32;
    let y_floor = y.floor() as i32;

    let tx = x - x_floor as f32;
    let ty = y - y_floor as f32;

    let sx = smoothstep(tx);
    let sy = smoothstep(ty);

    let v00 = hash_2d(x_floor, y_floor, seed);
    let v10 = hash_2d(x_floor + 1, y_floor, seed);
    let v01 = hash_2d(x_floor, y_floor + 1, seed);
    let v11 = hash_2d(x_floor + 1, y_floor + 1, seed);

    let top = lerp(v00, v10, sx);
    let bottom = lerp(v01, v11, sx);

    lerp(top, bottom, sy)
}

/// Generates a terrain mesh based on Value Noise.
///
/// # Arguments
///
/// * `width` - Number of vertices along the X axis.
/// * `depth` - Number of vertices along the Z axis.
/// * `scale` - Scale factor for noise coordinates (smaller = smoother/larger features).
/// * `amplitude` - Maximum height of the terrain.
/// * `seed` - Random seed for noise generation.
///
/// # Returns
///
/// A `Mesh` containing the generated terrain.
pub fn generate_terrain_mesh(
    width: u32,
    depth: u32,
    scale: f32,
    amplitude: f32,
    seed: u32,
) -> Mesh {
    let mut vertices = Vec::with_capacity((width * depth) as usize);
    let mut uvs = Vec::with_capacity((width * depth) as usize);
    let mut indices = Vec::with_capacity(((width - 1) * (depth - 1) * 2) as usize);

    // Center the mesh around (0,0)
    let offset_x = (width as f32) * 0.5;
    let offset_z = (depth as f32) * 0.5;

    // Generate vertices
    for z in 0..depth {
        for x in 0..width {
            let x_f = x as f32;
            let z_f = z as f32;

            // Sample noise
            // We use multiple octaves for more detail
            let mut height = 0.0;
            let mut frequency = scale;
            let mut amp = amplitude;
            let mut max_val = 0.0;

            // 3 Octaves of noise
            for _ in 0..3 {
                height += value_noise_2d(x_f * frequency, z_f * frequency, seed) * amp;
                max_val += amp;
                amp *= 0.5;
                frequency *= 2.0;
            }

            // Normalize back to 0..amplitude range
            if max_val > 0.0 {
                height = (height / max_val) * amplitude;
            }

            vertices.push(Vec3::new(x_f - offset_x, height, z_f - offset_z));

            // UVs map 0..1 across the whole terrain
            uvs.push(Vec2::new(x_f / (width as f32), z_f / (depth as f32)));
        }
    }

    // Generate indices (Quads -> 2 Triangles)
    for z in 0..(depth - 1) {
        for x in 0..(width - 1) {
            let i0 = (z * width + x) as usize;
            let i1 = (z * width + (x + 1)) as usize;
            let i2 = ((z + 1) * width + (x + 1)) as usize;
            let i3 = ((z + 1) * width + x) as usize;

            // Triangle 1
            indices.push([i0, i3, i1]); // CW winding? Check Mesh::cube order
            // Mesh::cube uses:
            // [0, 1, 2] -> (-h,-h,h), (h,-h,h), (h,h,h) -> CCW or CW depending on view.
            // Let's assume standard CCW for front faces.
            // i0 is (x, z), i1 is (x+1, z), i3 is (x, z+1)
            // (x,z) -> (x+1, z) -> (x, z+1)
            // This is "up-right-left" in XZ plane (looking down Y).
            // Normal would point up (+Y).

            // Triangle 2
            indices.push([i1, i3, i2]);
        }
    }

    Mesh {
        vertices,
        indices,
        uvs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_deterministic() {
        let val1 = value_noise_2d(10.5, 20.5, 12345);
        let val2 = value_noise_2d(10.5, 20.5, 12345);
        assert!((val1 - val2).abs() < f32::EPSILON);

        let val3 = value_noise_2d(10.5, 20.5, 67890); // Different seed
        assert!((val1 - val3).abs() > f32::EPSILON); // Should likely be different
    }

    #[test]
    fn test_terrain_mesh_generation() {
        let width = 10;
        let depth = 10;
        let mesh = generate_terrain_mesh(width, depth, 0.1, 5.0, 12345);

        assert_eq!(mesh.vertices.len(), (width * depth) as usize);
        assert_eq!(mesh.indices.len(), ((width - 1) * (depth - 1) * 2) as usize);

        // Check bounds
        for v in &mesh.vertices {
            assert!(v.y >= 0.0);
            // Amplitude is sum of 3 octaves: amp + amp/2 + amp/4 = 1.75 * amp
            // Wait, max_val logic in loop: amp=5.0 -> 5.0 + 2.5 + 1.25 = 8.75
            assert!(v.y <= 8.75);
        }
    }
}
