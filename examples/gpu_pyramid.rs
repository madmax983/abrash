use abrash::gpu_render::{GpuDemoConfig, GpuVertex, run_mesh_demo};

fn pyramid_mesh() -> (Vec<GpuVertex>, Vec<u16>) {
    let vertices = vec![
        GpuVertex {
            position: [-1.1, -1.0, -1.1],
            color: [0.9, 0.2, 0.2],
        },
        GpuVertex {
            position: [1.1, -1.0, -1.1],
            color: [0.2, 0.9, 0.2],
        },
        GpuVertex {
            position: [1.1, -1.0, 1.1],
            color: [0.2, 0.2, 0.9],
        },
        GpuVertex {
            position: [-1.1, -1.0, 1.1],
            color: [0.9, 0.9, 0.2],
        },
        GpuVertex {
            position: [0.0, 1.3, 0.0],
            color: [1.0, 0.7, 0.2],
        },
    ];

    let indices = vec![
        // Side faces
        0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4, // Base (two triangles)
        0, 3, 2, 2, 1, 0,
    ];

    (vertices, indices)
}

fn main() -> Result<(), String> {
    let (vertices, indices) = pyramid_mesh();

    let config = GpuDemoConfig {
        title: "Abrash GPU Pyramid (wgpu)".to_string(),
        rotation_speed: 0.55,
        initial_pitch: 0.45,
        initial_distance: 5.2,
        ..GpuDemoConfig::default()
    };

    run_mesh_demo(vertices, indices, config)
}
