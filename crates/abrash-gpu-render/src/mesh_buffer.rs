//! Converts `abrash_core::mesh::Mesh` to GPU vertex/index buffers.

use crate::shader::MvpVertex;
use abrash_core::mesh::Mesh;
use wgpu::util::DeviceExt;

/// GPU-side mesh buffers ready for indexed rendering.
pub struct GpuMeshBuffer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) triangle_count: u32,
}

/// Convert a mesh into GPU-ready vertices and flattened `u32` indices.
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

    Ok((vertices, indices))
}

impl GpuMeshBuffer {
    /// Upload a mesh to GPU vertex and index buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh data is invalid.
    pub fn from_mesh(device: &wgpu::Device, mesh: &Mesh) -> Result<Self, String> {
        let (vertices, indices) = prepare_mesh_data(mesh)?;
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
        assert_eq!(vertices[0].position[0], -0.5);
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
}
