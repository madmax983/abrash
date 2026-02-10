//! Compatibility wrapper around the external `abrash-gpu-render` crate.

#![cfg(feature = "gpu-render")]

pub use abrash_gpu_render::{
    GpuDemoConfig, GpuTriangle, GpuVertex, MeshValidationError, run_gpu_cube,
    run_gpu_cube_with_config, unit_cube_mesh, validate_demo_config, validate_mesh,
};
