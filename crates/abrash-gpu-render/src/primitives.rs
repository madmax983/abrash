use crate::*;
use crate::demo_app::GpuDemoConfig;
use std::fmt;
/// Validation errors for indexed triangle meshes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshValidationError {
    EmptyVertices,
    EmptyIndices,
    IndexCountNotMultipleOf3 { index_count: usize },
    IndexOutOfBounds { index: u16, vertex_count: usize },
}

impl fmt::Display for MeshValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVertices => write!(f, "mesh must contain at least one vertex"),
            Self::EmptyIndices => write!(f, "mesh must contain at least one index"),
            Self::IndexCountNotMultipleOf3 { index_count } => {
                write!(f, "index count ({index_count}) must be a multiple of 3")
            }
            Self::IndexOutOfBounds {
                index,
                vertex_count,
            } => {
                write!(
                    f,
                    "index {index} is out of bounds for vertex count {vertex_count}"
                )
            }
        }
    }
}

/// Validates that a mesh is a non-empty indexed triangle list.
///
/// # Errors
///
/// Returns a [`MeshValidationError`] when the slice data is empty, not a
/// multiple of three, or references an out-of-bounds vertex.
pub fn validate_mesh(vertices: &[GpuVertex], indices: &[u16]) -> Result<(), MeshValidationError> {
    if vertices.is_empty() {
        return Err(MeshValidationError::EmptyVertices);
    }
    if indices.is_empty() {
        return Err(MeshValidationError::EmptyIndices);
    }
    if indices.len() % 3 != 0 {
        return Err(MeshValidationError::IndexCountNotMultipleOf3 {
            index_count: indices.len(),
        });
    }
    for &index in indices {
        if usize::from(index) >= vertices.len() {
            return Err(MeshValidationError::IndexOutOfBounds {
                index,
                vertex_count: vertices.len(),
            });
        }
    }
    Ok(())
}

/// Validates that demo configuration has a drawable window size.
///
/// # Errors
///
/// Returns an error if window dimensions or camera distance limits are invalid.
pub fn validate_demo_config(config: &GpuDemoConfig) -> Result<(), &'static str> {
    if config.width == 0 || config.height == 0 {
        return Err("Window dimensions must be non-zero");
    }
    if config.min_distance <= 0.0 || config.max_distance <= 0.0 {
        return Err("Camera distance limits must be positive");
    }
    if config.min_distance > config.max_distance {
        return Err("min_distance must be less than or equal to max_distance");
    }
    if config.initial_distance < config.min_distance
        || config.initial_distance > config.max_distance
    {
        return Err("initial_distance must be within [min_distance, max_distance]");
    }
    Ok(())
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
