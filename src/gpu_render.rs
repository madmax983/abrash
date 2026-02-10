//! Compatibility wrapper around the external `abrash-gpu-render` crate.

#![cfg(feature = "gpu-render")]

pub use abrash_gpu_render::{GpuTriangle, GpuVertex, run_gpu_cube, unit_cube_mesh};
