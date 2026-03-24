//! The `Renderer` trait — core engine contract.

use crate::mesh::Mesh;
use crate::render_api::frame::Frame;
use crate::render_api::handles::{MaterialHandle, MeshHandle, TextureHandle};
use crate::render_api::material::Material;
use crate::render_api::target::RenderTarget;
use crate::texture::Texture;

/// Errors from renderer operations.
#[derive(Debug)]
pub enum RenderError {
    /// A handle referenced a resource that no longer exists.
    StaleHandle(&'static str),
    /// The mesh data was invalid (e.g., index out of bounds).
    InvalidMesh(String),
    /// The texture data was invalid.
    InvalidTexture(String),
    /// Internal renderer error.
    Internal(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleHandle(kind) => write!(f, "stale {kind} handle"),
            Self::InvalidMesh(msg) => write!(f, "invalid mesh: {msg}"),
            Self::InvalidTexture(msg) => write!(f, "invalid texture: {msg}"),
            Self::Internal(msg) => write!(f, "renderer error: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// The core renderer trait.
///
/// Implementations manage resource pools and render `Frame`s into `RenderTarget`s.
/// External consumers program against this trait, not against `fill_triangle_*`
/// or `TileRenderer` directly.
///
/// # Lifecycle
///
/// 1. Create renderer
/// 2. Upload resources (meshes, textures, materials)
/// 3. Each frame: build a `Frame`, call `render_frame`
/// 4. Read pixels from `RenderTarget` for display or export
/// 5. Clean up resources when done
///
/// # Examples
///
/// ```
/// use abrash_render::render_api::{RenderTarget, Renderer};
/// use abrash_render::render_api::cpu_renderer::CpuRenderer;
/// use abrash_render::render_api::frame::{Frame, FrameCamera};
/// use abrash_render::render_api::material::Material;
/// use abrash_core::mesh::Mesh;
/// use abrash_core::math::{Mat4, Vec3};
///
/// // Create our blank canvas and our artist (the renderer)
/// let mut target = RenderTarget::new(800, 600).unwrap();
/// let mut renderer = CpuRenderer::new(800, 600);
///
/// // Hand the artist a mesh and a color, getting back "claim checks" (handles)
/// let my_cube = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
/// let my_paint = renderer.create_material(Material::flat(0xFFFF_0000)).unwrap(); // Red
///
/// // Tell the artist where to stand
/// let camera = FrameCamera::new(
///     Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
///     Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
/// );
///
/// // Describe the scene
/// let mut frame = Frame::new(camera);
/// frame.draw(my_cube, my_paint, Mat4::identity());
///
/// // Render!
/// renderer.render_frame(&frame, &mut target).unwrap();
///
/// // Clean up when we're done with the resources
/// renderer.destroy_mesh(my_cube);
/// renderer.destroy_material(my_paint);
/// ```
pub trait Renderer {
    /// Upload a mesh and return a handle.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidMesh`] if the mesh data is malformed.
    fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, RenderError>;

    /// Update an existing mesh resource with new data.
    ///
    /// This is used for per-frame updates like vertex skinning.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::StaleHandle`] if the handle is invalid.
    /// Returns [`RenderError::InvalidMesh`] if the mesh data is malformed.
    fn update_mesh(&mut self, handle: MeshHandle, mesh: &Mesh) -> Result<(), RenderError>;

    /// Upload a texture and return a handle.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidTexture`] if the texture data is malformed.
    fn create_texture(&mut self, texture: &Texture) -> Result<TextureHandle, RenderError>;

    /// Register a material and return a handle.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Internal`] if the material cannot be registered.
    fn create_material(&mut self, material: Material) -> Result<MaterialHandle, RenderError>;

    /// Render a frame into the render target.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::StaleHandle`] if any handle in the frame is invalid,
    /// or [`RenderError::Internal`] for backend-specific failures.
    fn render_frame(&mut self, frame: &Frame, target: &mut RenderTarget)
    -> Result<(), RenderError>;

    /// Release a mesh resource.
    fn destroy_mesh(&mut self, handle: MeshHandle);

    /// Release a texture resource.
    fn destroy_texture(&mut self, handle: TextureHandle);

    /// Release a material resource.
    fn destroy_material(&mut self, handle: MaterialHandle);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use crate::mesh::Mesh;
    use crate::render_api::frame::FrameCamera;

    /// A null renderer that accepts all operations but renders nothing.
    struct NullRenderer;

    impl Renderer for NullRenderer {
        fn create_mesh(&mut self, _mesh: &Mesh) -> Result<MeshHandle, RenderError> {
            Ok(MeshHandle::new(0, 0))
        }
        fn update_mesh(&mut self, _handle: MeshHandle, _mesh: &Mesh) -> Result<(), RenderError> {
            Ok(())
        }
        fn create_texture(&mut self, _texture: &Texture) -> Result<TextureHandle, RenderError> {
            Ok(TextureHandle::new(0, 0))
        }
        fn create_material(&mut self, _material: Material) -> Result<MaterialHandle, RenderError> {
            Ok(MaterialHandle::new(0, 0))
        }
        fn render_frame(
            &mut self,
            _frame: &Frame,
            _target: &mut RenderTarget,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn destroy_mesh(&mut self, _handle: MeshHandle) {}
        fn destroy_texture(&mut self, _handle: TextureHandle) {}
        fn destroy_material(&mut self, _handle: MaterialHandle) {}
    }

    #[test]
    fn test_null_renderer_creates_resources() {
        let mut renderer = NullRenderer;
        let mesh = Mesh::cube(1.0);
        let handle = renderer.create_mesh(&mesh).unwrap();
        assert_eq!(handle.index, 0);
    }

    #[test]
    fn test_null_renderer_renders_frame() {
        let mut renderer = NullRenderer;
        let mut target = RenderTarget::new(100, 100).unwrap();
        let camera = FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 5.0, 10.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 1.0, 0.1, 100.0),
        );
        let frame = Frame::new(camera);
        assert!(renderer.render_frame(&frame, &mut target).is_ok());
    }

    #[test]
    fn test_render_error_display() {
        assert_eq!(
            format!("{}", RenderError::StaleHandle("mesh")),
            "stale mesh handle"
        );
        assert_eq!(
            format!(
                "{}",
                RenderError::InvalidMesh("index out of bounds".to_string())
            ),
            "invalid mesh: index out of bounds"
        );
    }
}
