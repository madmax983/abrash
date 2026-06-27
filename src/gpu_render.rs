//! Compatibility wrapper around the external `abrash-gpu-render` crate.

pub use abrash_gpu_render::blitter::{AtlasHandle, BlitMode, GpuBlitter};
pub use abrash_gpu_render::{
    demo_app::{GpuDemoConfig, GpuInteractionController}, offscreen_bench::{GpuOffscreenBench, GpuOffscreenBenchConfig},
    GpuTriangle, GpuVertex, primitives::MeshValidationError, demo_app::{run_gpu_cube, run_gpu_cube_with_config},
    demo_app::run_mesh_demo, primitives::{unit_cube_mesh, validate_demo_config, validate_mesh},
};

/// Converts a CPU `Mesh` to GPU-compatible vertex and index buffers.
///
/// This bridge function allows `abrash::mesh::Mesh` (loaded via OBJ or generated procedurally)
/// to be rendered by the `abrash-gpu-render` backend.
///
/// # Errors
/// Returns an error if the mesh has too many vertices for the 16-bit index buffer limit (65535).
pub fn mesh_to_gpu(mesh: &crate::mesh::Mesh) -> Result<(Vec<GpuVertex>, Vec<u16>), String> {
    if mesh.vertices.len() > u16::MAX as usize {
        return Err(format!(
            "Mesh has too many vertices ({}) for u16 index buffer (max {})",
            mesh.vertices.len(),
            u16::MAX
        ));
    }

    // Convert vertices
    // Note: Mesh uses Vec3 (f32, f32, f32), GpuVertex uses [f32; 3]
    // Mesh doesn't store vertex colors, so we default to white.

    let mut vertices = Vec::with_capacity(mesh.vertices.len());
    vertices.extend(mesh.vertices.iter().map(|v| GpuVertex {
        position: [v.x, v.y, v.z],
        color: [1.0, 1.0, 1.0],
    }));

    // Convert indices
    // Mesh uses [usize; 3], Gpu uses flat u16 buffer
    let mut indices = Vec::with_capacity(mesh.indices.len() * 3);
    for tri in &mesh.indices {
        // ⚡ Bolt: Use `extend` instead of sequential `push` calls to eliminate
        // repetitive bounds-checking inside the hot index conversion loop.
        // We already checked bounds above, so cast is safe.
        indices.extend([tri[0] as u16, tri[1] as u16, tri[2] as u16]);
    }

    Ok((vertices, indices))
}
