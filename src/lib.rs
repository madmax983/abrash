//! # Abrash Graphics Engine 🎻
//!
//! **Abrash** is an educational software rasterization engine written in Rust.
//! It is designed to teach the fundamentals of 3D graphics rendering, including:
//!
//! *   **Linear Algebra**: Vectors, Matrices, and transformations.
//! *   **The Graphics Pipeline**: From 3D world space to 2D screen space.
//! *   **Rasterization**: Drawing triangles, lines, and handling depth.
//! *   **Shading**: Flat, Gouraud, and lighting calculations.
//!
//! ## Quick Start
//!
//! Here is a minimal example of how to set up the engine and draw a single triangle:
//!
//! ```no_run
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::Vec3;
//! use abrash::pipeline::fill_triangle_3d;
//!
//! // 1. Create buffers
//! let width = 800;
//! let height = 600;
//! let mut fb = Framebuffer::new(width, height);
//! let mut zb = ZBuffer::new(width, height);
//!
//! // 2. Define a triangle in Screen Space (usually you'd project this)
//! // Format: (Position, W-component)
//! // W=1.0 for simple screen-space usage
//! let v0 = (Vec3::new(400.0, 100.0, 0.5), 1.0);
//! let v1 = (Vec3::new(200.0, 500.0, 0.5), 1.0);
//! let v2 = (Vec3::new(600.0, 500.0, 0.5), 1.0);
//!
//! // 3. Draw!
//! // Color is 0xAARRGGBB (Red in this case)
//! let color = 0xFFFF0000;
//! fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
//!
//! // 4. (Optional) Save or Display 'fb'
//! // window.update(&fb);
//! ```
//!
//! ## Core Modules
//!
//! *   [`pipeline`]: The heart of the 3D engine. Handles triangle filling and shading.
//! *   [`math`]: Provides `Vec3` and `Mat4` types optimized for graphics.
//! *   [`framebuffer`]: Manages the pixel data.
//! *   [`zbuffer`]: Manages depth data for hidden surface removal.
//! *   [`rasterizer`]: Low-level 2D primitives (lines, unchecked triangles).
//!
//! ## Coordinate Systems
//!
//! The engine uses a Right-Handed coordinate system:
//! *   **X**: Right
//! *   **Y**: Up
//! *   **Z**: Backward (Camera looks down -Z)
//!
//! Matrices are **Row-Major** and use **Row-Vector** multiplication order (`v * M`).

pub mod framebuffer;
pub mod light;
pub mod math;
pub mod mesh;
pub mod pipeline;
pub mod platform;
pub mod rasterizer;
pub mod shapes;
pub mod time;
pub mod zbuffer;

pub mod post_process;

pub mod experimental;
