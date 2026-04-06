
#![allow(warnings)]
//! Offscreen rendering adapter — abrash embedded with no platform dependency.
//!
//! This crate is the integration proof for ADR 005. It depends only on
//! `abrash-core` (types) and `abrash-render` (renderer + API). It has zero
//! imports from `abrash::platform` or any windowing crate.
//!
//! # Embed pattern
//!
//! ```
//! use embed_demo::{AbrashBackend, EmbedScene, EmbedCamera, EmbedDraw};
//! use abrash_core::math::{Mat4, Vec3};
//! use abrash_core::mesh::Mesh;
//!
//! let mut backend = AbrashBackend::new(320, 240);
//! let cube_idx = backend.register_mesh(&Mesh::cube(1.0));
//!
//! let scene = EmbedScene {
//!     camera: EmbedCamera {
//!         position: Vec3::new(0.0, 2.0, 5.0),
//!         target:   Vec3::ZERO,
//!         fov_y:    1.2,
//!     },
//!     draws: &[EmbedDraw {
//!         mesh_index: cube_idx,
//!         transform:  Mat4::identity(),
//!         color:      0xFFFF_4444,
//!     }],
//! };
//!
//! let pixels: &[u32] = backend.render(&scene);
//! let non_bg = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
//! assert!(non_bg > 0, "cube should produce visible pixels");
//! ```

#![allow(dead_code)]

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::cpu_renderer::CpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::handles::MeshHandle;
use abrash_render::render_api::material::Material;
use abrash_render::render_api::target::RenderTarget;

// ── Public types ──────────────────────────────────────────────────────────────

/// Camera for a single rendered frame.
#[derive(Clone)]
pub struct EmbedCamera {
    /// Camera position in world space.
    pub position: Vec3,
    /// Look-at target in world space.
    pub target: Vec3,
    /// Vertical field-of-view in radians (e.g. `std::f32::consts::FRAC_PI_3`).
    pub fov_y: f32,
}

/// A single draw call in a rendered scene.
pub struct EmbedDraw {
    /// Index returned by [`AbrashBackend::register_mesh`].
    pub mesh_index: usize,
    /// Model transform (local → world space).
    pub transform: Mat4,
    /// Flat surface color (0xAARRGGBB).
    pub color: u32,
}

/// A complete scene description for one frame.
pub struct EmbedScene<'a> {
    /// Camera parameters.
    pub camera: EmbedCamera,
    /// Draw commands. Ordering matters for correct z-buffering.
    pub draws: &'a [EmbedDraw],
}

// ── Backend ───────────────────────────────────────────────────────────────────

/// Offscreen rendering backend.
///
/// Renders 3D scenes into an internal pixel buffer. The caller reads pixels via
/// [`pixels`](AbrashBackend::pixels) and routes them to whatever display or file
/// output it prefers. This struct has **no platform dependency** — it will not
/// link `windows-sys`, `crossterm`, `ratatui`, or `ratzilla`.
///
/// # Lifecycle
///
/// 1. `AbrashBackend::new(width, height)` — create the backend
/// 2. `register_mesh(&mesh)` — upload geometry, receive an index
/// 3. Loop: build an [`EmbedScene`] and call [`render`](Self::render) each frame
/// 4. Read [`pixels`](Self::pixels) and hand them to the host's display layer
pub struct AbrashBackend {
    renderer: CpuRenderer,
    target: RenderTarget,
    /// Stable mesh handles indexed by registration order.
    mesh_handles: Vec<MeshHandle>,
    aspect: f32,
}

impl AbrashBackend {
    /// Create a new backend for the given pixel dimensions.
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` are zero or exceed `i32::MAX`.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "dimensions must be non-zero");
        let renderer = CpuRenderer::new(width, height);
        let target = RenderTarget::new(width, height).expect("valid dimensions");
        Self {
            renderer,
            target,
            mesh_handles: Vec::new(),
            aspect: width as f32 / height as f32,
        }
    }

    /// Upload a mesh and return its index for use in [`EmbedDraw::mesh_index`].
    ///
    /// Meshes are stable for the lifetime of the backend — you can register them
    /// once at startup and reuse the index every frame.
    ///
    /// # Panics
    ///
    /// Panics if the mesh has out-of-bounds triangle indices.
    pub fn register_mesh(&mut self, mesh: &Mesh) -> usize {
        let handle = self
            .renderer
            .create_mesh(mesh)
            .expect("mesh indices must be in bounds");
        let idx = self.mesh_handles.len();
        self.mesh_handles.push(handle);
        idx
    }

    /// Render a scene and return the pixel buffer.
    ///
    /// Returns a slice of `width × height` pixels in 0xAARRGGBB format.
    /// The slice is valid until the next call to `render`.
    ///
    /// Materials are transient: they are created for each draw call and freed
    /// after the frame, so the resource pool never grows unboundedly.
    ///
    /// # Panics
    ///
    /// Panics if a `draw.mesh_index` is out of bounds for registered meshes.
    pub fn render(&mut self, scene: &EmbedScene<'_>) -> &[u32] {
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(scene.camera.position, scene.camera.target, up);
        let proj = Mat4::perspective(scene.camera.fov_y, self.aspect, 0.1, 1000.0);
        let camera = FrameCamera::new(view, proj);

        // ⚡ Bolt: Use Frame::with_capacity to avoid per-frame vector allocations
        let mut frame = Frame::with_capacity(camera, scene.draws.len(), 0);

        // Collect transient material handles so we can destroy them after the frame.
        let mut mat_handles = Vec::with_capacity(scene.draws.len());

        for draw in scene.draws {
            assert!(
                draw.mesh_index < self.mesh_handles.len(),
                "mesh_index {} out of range (have {} registered meshes)",
                draw.mesh_index,
                self.mesh_handles.len()
            );
            let mesh_handle = self.mesh_handles[draw.mesh_index];
            let mat_handle = self
                .renderer
                .create_material(Material::flat(draw.color))
                .expect("material creation");
            mat_handles.push(mat_handle);
            frame.draw(mesh_handle, mat_handle, draw.transform);
        }

        self.renderer
            .render_frame(&frame, &mut self.target)
            .expect("render_frame");

        // Free transient materials — pool stays bounded.
        for mat_handle in mat_handles {
            self.renderer.destroy_material(mat_handle);
        }

        self.target.pixels()
    }

    /// Read-only access to the pixel buffer without re-rendering.
    ///
    /// Returns the last frame's pixels, or a cleared buffer if `render` has
    /// not been called yet.
    #[must_use]
    pub fn pixels(&self) -> &[u32] {
        self.target.pixels()
    }

    /// Width of the render target in pixels.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn width(&self) -> u32 {
        self.target.width()
    }

    /// Height of the render target in pixels.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn height(&self) -> u32 {
        self.target.height()
    }

    /// Access the underlying [`RenderTarget`] for post-processing or export.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn target(&self) -> &RenderTarget {
        &self.target
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_3;

    fn front_camera() -> EmbedCamera {
        EmbedCamera {
            position: Vec3::new(0.0, 1.5, 4.0),
            target: Vec3::ZERO,
            fov_y: FRAC_PI_3,
        }
    }

    #[test]
    fn test_empty_scene_produces_clear_color() {
        let mut backend = AbrashBackend::new(100, 100);
        let pixels = backend.render(&EmbedScene {
            camera: front_camera(),
            draws: &[],
        });
        assert!(
            pixels.iter().all(|&p| p == 0xFF00_0000),
            "empty scene should be all clear-color black"
        );
    }

    #[test]
    fn test_cube_produces_visible_pixels() {
        let mut backend = AbrashBackend::new(200, 200);
        let cube = backend.register_mesh(&Mesh::cube(1.0));

        let pixels = backend.render(&EmbedScene {
            camera: front_camera(),
            draws: &[EmbedDraw {
                mesh_index: cube,
                transform: Mat4::identity(),
                color: 0xFFFF_4444,
            }],
        });

        let visible = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
        assert!(
            visible > 100,
            "cube should produce visible pixels, got {visible}"
        );
    }

    #[test]
    fn test_two_objects_both_visible() {
        let mut backend = AbrashBackend::new(200, 200);
        let cube = backend.register_mesh(&Mesh::cube(1.0));

        let camera = EmbedCamera {
            position: Vec3::new(0.0, 2.0, 8.0),
            target: Vec3::ZERO,
            fov_y: FRAC_PI_3,
        };

        let pixels = backend.render(&EmbedScene {
            camera,
            draws: &[
                EmbedDraw {
                    mesh_index: cube,
                    transform: Mat4::translation(-1.5, 0.0, 0.0),
                    color: 0xFFFF_0000, // red
                },
                EmbedDraw {
                    mesh_index: cube,
                    transform: Mat4::translation(1.5, 0.0, 0.0),
                    color: 0xFF00_FF00, // green
                },
            ],
        });

        let red_pixels = pixels
            .iter()
            .filter(|&&p| (p >> 16) & 0xFF == 0xFF && (p >> 8) & 0xFF == 0)
            .count();
        let green_pixels = pixels
            .iter()
            .filter(|&&p| (p >> 8) & 0xFF == 0xFF && (p >> 16) & 0xFF == 0)
            .count();
        assert!(red_pixels > 0, "should have red pixels from left cube");
        assert!(green_pixels > 0, "should have green pixels from right cube");
    }

    #[test]
    fn test_mesh_reused_across_frames() {
        let mut backend = AbrashBackend::new(100, 100);
        let cube = backend.register_mesh(&Mesh::cube(1.0));

        let camera = EmbedCamera {
            position: Vec3::new(0.0, 1.5, 4.0),
            target: Vec3::ZERO,
            fov_y: FRAC_PI_3,
        };

        // Render 3 frames with different rotations — mesh handle stays valid.
        for i in 0..3u32 {
            let angle = i as f32 * std::f32::consts::FRAC_PI_4;
            let pixels = backend.render(&EmbedScene {
                camera: EmbedCamera {
                    position: camera.position,
                    target: camera.target,
                    fov_y: camera.fov_y,
                },
                draws: &[EmbedDraw {
                    mesh_index: cube,
                    transform: Mat4::rotation_y(angle),
                    color: 0xFFAA_BBCC,
                }],
            });
            let visible = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
            assert!(visible > 0, "frame {i} should have visible pixels");
        }
    }

    #[test]
    fn test_culled_object_behind_camera_produces_no_pixels() {
        let mut backend = AbrashBackend::new(100, 100);
        let cube = backend.register_mesh(&Mesh::cube(1.0));

        // Camera looks toward -z; object is far behind at +z=500
        let pixels = backend.render(&EmbedScene {
            camera: EmbedCamera {
                position: Vec3::new(0.0, 0.0, 5.0),
                target: Vec3::ZERO,
                fov_y: FRAC_PI_3,
            },
            draws: &[EmbedDraw {
                mesh_index: cube,
                transform: Mat4::translation(0.0, 0.0, 500.0),
                color: 0xFFFF_0000,
            }],
        });

        let visible = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
        assert_eq!(visible, 0, "object behind camera should be culled");
    }
}
