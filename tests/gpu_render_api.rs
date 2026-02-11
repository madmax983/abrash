#![cfg(feature = "gpu-render")]

use abrash::gpu_render::{
    GpuDemoConfig, GpuInteractionController, GpuOffscreenBench, GpuOffscreenBenchConfig,
    GpuVertex, MeshValidationError, validate_demo_config, validate_mesh,
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
        Err(MeshValidationError::IndexCountNotMultipleOf3 { index_count: 4 })
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

#[test]
fn validate_demo_config_rejects_invalid_distance_range() {
    let config = GpuDemoConfig {
        min_distance: 5.0,
        max_distance: 4.0,
        ..GpuDemoConfig::default()
    };

    assert_eq!(
        validate_demo_config(&config),
        Err("min_distance must be less than or equal to max_distance")
    );
}

#[test]
fn run_mesh_demo_is_exposed() {
    let _ = abrash::gpu_render::run_mesh_demo
        as fn(Vec<GpuVertex>, Vec<u16>, GpuDemoConfig) -> Result<(), String>;
}

#[test]
fn interaction_controller_updates_yaw_from_keyboard() {
    let config = GpuDemoConfig::default();
    let mut controller = GpuInteractionController::new(&config);

    controller.set_move_right(true);
    controller.update(0.5, &config);

    assert!(controller.yaw_radians() > config.initial_yaw);
}

#[test]
fn interaction_controller_clamps_distance() {
    let config = GpuDemoConfig {
        initial_distance: 3.0,
        min_distance: 2.0,
        max_distance: 4.0,
        ..GpuDemoConfig::default()
    };
    let mut controller = GpuInteractionController::new(&config);

    controller.adjust_zoom(-10.0, &config);
    assert_eq!(controller.distance(), 2.0);

    controller.adjust_zoom(10.0, &config);
    assert_eq!(controller.distance(), 4.0);
}

#[test]
fn offscreen_benchmark_types_are_exposed() {
    let _ = std::any::type_name::<GpuOffscreenBenchConfig>();
    let _ = std::any::type_name::<GpuOffscreenBench>();
}

#[test]
fn offscreen_benchmark_config_exposes_draw_repeats() {
    let cfg = GpuOffscreenBenchConfig::default();
    assert_eq!(cfg.draw_repeats, 1);
}
