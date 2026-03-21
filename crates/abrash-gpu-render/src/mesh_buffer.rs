//! Converts `abrash_core::mesh::Mesh` to GPU vertex/index buffers.

use crate::shader::{LitVertex, MvpVertex};
use abrash_core::mesh::Mesh;
use wgpu::util::DeviceExt;

/// GPU-side mesh buffers ready for indexed rendering.
pub struct GpuMeshBuffer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) triangle_count: u32,
}

/// Convert a mesh into position-only GPU vertices and flattened `u32` indices.
///
/// # Errors
///
/// Returns an error if the mesh is empty, contains out-of-bounds indices, or cannot fit
/// in the GPU index format.
pub fn prepare_mesh_data(mesh: &Mesh) -> Result<(Vec<MvpVertex>, Vec<u32>), String> {
    if mesh.vertices.is_empty() {
        return Err("mesh must contain at least one vertex".to_string());
    }
    if mesh.indices.is_empty() {
        return Err("mesh must contain at least one triangle".to_string());
    }

    let vertices = mesh
        .vertices
        .iter()
        .map(|vertex| MvpVertex {
            position: [vertex.x, vertex.y, vertex.z],
        })
        .collect::<Vec<_>>();

    let indices = flatten_indices(mesh)?;
    Ok((vertices, indices))
}

/// Convert a mesh into lit GPU vertices (position + normal) and flattened indices.
///
/// When the mesh has no normals, generates smooth per-vertex normals by averaging
/// the face normals of all adjacent triangles.
///
/// # Errors
///
/// Returns an error if the mesh is empty, contains out-of-bounds indices, or cannot fit
/// in the GPU index format.
pub fn prepare_lit_mesh_data(mesh: &Mesh) -> Result<(Vec<LitVertex>, Vec<u32>), String> {
    if mesh.vertices.is_empty() {
        return Err("mesh must contain at least one vertex".to_string());
    }
    if mesh.indices.is_empty() {
        return Err("mesh must contain at least one triangle".to_string());
    }

    let normals = if mesh.normals.len() == mesh.vertices.len() {
        // Use provided normals directly
        mesh.normals.clone()
    } else {
        // Generate smooth normals by averaging face normals
        generate_smooth_normals(&mesh.vertices, &mesh.indices)
    };

    let vertices = mesh
        .vertices
        .iter()
        .zip(normals.iter())
        .map(|(pos, norm)| LitVertex {
            position: [pos.x, pos.y, pos.z],
            normal: [norm.x, norm.y, norm.z],
        })
        .collect::<Vec<_>>();

    let indices = flatten_indices(mesh)?;
    Ok((vertices, indices))
}

/// Generate smooth per-vertex normals by averaging face normals of adjacent triangles.
fn generate_smooth_normals(
    vertices: &[abrash_core::math::Vec3],
    indices: &[[usize; 3]],
) -> Vec<abrash_core::math::Vec3> {
    use abrash_core::math::Vec3;

    let mut normals = vec![Vec3::ZERO; vertices.len()];

    for &[i0, i1, i2] in indices {
        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            continue;
        }
        let v0 = vertices[i0];
        let v1 = vertices[i1];
        let v2 = vertices[i2];
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);
        // Weight by face area (unnormalized cross product magnitude = 2× area)
        normals[i0] = normals[i0] + face_normal;
        normals[i1] = normals[i1] + face_normal;
        normals[i2] = normals[i2] + face_normal;
    }

    // Normalize
    for normal in &mut normals {
        let len = normal.length();
        if len > 1e-8 {
            *normal = *normal * (1.0 / len);
        } else {
            *normal = Vec3::new(0.0, 1.0, 0.0); // fallback up
        }
    }

    normals
}

fn flatten_indices(mesh: &Mesh) -> Result<Vec<u32>, String> {
    let mut indices = Vec::with_capacity(mesh.indices.len() * 3);
    for (triangle_index, triangle) in mesh.indices.iter().enumerate() {
        for &index in triangle {
            if index >= mesh.vertices.len() {
                return Err(format!(
                    "triangle {triangle_index} references vertex {index}, but only {} vertices exist",
                    mesh.vertices.len()
                ));
            }

            indices.push(
                u32::try_from(index)
                    .map_err(|_| format!("vertex index {index} does not fit in u32"))?,
            );
        }
    }
    Ok(indices)
}

impl GpuMeshBuffer {
    /// Upload a mesh to GPU vertex and index buffers with normals for lit rendering.
    ///
    /// Generates smooth normals automatically if the mesh has none.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh data is invalid.
    pub fn from_mesh(device: &wgpu::Device, mesh: &Mesh) -> Result<Self, String> {
        let (vertices, indices) = prepare_lit_mesh_data(mesh)?;
        let index_count =
            u32::try_from(indices.len()).map_err(|_| "index count exceeds u32".to_string())?;
        let triangle_count = u32::try_from(mesh.indices.len())
            .map_err(|_| "triangle count exceeds u32".to_string())?;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GpuMeshBuffer Vertex"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GpuMeshBuffer Index"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_count,
            triangle_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::mesh::Mesh;

    #[test]
    fn test_prepare_vertices_from_cube() {
        let mesh = Mesh::cube(1.0);
        let (vertices, indices) = prepare_mesh_data(&mesh).unwrap();

        assert_eq!(vertices.len(), mesh.vertices.len());
        assert_eq!(indices.len(), mesh.indices.len() * 3);
        assert!((vertices[0].position[0] - (-0.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_prepare_empty_mesh() {
        let mesh = Mesh::new();
        let result = prepare_mesh_data(&mesh);
        assert!(result.is_err());
    }

    #[test]
    fn test_indices_are_u32() {
        let mesh = Mesh::cube(1.0);
        let (_, indices) = prepare_mesh_data(&mesh).unwrap();

        for &index in &indices {
            assert!(index < 8);
        }
    }

    #[test]
    fn test_prepare_lit_mesh_generates_normals() {
        let mesh = Mesh::cube(1.0);
        assert!(mesh.normals.is_empty(), "cube has no normals by default");

        let (vertices, indices) = prepare_lit_mesh_data(&mesh).unwrap();
        assert_eq!(vertices.len(), mesh.vertices.len());
        assert_eq!(indices.len(), mesh.indices.len() * 3);

        // Generated normals should be unit-length
        for v in &vertices {
            let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
            assert!(
                (len - 1.0).abs() < 0.01,
                "normal should be unit-length, got {len}"
            );
        }
    }

    #[test]
    fn test_prepare_lit_mesh_uses_provided_normals() {
        use abrash_core::math::Vec3;

        let mut mesh = Mesh::cube(1.0);
        // Provide explicit normals (all pointing up)
        mesh.normals = vec![Vec3::new(0.0, 1.0, 0.0); mesh.vertices.len()];

        let (vertices, _) = prepare_lit_mesh_data(&mesh).unwrap();
        for v in &vertices {
            assert!((v.normal[1] - 1.0).abs() < f32::EPSILON);
        }
    }
}
