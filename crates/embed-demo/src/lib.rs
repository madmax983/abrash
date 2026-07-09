#![allow(clippy::verbose_bit_mask)]
#![allow(clippy::unnecessary_struct_initialization)]
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
//! let mut pixels = vec![0xFF00_0000; 320 * 240];
//! let mut depths = vec![f32::INFINITY; 320 * 240];
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
//! backend
//!     .render_into(&scene, pixels.as_mut_slice(), depths.as_mut_slice())
//!     .expect("render_into");
//! let non_bg = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
//! assert!(non_bg > 0, "cube should produce visible pixels");
//! ```

#![allow(dead_code)]

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::cpu_renderer::CpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle};
use abrash_render::render_api::material::Material;
use abrash_render::render_api::target::RenderTarget;
use abrash_render::render_api::{BorrowedRenderTarget, RenderError};

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
/// The primary embed path is [`render_into`](Self::render_into), which renders
/// directly into caller-owned pixel and depth buffers. [`render`](Self::render)
/// remains as a convenience wrapper backed by an internal [`RenderTarget`].
///
/// This struct has **no platform dependency** — it will not link `windows-sys`,
/// `crossterm`, `ratatui`, or `ratzilla`.
///
/// # Lifecycle
///
/// 1. `AbrashBackend::new(width, height)` — create the backend
/// 2. `register_mesh(&mesh)` — upload geometry, receive an index
/// 3. Loop: build an [`EmbedScene`] and call [`render_into`](Self::render_into)
///    with the host-owned pixel/depth buffers
/// 4. Optionally use [`render`](Self::render) / [`pixels`](Self::pixels) as a
///    convenience path for examples and tests
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

    fn build_frame(&mut self, scene: &EmbedScene<'_>) -> (Frame, Vec<MaterialHandle>) {
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(scene.camera.position, scene.camera.target, up);
        let proj = Mat4::perspective(scene.camera.fov_y, self.aspect, 0.1, 1000.0);
        let camera = FrameCamera::new(view, proj);

        let mut frame = Frame::with_capacity(camera, scene.draws.len(), 0);
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

        (frame, mat_handles)
    }

    fn destroy_materials(&mut self, mat_handles: Vec<MaterialHandle>) {
        for mat_handle in mat_handles {
            self.renderer.destroy_material(mat_handle);
        }
    }

    /// Render a scene into caller-owned pixel and depth buffers.
    ///
    /// This is the primary embed API. The caller keeps ownership of the buffers
    /// and can route them to Bevy, Doom, image export, or any other host layer.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Internal`] if the provided buffers do not match the
    /// backend dimensions, or any renderer error returned by
    /// [`CpuRenderer::render_frame_into`].
    ///
    /// # Panics
    ///
    /// Panics if a `draw.mesh_index` is out of bounds for registered meshes.
    pub fn render_into(
        &mut self,
        scene: &EmbedScene<'_>,
        pixels: &mut [u32],
        depths: &mut [f32],
    ) -> Result<(), RenderError> {
        let mut target = BorrowedRenderTarget::new(self.width(), self.height(), pixels, depths)
            .map_err(|msg| RenderError::Internal(msg.to_string()))?;
        let (frame, mat_handles) = self.build_frame(scene);
        let result = self.renderer.render_frame_into(&frame, &mut target);
        self.destroy_materials(mat_handles);
        result
    }

    /// Render a scene into the backend-owned convenience target and return the pixels.
    ///
    /// Returns a slice of `width × height` pixels in 0xAARRGGBB format.
    /// The slice is valid until the next call to `render`.
    ///
    /// # Panics
    ///
    /// Panics if a `draw.mesh_index` is out of bounds for registered meshes, or
    /// if the internal convenience target fails to render.
    pub fn render(&mut self, scene: &EmbedScene<'_>) -> &[u32] {
        let (frame, mat_handles) = self.build_frame(scene);
        let result = self.renderer.render_frame(&frame, &mut self.target);
        self.destroy_materials(mat_handles);
        result.expect("render_frame");

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
    fn test_render_into_external_buffers_produces_visible_pixels() {
        let mut backend = AbrashBackend::new(200, 200);
        let cube = backend.register_mesh(&Mesh::cube(1.0));
        let mut pixels = vec![0xFF00_0000; 200 * 200];
        let mut depths = vec![f32::INFINITY; 200 * 200];

        backend
            .render_into(
                &EmbedScene {
                    camera: front_camera(),
                    draws: &[EmbedDraw {
                        mesh_index: cube,
                        transform: Mat4::identity(),
                        color: 0xFFFF_4444,
                    }],
                },
                pixels.as_mut_slice(),
                depths.as_mut_slice(),
            )
            .expect("render_into");

        let visible = pixels.iter().filter(|&&p| p != 0xFF00_0000).count();
        assert!(
            visible > 100,
            "cube should produce visible pixels, got {visible}"
        );
    }

    #[test]
    fn test_render_into_external_buffers_preserves_host_ownership() {
        let mut backend = AbrashBackend::new(128, 128);
        let cube = backend.register_mesh(&Mesh::cube(1.0));
        let mut pixels = vec![0xFF00_0000; 128 * 128];
        let mut depths = vec![f32::INFINITY; 128 * 128];
        let pixels_ptr = pixels.as_ptr();
        let depths_ptr = depths.as_ptr();

        let empty_scene = EmbedScene {
            camera: front_camera(),
            draws: &[],
        };
        let internal_pixels = backend.render(&empty_scene).to_vec();

        backend
            .render_into(
                &EmbedScene {
                    camera: front_camera(),
                    draws: &[EmbedDraw {
                        mesh_index: cube,
                        transform: Mat4::identity(),
                        color: 0xFFFF_4444,
                    }],
                },
                pixels.as_mut_slice(),
                depths.as_mut_slice(),
            )
            .expect("render_into");

        assert_eq!(pixels.as_ptr(), pixels_ptr, "pixel buffer identity changed");
        assert_eq!(depths.as_ptr(), depths_ptr, "depth buffer identity changed");
        assert_eq!(
            backend.pixels(),
            internal_pixels.as_slice(),
            "render_into should not overwrite the backend-owned convenience target"
        );

        pixels[0] = 0x1234_5678;
        depths[0] = 42.0;
        assert_eq!(pixels[0], 0x1234_5678);
        assert!((depths[0] - 42.0).abs() < 1e-5);
    }

    #[test]
    fn test_render_into_external_buffers_matches_render_convenience_path() {
        let mut convenience_backend = AbrashBackend::new(160, 120);
        let convenience_cube = convenience_backend.register_mesh(&Mesh::cube(1.0));
        let scene = EmbedScene {
            camera: front_camera(),
            draws: &[EmbedDraw {
                mesh_index: convenience_cube,
                transform: Mat4::rotation_y(std::f32::consts::FRAC_PI_6),
                color: 0xFFAA_BBCC,
            }],
        };
        let convenience_pixels = convenience_backend.render(&scene).to_vec();
        let convenience_depths = convenience_backend.target().depths().to_vec();

        let mut external_backend = AbrashBackend::new(160, 120);
        let external_cube = external_backend.register_mesh(&Mesh::cube(1.0));
        let mut pixels = vec![0xFF00_0000; 160 * 120];
        let mut depths = vec![f32::INFINITY; 160 * 120];
        external_backend
            .render_into(
                &EmbedScene {
                    camera: front_camera(),
                    draws: &[EmbedDraw {
                        mesh_index: external_cube,
                        transform: Mat4::rotation_y(std::f32::consts::FRAC_PI_6),
                        color: 0xFFAA_BBCC,
                    }],
                },
                pixels.as_mut_slice(),
                depths.as_mut_slice(),
            )
            .expect("render_into");

        assert_eq!(pixels, convenience_pixels);
        assert_eq!(depths, convenience_depths);
    }

    #[test]
    fn test_render_into_external_buffers_rejects_invalid_slices_before_rendering() {
        let mut backend = AbrashBackend::new(64, 64);
        let cube = backend.register_mesh(&Mesh::cube(1.0));
        let mut pixels = vec![0xDEAD_BEEF; 8];
        let mut depths = vec![123.0; 8];
        let clear_scene = EmbedScene {
            camera: front_camera(),
            draws: &[],
        };
        let baseline = backend.render(&clear_scene).to_vec();

        let err = backend.render_into(
            &EmbedScene {
                camera: front_camera(),
                draws: &[EmbedDraw {
                    mesh_index: cube,
                    transform: Mat4::identity(),
                    color: 0xFFFF_4444,
                }],
            },
            pixels.as_mut_slice(),
            depths.as_mut_slice(),
        );
        assert!(err.is_err(), "short slices should be rejected");
        assert_eq!(
            backend.pixels(),
            baseline.as_slice(),
            "failed render_into should not mutate the convenience target"
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
                camera: camera.clone(),
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

    #[test]
    #[should_panic(expected = "mesh indices must be in bounds")]
    fn test_register_mesh_panics_on_invalid_indices() {
        let mut backend = AbrashBackend::new(100, 100);
        let mut bad_mesh = Mesh::cube(1.0);
        bad_mesh.indices.push([999, 999, 999]); // Add out-of-bounds indices
        backend.register_mesh(&bad_mesh);
    }
}
