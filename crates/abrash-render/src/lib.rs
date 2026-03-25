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

// Re-export abrash-core modules at crate root so all internal `use crate::X`
// imports from moved files continue to work without modification.
pub use abrash_core::clipping;
pub use abrash_core::culling;
pub use abrash_core::framebuffer;
pub use abrash_core::geometry;
pub use abrash_core::hiz_buffer;
pub use abrash_core::math;
pub use abrash_core::mesh;
pub use abrash_core::obj_loader;
pub use abrash_core::texture;
pub use abrash_core::time;
pub use abrash_core::utils;
pub use abrash_core::zbuffer;

pub mod ascii;
pub mod heat_vision;
pub mod particles;
pub mod post_process;
pub mod procedural;
pub mod rasterizer;
pub mod raycaster;
pub mod render_api;
pub mod scene;
pub mod skybox;

#[cfg(feature = "nova")]
pub mod experimental;
