#![allow(
    clippy::all,
    unused_variables,
    dead_code,
    unused_imports,
    unsafe_op_in_unsafe_fn,
    unused_mut
)]
//! Core math, geometry, and buffer types for the Abrash rendering engine.

pub mod clipping;
pub mod culling;
pub mod framebuffer;
pub mod geometry;
pub mod hiz_buffer;
pub mod math;
pub mod mesh;
pub mod obj_loader;
pub mod texture;
pub mod time;
pub mod utils;
pub mod zbuffer;
