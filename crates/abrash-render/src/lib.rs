#![allow(
    clippy::all,
    unused_variables,
    dead_code,
    unused_imports,
    unsafe_op_in_unsafe_fn,
    unused_mut
)]
//! Software rendering engine for the Abrash 3D engine.
//!
//! Depends on `abrash-core` for foundational types.
//! Contains no platform (windowing/TUI/WASM) dependencies.

pub mod ascii;
pub mod heat_vision;
pub mod particles;
pub mod post_process;
pub mod procedural;
pub mod rasterizer;
pub mod render_api;
pub mod scene;
pub mod skybox;

#[cfg(feature = "nova")]
pub mod experimental;
