//! Frame submission types — the contract between scene logic and the renderer.

use crate::math::{Mat4, Vec3};
use crate::render_api::handles::{MaterialHandle, MeshHandle};

/// A directional light source (infinite distance, parallel rays).
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    /// Normalized direction vector (points FROM light TO scene).
    pub direction: Vec3,
    /// Light color as `0xRRGGBB` (no alpha).
    pub color: u32,
    /// Intensity multiplier (0.0 = off, 1.0 = normal).
    pub intensity: f32,
}

/// A point light source.
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    /// World-space position.
    pub position: Vec3,
    /// Light color as `0xRRGGBB`.
    pub color: u32,
    /// Intensity.
    pub intensity: f32,
    /// Attenuation radius (light fades to zero at this distance).
    pub radius: f32,
}

/// Light source in the scene.
#[derive(Debug, Clone, Copy)]
pub enum Light {
    /// A directional light source (infinite distance, parallel rays).
    Directional(DirectionalLight),
    /// A point light source.
    Point(PointLight),
}

/// Camera definition for a frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameCamera {
    /// View matrix (world -> view space).
    pub view: Mat4,
    /// Projection matrix (view -> clip space).
    pub projection: Mat4,
}

impl FrameCamera {
    /// Create a camera from view and projection matrices.
    #[must_use]
    pub const fn new(view: Mat4, projection: Mat4) -> Self {
        Self { view, projection }
    }
}

/// A single draw command: render this mesh with this material at this transform.
#[derive(Debug, Clone, Copy)]
pub struct DrawCommand {
    /// Handle to the mesh to render.
    pub mesh: MeshHandle,
    /// Handle to the material to use.
    pub material: MaterialHandle,
    /// Model transform (local -> world space).
    pub transform: Mat4,
}

/// A complete frame to be rendered.
///
/// The `Frame` is the unit of work submitted to [`crate::render_api::cpu_renderer::CpuRenderer::render_frame`].
/// It is backend-agnostic — both CPU and GPU renderers consume the same `Frame`.
pub struct Frame {
    /// Camera for this frame.
    pub camera: FrameCamera,
    /// Active lights.
    pub lights: Vec<Light>,
    /// Draw commands (mesh + material + transform).
    pub commands: Vec<DrawCommand>,
    /// If `Some`, clear the render target to this color before rendering.
    /// If `None`, render over the existing contents.
    pub clear_color: Option<u32>,
}

impl Frame {
    /// Create an empty frame with the given camera. Clears to black by default.
    #[must_use]
    pub const fn new(camera: FrameCamera) -> Self {
        Self {
            camera,
            lights: Vec::new(),
            commands: Vec::new(),
            clear_color: Some(0xFF00_0000),
        }
    }

    /// ⚡ Bolt: Create an empty frame, pre-allocating the underlying vectors.
    /// This drastically reduces heap reallocations per frame when drawing many objects.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_render::render_api::frame::{Frame, FrameCamera};
    /// use abrash_core::math::Mat4;
    /// let camera = FrameCamera::new(Mat4::identity(), Mat4::identity());
    /// let frame = Frame::with_capacity(camera, 100, 10);
    /// ```
    ///
    /// # Panics
    /// Panics if the requested capacity exceeds the maximum allowed allocation size (`isize::MAX` bytes).
    #[must_use]
    pub fn with_capacity(camera: FrameCamera, num_commands: usize, num_lights: usize) -> Self {
        // WARDEN DEFENSE: Prevent capacity overflow panics
        assert!(
            num_lights <= (isize::MAX as usize) / std::mem::size_of::<Light>(),
            "capacity overflow"
        );
        assert!(
            num_commands <= (isize::MAX as usize) / std::mem::size_of::<DrawCommand>(),
            "capacity overflow"
        );
        Self {
            camera,
            lights: Vec::with_capacity(num_lights),
            commands: Vec::with_capacity(num_commands),
            clear_color: Some(0xFF00_0000),
        }
    }

    /// Record a draw command in this frame.
    ///
    /// This adds a mesh to the rendering queue for the frame, specifying its material
    /// and its transformation matrix in the world.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_render::render_api::frame::{Frame, FrameCamera};
    /// use abrash_core::math::Mat4;
    /// use abrash_render::render_api::Handle;
    /// let camera = FrameCamera::new(Mat4::identity(), Mat4::identity());
    /// let mut frame = Frame::new(camera);
    /// let mesh_h = Handle::from_raw_parts(0, 0);
    /// let mat_h = Handle::from_raw_parts(0, 0);
    /// frame.draw(mesh_h, mat_h, Mat4::identity());
    /// ```
    pub fn draw(&mut self, mesh: MeshHandle, material: MaterialHandle, transform: Mat4) {
        self.commands.push(DrawCommand {
            mesh,
            material,
            transform,
        });
    }

    /// Add a light source to the frame.
    ///
    /// Affects how materials with shading enabled will be illuminated.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_render::render_api::frame::{Frame, FrameCamera, Light, DirectionalLight};
    /// use abrash_core::math::{Mat4, Vec3};
    /// let camera = FrameCamera::new(Mat4::identity(), Mat4::identity());
    /// let mut frame = Frame::new(camera);
    /// frame.add_light(Light::Directional(DirectionalLight { direction: Vec3::new(0.0, -1.0, 0.0), color: 0xFFFF_FFFF, intensity: 1.0 }));
    /// ```
    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_api::handles::Handle;

    fn test_camera() -> FrameCamera {
        FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 5.0, 10.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
        )
    }

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new(test_camera());
        assert!(frame.commands.is_empty());
        assert!(frame.lights.is_empty());
        assert_eq!(frame.clear_color, Some(0xFF00_0000));
    }

    #[test]
    fn test_frame_add_draw_command() {
        let mut frame = Frame::new(test_camera());
        let mesh: MeshHandle = Handle::new(0, 0);
        let mat: MaterialHandle = Handle::new(1, 0);
        let transform = Mat4::translation(1.0, 2.0, 3.0);

        frame.draw(mesh, mat, transform);
        assert_eq!(frame.commands.len(), 1);
        assert_eq!(frame.commands[0].mesh, mesh);
        assert_eq!(frame.commands[0].material, mat);
    }

    #[test]
    fn test_frame_add_lights() {
        let mut frame = Frame::new(test_camera());
        frame.add_light(Light::Directional(DirectionalLight {
            direction: Vec3::new(0.0, -1.0, 0.0),
            color: 0x00FF_FFFF,
            intensity: 1.0,
        }));
        frame.add_light(Light::Point(PointLight {
            position: Vec3::new(5.0, 5.0, 5.0),
            color: 0x00FF_0000,
            intensity: 2.0,
            radius: 10.0,
        }));
        assert_eq!(frame.lights.len(), 2);
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn test_frame_capacity_overflow() {
        let cam = test_camera();
        // Simulating absurd memory bounds from a fuzzed command buffer length
        let _frame = Frame::with_capacity(cam, usize::MAX, usize::MAX);
    }
}
