//! # Abrash Graphics Engine 🎻
//!
//! **Abrash** is a software rasterization library designed for educational purposes.
//! It implements a complete 3D pipeline from scratch, including:
//!
//! *   **Math:** Linear algebra for 3D graphics (`math`).
//! *   **Rasterization:** Scanline triangle filling, line drawing, and clipping (`rasterizer`).
//! *   **Buffers:** Framebuffer and Z-buffer management (`framebuffer`, `zbuffer`).
//! *   **Platform:** Simple windowing and event handling (`platform`).
//!
//! ## Example
//!
//! ```no_run
//! use abrash::math::Vec3;
//! use abrash::framebuffer::Framebuffer;
//! // See specific modules for more detailed examples.
//! ```

pub mod framebuffer;
pub mod light;
pub mod math;
pub mod mesh;
pub mod platform;
pub mod rasterizer;
pub mod shapes;
pub mod time;
pub mod zbuffer;

pub mod clipping;
pub mod experimental;
pub mod post_process;
