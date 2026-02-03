//! # Abrash 🎻
//!
//! **Abrash** is a software rasterization engine written in Rust, designed for educational purposes and retro-graphics enthusiasts.
//! It pays homage to the software rendering techniques popularized by legends like Michael Abrash.
//!
//! ## The Story
//!
//! Before GPUs ruled the world, every pixel was placed by hand (or at least by CPU cycles).
//! This crate explores that lost art, providing a complete 3D pipeline from vertex to pixel, entirely in software.
//!
//! ## Core Modules
//!
//! *   [`pipeline`]: The heart of the engine. Handles 3D transformations, clipping, and rasterization.
//! *   [`math`]: A purpose-built linear algebra library (Vec3, Mat4) optimized for graphics.
//! *   [`rasterizer`]: Low-level 2D drawing primitives (lines, circles, triangles).
//! *   [`framebuffer`]: Manages the pixel data (your canvas).
//!
//! ## Quick Start
//!
//! Want to draw a triangle? Here is the shortest path to pixels:
//!
//! ```rust
//! use abrash::framebuffer::Framebuffer;
//! use abrash::rasterizer::draw_line;
//!
//! // 1. Create a framebuffer (800x600)
//! let mut fb = Framebuffer::new(800, 600);
//!
//! // 2. Draw something (Green Line)
//! draw_line(&mut fb, 100, 100, 700, 500, 0xFF00FF00);
//!
//! // 3. (Optional) Save or display the framebuffer
//! // (The windowing system is handled by the `platform` module)
//! ```
//!
//! ## Philosophy
//!
//! > "If it isn't documented, it doesn't exist."
//!
//! This crate aims to be fully documented, explaining not just the *what*, but the *why* of software rendering.

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
