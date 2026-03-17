//! Stable engine-facing render API.
//!
//! # Quick Start
//!
//! ```
//! use abrash_render::render_api::{RenderTarget, Renderer};
//! use abrash_render::render_api::cpu_renderer::CpuRenderer;
//! use abrash_render::render_api::frame::{Frame, FrameCamera};
//! use abrash_render::render_api::material::Material;
//! use abrash_core::mesh::Mesh;
//! use abrash_core::math::{Mat4, Vec3};
//!
//! let mut renderer = CpuRenderer::new(800, 600);
//! let mut target = RenderTarget::new(800, 600).unwrap();
//!
//! let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
//! let mat_h = renderer.create_material(Material::flat(0xFFFF0000)).unwrap();
//!
//! let camera = FrameCamera::new(
//!     Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
//!     Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
//! );
//! let mut frame = Frame::new(camera);
//! frame.draw(mesh_h, mat_h, Mat4::identity());
//! renderer.render_frame(&frame, &mut target).unwrap();
//!
//! let pixels: &[u32] = target.pixels();
//! ```

pub mod cpu_renderer;
pub mod frame;
pub mod handles;
pub mod material;
pub mod renderer;
pub mod target;

pub use frame::{DrawCommand, Frame, FrameCamera, Light};
pub use handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
pub use material::{Material, ShadingMode};
pub use renderer::{RenderError, Renderer};
pub use target::RenderTarget;
