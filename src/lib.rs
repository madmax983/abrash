//! Abrash: A Software Rasterization Engine 🎻
//!
//! **Abrash** is an educational graphics engine written in Rust, paying homage to the
//! software rendering techniques of the 90s. It features a complete 3D pipeline,
//! perspective-correct texturing, and multiple rasterization backends.
//!
//! # Core Modules
//!
//! *   [`math`]: Custom linear algebra types (`Vec3`, `Mat4`) optimized for 3D graphics.
//! *   [`rasterizer`]: Low-level triangle rasterization (flat, Gouraud, textured).
//! *   [`framebuffer`]: Pixel buffer management.
//! *   [`zbuffer`]: Depth buffer for visibility testing.
//! *   [`tile_renderer`]: High-performance tile-based rasterizer for large resolutions.
//!
//! # Quick Start
//!
//! ```rust
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::Vec3;
//! use abrash::rasterizer::fill_triangle_3d;
//!
//! // 1. Create buffers
//! let width = 640;
//! let height = 480;
//! let mut fb = Framebuffer::new(width, height).unwrap();
//! let mut zb = ZBuffer::new(width, height).unwrap();
//!
//! // 2. Define a triangle (clip space coordinates + W component)
//! let v0 = (Vec3::new(0.0, 0.5, 2.0), 2.0);
//! let v1 = (Vec3::new(-0.5, -0.5, 2.0), 2.0);
//! let v2 = (Vec3::new(0.5, -0.5, 2.0), 2.0);
//! let color = 0xFFFF_0000; // Red (ARGB)
//!
//! // 3. Draw!
//! fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
//!
//! // 4. Check a pixel
//! assert_eq!(fb.get_pixel(320, 240), Some(0xFFFF_0000));
//! ```

pub mod framebuffer;
pub mod math;
pub mod mesh;
pub mod platform;
pub mod rasterizer;
pub mod time;
pub mod zbuffer;

pub mod clipping;
pub mod hiz_buffer;
pub mod obj_loader;
pub mod tile_renderer;
