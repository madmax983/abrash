//! Shared data structures for GPU raster examples and future GPU backends.

/// A GPU-ready vertex with position and linear color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

/// A single triangle for GPU rasterization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuTriangle {
    pub vertices: [GpuVertex; 3],
}

/// Returns a colored unit cube mesh (8 vertices, 36 indices).
#[must_use]
pub fn unit_cube_mesh() -> (Vec<GpuVertex>, Vec<u16>) {
    let vertices = vec![
        GpuVertex {
            position: [-1.0, -1.0, -1.0],
            color: [1.0, 0.2, 0.2],
        },
        GpuVertex {
            position: [1.0, -1.0, -1.0],
            color: [0.2, 1.0, 0.2],
        },
        GpuVertex {
            position: [1.0, 1.0, -1.0],
            color: [0.2, 0.2, 1.0],
        },
        GpuVertex {
            position: [-1.0, 1.0, -1.0],
            color: [1.0, 1.0, 0.2],
        },
        GpuVertex {
            position: [-1.0, -1.0, 1.0],
            color: [1.0, 0.2, 1.0],
        },
        GpuVertex {
            position: [1.0, -1.0, 1.0],
            color: [0.2, 1.0, 1.0],
        },
        GpuVertex {
            position: [1.0, 1.0, 1.0],
            color: [1.0, 0.7, 0.2],
        },
        GpuVertex {
            position: [-1.0, 1.0, 1.0],
            color: [0.8, 0.8, 0.8],
        },
    ];

    let indices: Vec<u16> = vec![
        // Back face
        0, 1, 2, 2, 3, 0, // Front face
        4, 6, 5, 6, 4, 7, // Left face
        4, 0, 3, 3, 7, 4, // Right face
        1, 5, 6, 6, 2, 1, // Bottom face
        4, 5, 1, 1, 0, 4, // Top face
        3, 2, 6, 6, 7, 3,
    ];

    (vertices, indices)
}
