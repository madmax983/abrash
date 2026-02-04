//! Procedural terrain generation.

use crate::math::Vec3;
use crate::mesh::Mesh;

/// Generates a procedural terrain mesh based on a height function.
///
/// # Arguments
///
/// * `width_segments` - Number of segments along the X axis.
/// * `depth_segments` - Number of segments along the Z axis.
/// * `size_scale` - Physical size of each grid cell.
/// * `height_func` - A function that takes (x, z) coordinates and returns height (y).
pub fn generate_heightmap<F>(
    width_segments: usize,
    depth_segments: usize,
    size_scale: f32,
    height_func: F,
) -> Mesh
where
    F: Fn(f32, f32) -> f32,
{
    let num_vertices = (width_segments + 1) * (depth_segments + 1);
    let mut vertices = Vec::with_capacity(num_vertices);
    let mut indices = Vec::with_capacity(width_segments * depth_segments * 6);

    let width_f = width_segments as f32 * size_scale;
    let depth_f = depth_segments as f32 * size_scale;
    let offset_x = -width_f / 2.0;
    let offset_z = -depth_f / 2.0;

    for z in 0..=depth_segments {
        for x in 0..=width_segments {
            let x_pos = offset_x + x as f32 * size_scale;
            let z_pos = offset_z + z as f32 * size_scale;
            let y_pos = height_func(x_pos, z_pos);
            vertices.push(Vec3::new(x_pos, y_pos, z_pos));
        }
    }

    for z in 0..depth_segments {
        for x in 0..width_segments {
            // Grid indices:
            // TL -- TR
            // |  /  |
            // BL -- BR
            //
            // Vertices are row-major, so z=0 is the "bottom" row (visually, usually min Z).
            // Let's assume standard XZ plane grid where +Z is "forward" or "up" in index.
            let row_len = width_segments + 1;

            let v0 = z * row_len + x; // Bottom-Left
            let v1 = z * row_len + (x + 1); // Bottom-Right
            let v2 = (z + 1) * row_len + x; // Top-Left
            let v3 = (z + 1) * row_len + (x + 1); // Top-Right

            // Triangle 1: v0 -> v1 -> v3 (Bottom-Left -> Bottom-Right -> Top-Right)
            indices.push([v0, v1, v3]);

            // Triangle 2: v0 -> v3 -> v2 (Bottom-Left -> Top-Right -> Top-Left)
            indices.push([v0, v3, v2]);
        }
    }

    Mesh { vertices, indices }
}

/// A simple sine-wave terrain function for testing/example.
pub fn sine_wave_terrain(x: f32, z: f32) -> f32 {
    (x * 0.5).sin() * 2.0 + (z * 0.5).cos() * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_heightmap_dimensions() {
        let w = 4;
        let d = 4;
        let mesh = generate_heightmap(w, d, 1.0, |_, _| 0.0);

        // Vertices = (w+1)*(d+1) = 5*5 = 25
        assert_eq!(mesh.vertices.len(), 25);

        // Indices = w * d * 2 triangles
        // 4 * 4 * 2 = 32 triangles
        assert_eq!(mesh.indices.len(), 32);
    }

    #[test]
    fn test_generate_heightmap_values() {
        let mesh = generate_heightmap(1, 1, 10.0, |x, z| x + z);

        // 1 segment means 2x2 vertices
        // Offset is -5.0, -5.0 (width/2)
        // Vertices:
        // (-5, -10, -5)
        // ( 5,   0, -5)
        // (-5,   0,  5)
        // ( 5,  10,  5)

        // Let's check a few
        let v0 = mesh.vertices[0]; // x=0, z=0 index -> pos (-5, -5)
        assert!((v0.x - -5.0).abs() < 0.001);
        assert!((v0.z - -5.0).abs() < 0.001);
        assert!((v0.y - (-10.0)).abs() < 0.001); // -5 + -5
    }
}
