//! Procedural Terrain Generation Module
//!
//! Provides functions to generate terrain meshes using value noise.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

/// Generates a procedural terrain mesh using layered value noise.
///
/// # Arguments
///
/// * `width` - Number of vertices along the X axis.
/// * `depth` - Number of vertices along the Z axis.
/// * `max_height` - Maximum height of the terrain.
/// * `seed` - Random seed for procedural generation.
#[must_use]
pub fn generate_terrain_mesh(width: usize, depth: usize, max_height: f32, seed: u32) -> Mesh {
    let mut vertices = Vec::with_capacity(width * depth);
    let mut uvs = Vec::with_capacity(width * depth);
    let mut indices = Vec::with_capacity((width - 1) * (depth - 1) * 2);

    // Generate vertices
    for z in 0..depth {
        for x in 0..width {
            let u = x as f32 / (width as f32 - 1.0);
            let v = z as f32 / (depth as f32 - 1.0);

            // Layered noise for detail
            let noise = layered_noise(u * 5.0, v * 5.0, seed);

            let h = noise * max_height;

            // Center the mesh around (0,0) in XZ plane
            let px = x as f32 - width as f32 / 2.0;
            let pz = z as f32 - depth as f32 / 2.0;

            vertices.push(Vec3::new(px, h, pz));
            uvs.push(Vec2::new(u, v));
        }
    }

    // Generate indices (two triangles per quad)
    for z in 0..depth - 1 {
        for x in 0..width - 1 {
            let i0 = z * width + x;
            let i1 = i0 + 1;
            let i2 = (z + 1) * width + x;
            let i3 = i2 + 1;

            // First triangle (0 -> 2 -> 1)
            indices.push([i0, i2, i1]);
            // Second triangle (1 -> 2 -> 3)
            indices.push([i1, i2, i3]);
        }
    }

    Mesh {
        vertices,
        indices,
        uvs,
    }
}

/// Computes smooth vertex normals by averaging face normals.
///
/// This is essential for smooth shading of the terrain.
///
/// # Arguments
///
/// * `mesh` - The terrain mesh.
/// * `width` - Grid width (used for validation, though implied by mesh).
/// * `depth` - Grid depth.
#[must_use]
pub fn compute_vertex_normals(mesh: &Mesh) -> Vec<Vec3> {
    let mut normals = vec![Vec3::default(); mesh.vertices.len()];

    // Accumulate face normals onto vertices
    for tri in &mesh.indices {
        let i0 = tri[0];
        let i1 = tri[1];
        let i2 = tri[2];

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        // Cross product gives normal perpendicular to face
        // We don't normalize yet to weight by triangle area (larger triangles contribute more)
        let normal = edge1.cross(edge2);

        normals[i0] = normals[i0] + normal;
        normals[i1] = normals[i1] + normal;
        normals[i2] = normals[i2] + normal;
    }

    // Normalize all vertex normals
    for n in &mut normals {
        *n = n.normalize();
    }

    normals
}

// --- Noise Functions ---

fn hash(n: u32) -> u32 {
    let mut n = n;
    n = (n << 13) ^ n;
    n = n.wrapping_mul(n.wrapping_mul(n).wrapping_mul(15731).wrapping_add(789221)).wrapping_add(1376312589);
    n
}

fn hash_coord(x: u32, z: u32, seed: u32) -> f32 {
    // Simple hash combining coordinates
    let h = hash(x.wrapping_add(z.wrapping_mul(57)).wrapping_add(seed));
    (h & 0xFFFF) as f32 / 65535.0
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn value_noise_2d(x: f32, z: f32, seed: u32) -> f32 {
    let x_int = x.floor() as u32;
    let z_int = z.floor() as u32;
    let tx = x - x.floor();
    let tz = z - z.floor();

    let v00 = hash_coord(x_int, z_int, seed);
    let v10 = hash_coord(x_int + 1, z_int, seed);
    let v01 = hash_coord(x_int, z_int + 1, seed);
    let v11 = hash_coord(x_int + 1, z_int + 1, seed);

    let u = smoothstep(tx);
    let v = smoothstep(tz);

    let i1 = lerp(v00, v10, u);
    let i2 = lerp(v01, v11, u);

    lerp(i1, i2, v)
}

fn layered_noise(x: f32, z: f32, seed: u32) -> f32 {
    let mut total = 0.0;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut max_val = 0.0;

    // 4 Octaves of noise
    for i in 0..4 {
        // Offset seed per octave to avoid correlation
        total += value_noise_2d(x * frequency, z * frequency, seed.wrapping_add(i)) * amplitude;
        max_val += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    total / max_val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_generation_counts() {
        let width = 10;
        let depth = 10;
        let mesh = generate_terrain_mesh(width, depth, 10.0, 12345);

        assert_eq!(mesh.vertices.len(), width * depth);
        assert_eq!(mesh.uvs.len(), width * depth);
        // (width-1) * (depth-1) quads * 2 triangles * 3 indices
        let expected_indices = (width - 1) * (depth - 1) * 2;
        assert_eq!(mesh.indices.len(), expected_indices);
    }

    #[test]
    fn test_determinism() {
        let mesh1 = generate_terrain_mesh(10, 10, 5.0, 42);
        let mesh2 = generate_terrain_mesh(10, 10, 5.0, 42);

        // Check vertices are identical
        for (v1, v2) in mesh1.vertices.iter().zip(mesh2.vertices.iter()) {
            assert!((v1.y - v2.y).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_normals_generation() {
        let width = 4;
        let depth = 4;
        let mesh = generate_terrain_mesh(width, depth, 5.0, 99);
        let normals = compute_vertex_normals(&mesh);

        assert_eq!(normals.len(), mesh.vertices.len());

        // Check normals are normalized
        for n in normals {
            assert!((n.length() - 1.0).abs() < 0.001);
        }
    }
}
