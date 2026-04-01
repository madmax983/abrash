//! Isosurface Extraction using Marching Tetrahedra.
//!
//! This module converts a signed distance field (SDF) or scalar field into a triangle mesh.
//! It uses the Marching Tetrahedra algorithm, which decomposes each grid cell (cube) into
//! 6 tetrahedra to avoid topological ambiguities present in standard Marching Cubes.

use abrash_core::math::{Vec3, Vec4};
use abrash_core::mesh::Mesh;

/// Extracts an isosurface from a scalar field.
///
/// # Arguments
///
/// * `sdf` - A function that returns the signed distance value at a given point.
/// * `min` - The minimum corner of the bounding box.
/// * `max` - The maximum corner of the bounding box.
/// * `resolution` - The number of cells along the longest axis.
///
/// # Returns
///
/// A `Mesh` representing the zero-level set of the field.
pub fn extract_isosurface<F>(sdf: F, min: Vec3, max: Vec3, resolution: usize) -> Mesh
where
    F: Fn(Vec3) -> f32,
{
    let size = max - min;
    let max_dim = size.x.max(size.y).max(size.z);
    let step = max_dim / resolution as f32;

    let width = (size.x / step).ceil() as usize + 1;
    let height = (size.y / step).ceil() as usize + 1;
    let depth = (size.z / step).ceil() as usize + 1;

    // Optimization: Preallocate vectors using a surface-area heuristic.
    // The number of surface cells is typically proportional to the surface area,
    // which scales as the 2/3 power of the total volume (total_cells).
    let total_cells = width * height * depth;
    let estimated_vertices = (total_cells as f32).powf(0.666_666_7) as usize * 3;
    let estimated_indices = estimated_vertices * 2 / 3; // Rough estimate of triangles from vertices

    let mut vertices = Vec::with_capacity(estimated_vertices);
    let mut indices = Vec::with_capacity(estimated_indices);
    let mut normals = Vec::with_capacity(estimated_vertices);

    // Cache SDF values to avoid recomputing
    // Index: z * height * width + y * width + x
    let mut grid_values = vec![0.0; width * height * depth];

    for z in 0..depth {
        for y in 0..height {
            for x in 0..width {
                let pos = min + Vec3::new(x as f32, y as f32, z as f32) * step;
                let val = sdf(pos);
                grid_values[z * height * width + y * width + x] = val;
            }
        }
    }

    let get_val = |x, y, z| grid_values[z * height * width + y * width + x];
    let get_pos = |x, y, z| min + Vec3::new(x as f32, y as f32, z as f32) * step;

    for z in 0..depth - 1 {
        for y in 0..height - 1 {
            for x in 0..width - 1 {
                // Cube vertices:
                // 0: x, y, z
                // 1: x+1, y, z
                // 2: x+1, y+1, z
                // 3: x, y+1, z
                // 4: x, y, z+1
                // 5: x+1, y, z+1
                // 6: x+1, y+1, z+1
                // 7: x, y+1, z+1

                let p = [
                    get_pos(x, y, z),             // 0
                    get_pos(x + 1, y, z),         // 1
                    get_pos(x + 1, y + 1, z),     // 2
                    get_pos(x, y + 1, z),         // 3
                    get_pos(x, y, z + 1),         // 4
                    get_pos(x + 1, y, z + 1),     // 5
                    get_pos(x + 1, y + 1, z + 1), // 6
                    get_pos(x, y + 1, z + 1),     // 7
                ];

                let v = [
                    get_val(x, y, z),
                    get_val(x + 1, y, z),
                    get_val(x + 1, y + 1, z),
                    get_val(x, y + 1, z),
                    get_val(x, y, z + 1),
                    get_val(x + 1, y, z + 1),
                    get_val(x + 1, y + 1, z + 1),
                    get_val(x, y + 1, z + 1),
                ];

                // Decompose into 6 tetrahedra
                // T1: 0, 1, 3, 5
                polygonize_tetrahedron(&mut vertices, &mut indices, &p, &v, [0, 1, 3, 5]);
                // T2: 1, 2, 3, 5
                polygonize_tetrahedron(&mut vertices, &mut indices, &p, &v, [1, 2, 3, 5]);
                // T3: 2, 3, 5, 6
                polygonize_tetrahedron(&mut vertices, &mut indices, &p, &v, [2, 3, 5, 6]);
                // T4: 0, 3, 4, 5
                polygonize_tetrahedron(&mut vertices, &mut indices, &p, &v, [0, 3, 4, 5]);
                // T5: 7, 4, 5, 6
                polygonize_tetrahedron(&mut vertices, &mut indices, &p, &v, [7, 4, 5, 6]);
                // T6: 3, 4, 5, 6
                polygonize_tetrahedron(&mut vertices, &mut indices, &p, &v, [3, 4, 5, 6]);
            }
        }
    }

    // Compute normals using gradient
    let eps = step * 0.1;
    for v in &vertices {
        let n = compute_normal(&sdf, *v, eps);
        normals.push(n);
    }

    // Tangents are populated with default values.
    // Full tangent generation requires complete UV maps which are currently placeholders.
    let tangents = vec![Vec4::default(); vertices.len()];

    Mesh {
        vertices,
        indices,
        uvs: vec![abrash_core::math::Vec2::default(); normals.len()], // Placeholder UVs
        normals,
        tangents,
    }
}

fn compute_normal<F>(sdf: &F, p: Vec3, eps: f32) -> Vec3
where
    F: Fn(Vec3) -> f32,
{
    let dx = sdf(Vec3::new(p.x + eps, p.y, p.z)) - sdf(Vec3::new(p.x - eps, p.y, p.z));
    let dy = sdf(Vec3::new(p.x, p.y + eps, p.z)) - sdf(Vec3::new(p.x, p.y - eps, p.z));
    let dz = sdf(Vec3::new(p.x, p.y, p.z + eps)) - sdf(Vec3::new(p.x, p.y, p.z - eps));
    Vec3::new(dx, dy, dz).normalize()
}

fn polygonize_tetrahedron(
    vertices: &mut Vec<Vec3>,
    indices: &mut Vec<[usize; 3]>,
    p: &[Vec3; 8],
    v: &[f32; 8],
    idxs: [usize; 4],
) {
    let i0 = idxs[0];
    let i1 = idxs[1];
    let i2 = idxs[2];
    let i3 = idxs[3];

    let mut case = 0;
    if v[i0] < 0.0 {
        case |= 1;
    }
    if v[i1] < 0.0 {
        case |= 2;
    }
    if v[i2] < 0.0 {
        case |= 4;
    }
    if v[i3] < 0.0 {
        case |= 8;
    }

    // Table of edges. Each entry is a list of edges (pairs of vertex indices relative to tetrahedron).
    // Tetrahedron vertices: 0->i0, 1->i1, 2->i2, 3->i3
    // Edges: 0:(0,1), 1:(1,2), 2:(2,0), 3:(0,3), 4:(1,3), 5:(2,3)
    // -1 indicates end of list.
    #[rustfmt::skip]
    let tri_table: [[i8; 7]; 16] = [
        [-1, -1, -1, -1, -1, -1, -1], // 0: ----
        [ 0,  2,  3, -1, -1, -1, -1], // 1: 0---
        [ 1,  0,  4, -1, -1, -1, -1], // 2: -1--
        [ 3,  1,  2,  3,  4,  1, -1], // 3: 01--
        [ 2,  1,  5, -1, -1, -1, -1], // 4: --2-
        [ 3,  0,  5,  5,  0,  1, -1], // 5: 0-2-
        [ 2,  4,  5,  2,  0,  4, -1], // 6: -12-
        [ 3,  5,  4, -1, -1, -1, -1], // 7: 012-
        [ 3,  4,  5, -1, -1, -1, -1], // 8: ---3
        [ 0,  5,  4,  0,  2,  5, -1], // 9: 0--3
        [ 5,  3,  1,  5,  1,  0, -1], // 10:-1-3
        [ 2,  5,  1, -1, -1, -1, -1], // 11:01-3
        [ 1,  3,  2,  1,  4,  3, -1], // 12:--23
        [ 1,  4,  0, -1, -1, -1, -1], // 13:0-23
        [ 0,  3,  2, -1, -1, -1, -1], // 14:-123
        [-1, -1, -1, -1, -1, -1, -1], // 15:0123
    ];

    let edges = tri_table[case];
    let mut current_tri = [0, 0, 0];
    let mut vert_count = 0;

    for &edge_idx in &edges {
        if edge_idx == -1 {
            break;
        }

        // Map edge index to vertex pairs
        let (va, vb) = match edge_idx {
            0 => (i0, i1),
            1 => (i1, i2),
            2 => (i2, i0),
            3 => (i0, i3),
            4 => (i1, i3),
            5 => (i2, i3),
            _ => unreachable!(),
        };

        let p_a = p[va];
        let p_b = p[vb];
        let v_a = v[va];
        let v_b = v[vb];

        // Linear interpolation:
        // val = v_a + t * (v_b - v_a) = 0
        // t = -v_a / (v_b - v_a) = v_a / (v_a - v_b)
        let t = if (v_a - v_b).abs() > 1e-5 {
            v_a / (v_a - v_b)
        } else {
            0.5
        };

        // Safety clamp
        let t = t.clamp(0.0, 1.0);

        // Interpolate position
        // p = p_a + t * (p_b - p_a)
        let pos = Vec3::new(
            p_a.x + t * (p_b.x - p_a.x),
            p_a.y + t * (p_b.y - p_a.y),
            p_a.z + t * (p_b.z - p_a.z),
        );

        // Simple index buffer generation (no welding/sharing for now)
        current_tri[vert_count] = vertices.len();
        vertices.push(pos);
        vert_count += 1;

        if vert_count == 3 {
            indices.push(current_tri);
            vert_count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_mesh() {
        let sphere_sdf = |p: Vec3| p.length() - 1.0;
        let min = Vec3::new(-1.5, -1.5, -1.5);
        let max = Vec3::new(1.5, 1.5, 1.5);
        let resolution = 10;

        let mesh = extract_isosurface(sphere_sdf, min, max, resolution);

        assert!(!mesh.vertices.is_empty(), "Should generate vertices");
        assert!(!mesh.indices.is_empty(), "Should generate triangles");
        assert!(
            mesh.normals.len() == mesh.vertices.len(),
            "Should have normals"
        );

        // Check if vertices are roughly on the sphere surface (radius 1)
        for v in &mesh.vertices {
            let dist = v.length();
            assert!(
                (dist - 1.0).abs() < 0.2,
                "Vertex should be near surface, got dist {dist}"
            );
        }
    }
}
