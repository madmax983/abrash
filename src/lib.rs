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
//! Here is a minimal working example of a render loop:
//!
//! ```
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
//!
//! // Combine matrices: View * Projection (Row-Major: Vertex * View * Projection)
//! // Note: This library uses row vectors (v * M), so transformations are applied left-to-right.
//! let view_proj = view * proj;
//!
//! // 3. Render Loop (Minimal)
//! fb.clear(0xFF000000); // Clear to Black
//! zb.clear();
//!
//! // Define a triangle in Local Space
//! let v0_local = Vec3::new(0.0, 0.5, 0.0);
//! let v1_local = Vec3::new(-0.5, -0.5, 0.0);
//! let v2_local = Vec3::new(0.5, -0.5, 0.0);
//!
//! // Transform vertices to Homogeneous Clip Space.
//! // transform_point returns `(Vec3, f32)` where the f32 is the 'w' component.
//! // The rasterizer needs 'w' for perspective-correct interpolation.
//! let v0_clip = view_proj.transform_point(v0_local);
//! let v1_clip = view_proj.transform_point(v1_local);
//! let v2_clip = view_proj.transform_point(v2_local);
//!
//! // Rasterize the triangle
//! // Arguments: Framebuffer, ZBuffer, Vertex0, Vertex1, Vertex2, Color (ARGB)
//! fill_triangle_3d(&mut fb, &mut zb, v0_clip, v1_clip, v2_clip, 0xFFFF0000);
//!
//! // 4. Display Framebuffer
//! // (Platform-specific windowing code goes here)
//! // For verification, check that the center pixel is red:
//! let center_pixel = fb.get_pixel(400, 300);
//! assert_eq!(center_pixel, Some(0xFFFF0000));
//! ```
//!
//! ## Key Modules
//!
//! *   [`rasterizer`]: The core drawing algorithms.
//! *   [`math`]: Matrix and Vector math libraries.
//! *   [`framebuffer`]: Pixel storage and manipulation.
//! *   [`zbuffer`]: Depth buffering for correct occlusion.
//! *   [`texture`]: Texture loading and sampling.
//! *   [`obj_loader`]: Wavefront OBJ parser.

pub mod framebuffer;
pub mod math;
pub mod mesh;
pub mod platform;
pub mod rasterizer;
pub mod time;
pub mod utils;
pub mod zbuffer;

pub mod ascii;
pub mod clipping;
pub mod culling;
pub mod heat_vision;
pub mod obj_loader;
pub mod particles;
pub mod post_process;
pub mod procedural;
pub mod skybox;
pub mod texture;

#[cfg(feature = "gpu-render")]
pub mod gpu_render;
