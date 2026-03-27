//! # The Render API 🎭
//!
//! Stable engine-facing render API.
//!
//! The graphics pipeline is complex, and sending raw vertices to a low-level rasterizer every
//! frame is exhausting. The Render API solves this by providing a high-level, state-managed
//! interface for defining what you want to draw, rather than *how* to draw it.
//!
//! ## The Three Pillars of Rendering
//!
//! 1. **Resources (`handles.rs`)**: Textures, Meshes, and Materials are heavy. Instead of copying them, you upload them to the [`crate::render_api::cpu_renderer::CpuRenderer`] once and get back a lightweight, type-safe [`Handle`]. Handles are generation-checked to prevent use-after-free errors.
//! 2. **The `Frame` (`frame.rs`)**: A `Frame` is your declarative "shopping list" for a single screen update. You give it a [`FrameCamera`] and record [`DrawCommand`]s (which pair a Mesh Handle, a Material Handle, and a Transform).
//! 3. **The `CpuRenderer` (`cpu_renderer.rs`)**: The engine that executes your `Frame`. It takes your high-level commands, looks up the heavy resources using your Handles, and pushes pixels to the [`RenderTarget`].
//!
//! ## Quick Start
//!
//! ```
//! use abrash_render::render_api::RenderTarget;
//! use abrash_render::render_api::cpu_renderer::CpuRenderer;
//! use abrash_render::render_api::frame::{Frame, FrameCamera};
//! use abrash_render::render_api::material::Material;
//! use abrash_core::mesh::Mesh;
//! use abrash_core::math::{Mat4, Vec3};
//!
//! // 1. Initialize the Stage (The Renderer and the Target Buffer)
//! let mut renderer = CpuRenderer::new(800, 600);
//! let mut target = RenderTarget::new(800, 600).unwrap();
//!
//! // 2. Load Actors (Resources) into the Renderer's memory pool
//! let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
//! let mat_h = renderer.create_material(Material::flat(0xFFFF_0000)).unwrap();
//!
//! // 3. Set the Camera
//! let camera = FrameCamera::new(
//!     Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
//!     Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
//! );
//!
//! // 4. Action! Record the Frame and ask the Renderer to paint it.
//! let mut frame = Frame::new(camera);
//! frame.draw(mesh_h, mat_h, Mat4::identity()); // Draw the cube at the origin
//! renderer.render_frame(&frame, &mut target).unwrap();
//!
//! // 5. The final masterpiece is now in the target buffer.
//! let pixels: &[u32] = target.pixels();
//! ```

pub mod cpu_renderer;
pub mod draw_list;
pub mod frame;
pub mod handles;
pub mod material;
pub mod target;

pub use cpu_renderer::RenderError;
pub use draw_list::{DrawBatch, DrawList};
pub use frame::{DrawCommand, Frame, FrameCamera, Light};
pub use handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
pub use material::{Material, ShadingMode};
pub use target::RenderTarget;
