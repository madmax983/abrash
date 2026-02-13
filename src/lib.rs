//! # Abrash - A Software Rasterizer in Rust
//!
//! Abrash is a high-performance software rasterizer built for educational purposes and retro-style rendering.
//! It implements a complete 3D graphics pipeline from scratch, including:
//!
//! *   **Vertex Processing**: Transformations, Clipping, and Projection.
//! *   **Rasterization**: Scanline-based triangle filling with perspective-correct texture mapping.
//! *   **Shading**: Flat, Gouraud, and Phong shading models.
//! *   **Post-Processing**: Screen-space effects like Sepia, Grayscale, and Scanlines.
//!
//! ## The Graphics Pipeline
//!
//! The rendering process generally follows these steps:
//!
//! 1.  **Load Assets**: Use [`obj_loader`] to load 3D models and [`texture`] to load images.
//! 2.  **Transform**: Use [`math::Mat4`] to transform vertices from Model Space to World Space, View Space, and finally Clip Space.
//! 3.  **Clip**: Primitives are clipped against the view frustum to ensure they are within the visible volume.
//! 4.  **Rasterize**: The [`rasterizer`] module converts 3D triangles into 2D pixels on the [`framebuffer::Framebuffer`].
//! 5.  **Shade**: Pixels are colored based on lighting, textures, and material properties.
//!
//! ## Getting Started
//!
//! Here is a conceptual example of a render loop:
//!
//! ```no_run
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::{Mat4, Vec3};
//! use abrash::rasterizer::fill_triangle_3d;
//!
//! // 1. Initialize Buffers
//! let width = 800;
//! let height = 600;
//! let mut fb = Framebuffer::new(width, height).unwrap();
//! let mut zb = ZBuffer::new(width, height).unwrap();
//!
//! // 2. Set up Camera
//! let eye = Vec3::new(0.0, 0.0, 5.0);
//! let target = Vec3::new(0.0, 0.0, 0.0);
//! let up = Vec3::new(0.0, 1.0, 0.0);
//! let view = Mat4::look_at(eye, target, up);
//! let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
//! let view_proj = proj * view;
//!
//! // 3. Render Loop (Simulated)
//! fb.clear(0xFF000000);
//! zb.clear();
//!
//! // Define a triangle
//! let v0 = (Vec3::new(0.0, 0.5, 0.0), 1.0);
//! let v1 = (Vec3::new(-0.5, -0.5, 0.0), 1.0);
//! let v2 = (Vec3::new(0.5, -0.5, 0.0), 1.0);
//!
//! // Transform vertices (Simplified)
//! // In a real engine, you would transform all vertices in a mesh.
//! // Here we just pass them to the rasterizer which expects Clip Space coordinates if they are already transformed.
//! // Note: fill_triangle_3d expects (Vec3, w).
//!
//! // ... (Transformation logic would go here) ...
//!
//! // Rasterize
//! fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFF0000);
//!
//! // 4. Display Framebuffer
//! // (Platform-specific windowing code goes here)
//! ```
//!
//! ## Key Modules
//!
//! *   [`rasterizer`]: The core drawing algorithms.
//! *   [`math`]: Matrix and Vector math libraries.
//! *   [`framebuffer`]: Pixel storage and manipulation.
//! *   [`obj_loader`]: Wavefront OBJ parser.

pub mod framebuffer;
pub mod math;
pub mod mesh;
pub mod platform;
pub mod rasterizer;
pub mod time;
pub mod zbuffer;

pub mod clipping;
pub mod experimental;
pub mod obj_loader;
pub mod post_process;
pub mod texture;

#[cfg(feature = "gpu-render")]
pub mod gpu_render;
