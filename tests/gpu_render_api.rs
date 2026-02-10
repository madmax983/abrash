#![cfg(feature = "gpu-render")]

use abrash::gpu_render::{
    GpuDemoConfig, GpuVertex, MeshValidationError, validate_demo_config, validate_mesh,
};

#[test]
fn validate_mesh_accepts_unit_cube() {
    let (vertices, indices) = abrash::gpu_render::unit_cube_mesh();
    assert!(validate_mesh(&vertices, &indices).is_ok());
}

#[test]
fn validate_mesh_rejects_non_triangle_index_count() {
    let vertices = vec![
        GpuVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        GpuVertex {
            position: [1.0, 0.0, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        GpuVertex {
            position: [0.0, 1.0, 0.0],
            color: [0.0, 0.0, 1.0],
        },
    ];
    let indices = vec![0, 1, 2, 0];

    assert_eq!(
        validate_mesh(&vertices, &indices),
        Err(MeshValidationError::IndexCountNotMultipleOf3 {
            index_count: 4
        })
    );
}

#[test]
fn validate_mesh_rejects_out_of_bounds_index() {
    let vertices = vec![GpuVertex {
        position: [0.0, 0.0, 0.0],
        color: [1.0, 1.0, 1.0],
    }];
    let indices = vec![0, 1, 0];

    assert_eq!(
        validate_mesh(&vertices, &indices),
        Err(MeshValidationError::IndexOutOfBounds {
            index: 1,
            vertex_count: 1,
        })
    );
}

#[test]
fn validate_demo_config_rejects_zero_dimensions() {
    let config = GpuDemoConfig {
        width: 0,
        ..GpuDemoConfig::default()
    };

    assert_eq!(
        validate_demo_config(&config),
        Err("Window dimensions must be non-zero")
    );
}
