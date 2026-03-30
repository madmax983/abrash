//! CPU software renderer implementing the [`Renderer`] trait.
//!
//! Bridges the render API to the existing `TileRenderer` / scanline rasterization.

use crate::mesh::Mesh;
use crate::rasterizer::TileRenderer;
use crate::render_api::draw_list::{DrawBatch, DrawList};
use crate::render_api::frame::Frame;
use crate::render_api::handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
use crate::render_api::material::Material;
use crate::render_api::renderer::{RenderError, Renderer};
use crate::render_api::target::RenderTarget;
use crate::texture::Texture;

struct CpuMesh {
    mesh: Mesh,
    shared_indices: std::sync::Arc<[[usize; 3]]>,
}

/// Software rasterizer implementing the [`Renderer`] trait.
///
/// Uses `TileRenderer` internally for cache-efficient tile-based rendering.
pub struct CpuRenderer {
    tile_renderer: TileRenderer,
    meshes: ResourcePool<CpuMesh>,
    textures: ResourcePool<Texture>,
    materials: ResourcePool<Material>,
}

/// Convert an internal pool handle to the public API handle type by copying
/// the index/generation — the phantom type is zero-sized so the layout is identical.
#[inline]
const fn to_mesh_handle(h: Handle<CpuMesh>) -> MeshHandle {
    Handle::new(h.index, h.generation)
}
#[inline]
const fn from_mesh_handle(h: MeshHandle) -> Handle<CpuMesh> {
    Handle::new(h.index, h.generation)
}
#[inline]
const fn to_texture_handle(h: Handle<Texture>) -> TextureHandle {
    Handle::new(h.index, h.generation)
}
#[inline]
const fn from_texture_handle(h: TextureHandle) -> Handle<Texture> {
    Handle::new(h.index, h.generation)
}
#[inline]
const fn to_material_handle(h: Handle<Material>) -> MaterialHandle {
    Handle::new(h.index, h.generation)
}
#[inline]
const fn from_material_handle(h: MaterialHandle) -> Handle<Material> {
    Handle::new(h.index, h.generation)
}

impl CpuRenderer {
    /// Create a new CPU renderer for the given resolution.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let mut tile_renderer = TileRenderer::new(width, height);
        tile_renderer.enable_hiz();
        Self {
            tile_renderer,
            meshes: ResourcePool::new(),
            textures: ResourcePool::new(),
            materials: ResourcePool::new(),
        }
    }

    /// Extract a [`Frame`] into a [`DrawList`] by resolving handles, transforming
    /// vertices to clip-space, and resolving material colors.
    ///
    /// # Errors
    ///
    /// Returns an error if any handle in the frame is stale. On success the
    /// returned `DrawList` is self-contained and can be executed or inspected
    /// independently of this renderer's internal pools.
    #[allow(clippy::missing_errors_doc)]
    pub fn extract_draw_list(&self, frame: &Frame) -> Result<DrawList, RenderError> {
        let view_proj = frame.camera.view * frame.camera.projection;
        let mut draw_list = DrawList::new(frame.camera);
        draw_list.clear_color = frame.clear_color;
        draw_list.lights.clone_from(&frame.lights);

        // Pre-allocate the batches vector if we know how many commands there are
        draw_list.batches.reserve(frame.commands.len());

        for cmd in &frame.commands {
            let cpu_mesh = self
                .meshes
                .get(from_mesh_handle(cmd.mesh))
                .ok_or(RenderError::StaleHandle("mesh"))?;
            let material = self
                .materials
                .get(from_material_handle(cmd.material))
                .ok_or(RenderError::StaleHandle("material"))?;

            let mvp = cmd.transform * view_proj;
            let mesh = &cpu_mesh.mesh;

            let mut vertices = Vec::with_capacity(mesh.vertices.len());
            let uninit_slice = vertices.spare_capacity_mut();
            // We know the slice length exactly matches `mesh.vertices.len()`
            let uninit_slice = &mut uninit_slice[..mesh.vertices.len()];

            #[cfg(feature = "parallel")]
            mvp.transform_points_uninit_parallel(&mesh.vertices, uninit_slice);

            #[cfg(not(feature = "parallel"))]
            mvp.transform_points_uninit(&mesh.vertices, uninit_slice);

            // SAFETY: `transform_points_uninit` initialized exactly `mesh.vertices.len()` elements.
            unsafe {
                vertices.set_len(mesh.vertices.len());
            }

            draw_list.push(DrawBatch::new(
                vertices,
                std::sync::Arc::clone(&cpu_mesh.shared_indices),
                material.color,
            ));
        }

        Ok(draw_list)
    }

    /// Execute a pre-built [`DrawList`] into the given render target.
    ///
    /// Uses tile-integrated clearing when `draw_list.clear_color` is set,
    /// eliminating the separate full-frame memset by writing clear color
    /// per-tile during `end_frame`.
    pub fn execute_draw_list(&mut self, draw_list: &DrawList, target: &mut RenderTarget) {
        self.tile_renderer.set_clear_color(draw_list.clear_color);

        self.tile_renderer.begin_frame();
        for batch in &draw_list.batches {
            self.tile_renderer
                .submit_mesh(&batch.indices, &batch.vertices, batch.color);
        }
        self.tile_renderer
            .end_frame(&mut target.framebuffer, &mut target.zbuffer);
    }
}

impl Renderer for CpuRenderer {
    fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, RenderError> {
        // Validate all triangle indices are in bounds
        for (tri_idx, indices) in mesh.indices.iter().enumerate() {
            for &idx in indices {
                if idx >= mesh.vertices.len() {
                    return Err(RenderError::InvalidMesh(format!(
                        "triangle {tri_idx} has index {idx} but mesh only has {} vertices",
                        mesh.vertices.len()
                    )));
                }
            }
        }
        // Use `as_slice()` directly to avoid a redundant `Vec` heap allocation and copy when creating an `Arc<[T]>`.
        // Expected impact: Removes 1 full mesh data heap allocation per `CpuMesh`.
        let shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
        Ok(to_mesh_handle(self.meshes.insert(CpuMesh {
            mesh: mesh.clone(),
            shared_indices,
        })))
    }

    fn create_texture(&mut self, texture: &Texture) -> Result<TextureHandle, RenderError> {
        Ok(to_texture_handle(self.textures.insert(texture.clone())))
    }

    fn create_material(&mut self, material: Material) -> Result<MaterialHandle, RenderError> {
        Ok(to_material_handle(self.materials.insert(material)))
    }

    fn render_frame(
        &mut self,
        frame: &Frame,
        target: &mut RenderTarget,
    ) -> Result<(), RenderError> {
        let draw_list = self.extract_draw_list(frame)?;
        self.execute_draw_list(&draw_list, target);
        Ok(())
    }

    fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes.remove(from_mesh_handle(handle));
    }

    fn destroy_texture(&mut self, handle: TextureHandle) {
        self.textures.remove(from_texture_handle(handle));
    }

    fn destroy_material(&mut self, handle: MaterialHandle) {
        self.materials.remove(from_material_handle(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use crate::mesh::Mesh;
    use crate::render_api::Renderer;
    use crate::render_api::frame::{Frame, FrameCamera};
    use crate::render_api::material::Material;
    use crate::render_api::target::RenderTarget;

    fn test_camera() -> FrameCamera {
        FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
        )
    }

    #[test]
    fn test_cpu_renderer_lifecycle() {
        let mut renderer = CpuRenderer::new(800, 600);
        let mut target = RenderTarget::new(800, 600).unwrap();

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF_0000))
            .unwrap();

        let mut frame = Frame::new(test_camera());
        frame.draw(mesh_h, mat_h, Mat4::identity());
        assert!(renderer.render_frame(&frame, &mut target).is_ok());

        renderer.destroy_mesh(mesh_h);
        renderer.destroy_material(mat_h);
    }

    #[test]
    fn test_cpu_renderer_stale_handle() {
        let mut renderer = CpuRenderer::new(100, 100);
        let mut target = RenderTarget::new(100, 100).unwrap();

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF_0000))
            .unwrap();

        renderer.destroy_mesh(mesh_h);

        let mut frame = Frame::new(test_camera());
        frame.draw(mesh_h, mat_h, Mat4::identity());
        assert!(renderer.render_frame(&frame, &mut target).is_err());
    }

    #[test]
    fn test_cpu_renderer_invalid_mesh() {
        let mut renderer = CpuRenderer::new(100, 100);

        let mut bad_mesh = Mesh::new();
        bad_mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        bad_mesh.indices.push([0, 1, 2]); // indices 1, 2 are OOB

        assert!(renderer.create_mesh(&bad_mesh).is_err());
    }

    #[test]
    fn test_cpu_renderer_empty_frame() {
        let mut renderer = CpuRenderer::new(100, 100);
        let mut target = RenderTarget::new(100, 100).unwrap();

        let frame = Frame::new(test_camera());
        assert!(renderer.render_frame(&frame, &mut target).is_ok());

        // All pixels should be clear color (0xFF000000 = opaque black)
        assert!(target.pixels().iter().all(|&p| p == 0xFF00_0000));
    }

    #[test]
    fn test_cpu_renderer_renders_visible_pixels() {
        let mut renderer = CpuRenderer::new(200, 200);
        let mut target = RenderTarget::new(200, 200).unwrap();

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF_0000))
            .unwrap();

        let camera = FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 0.0, 3.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 1.0, 0.1, 100.0),
        );

        let mut frame = Frame::new(camera);
        frame.draw(mesh_h, mat_h, Mat4::identity());
        renderer.render_frame(&frame, &mut target).unwrap();

        let non_black = target
            .pixels()
            .iter()
            .filter(|&&p| p != 0xFF00_0000)
            .count();
        assert!(
            non_black > 100,
            "Expected visible cube pixels, got {non_black}"
        );
    }

    #[test]
    fn test_extract_draw_list_batch_count() {
        let mut renderer = CpuRenderer::new(200, 200);
        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF_0000))
            .unwrap();

        let mut frame = Frame::new(test_camera());
        frame.draw(mesh_h, mat_h, Mat4::identity());
        frame.draw(mesh_h, mat_h, Mat4::translation(3.0, 0.0, 0.0));

        let dl = renderer.extract_draw_list(&frame).unwrap();
        assert_eq!(dl.batches.len(), 2, "One batch per draw command");
        assert!(dl.triangle_count() > 0);
    }

    #[test]
    fn test_extract_draw_list_stale_handle_error() {
        let mut renderer = CpuRenderer::new(100, 100);
        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF_0000))
            .unwrap();

        renderer.destroy_mesh(mesh_h);

        let mut frame = Frame::new(test_camera());
        frame.draw(mesh_h, mat_h, Mat4::identity());

        assert!(renderer.extract_draw_list(&frame).is_err());
    }

    #[test]
    fn test_execute_draw_list_matches_render_frame() {
        // Both paths (render_frame vs extract+execute) must produce identical pixels.
        let mut r1 = CpuRenderer::new(100, 100);
        let mut r2 = CpuRenderer::new(100, 100);
        let mut t1 = RenderTarget::new(100, 100).unwrap();
        let mut t2 = RenderTarget::new(100, 100).unwrap();

        let setup = |r: &mut CpuRenderer| -> Frame {
            let mesh_h = r.create_mesh(&Mesh::cube(1.0)).unwrap();
            let mat_h = r.create_material(Material::flat(0xFFAA_BBCC)).unwrap();
            let mut frame = Frame::new(FrameCamera::new(
                Mat4::look_at(
                    Vec3::new(0.0, 0.0, 3.0),
                    Vec3::ZERO,
                    Vec3::new(0.0, 1.0, 0.0),
                ),
                Mat4::perspective(1.57, 1.0, 0.1, 100.0),
            ));
            frame.draw(mesh_h, mat_h, Mat4::identity());
            frame
        };

        let frame1 = setup(&mut r1);
        let frame2 = setup(&mut r2);

        // Path A: render_frame (goes through extract+execute internally)
        r1.render_frame(&frame1, &mut t1).unwrap();

        // Path B: extract then execute explicitly
        let dl = r2.extract_draw_list(&frame2).unwrap();
        r2.execute_draw_list(&dl, &mut t2);

        let mismatches = t1
            .pixels()
            .iter()
            .zip(t2.pixels().iter())
            .filter(|&(&a, &b)| a != b)
            .count();
        assert_eq!(
            mismatches, 0,
            "render_frame and extract+execute must be pixel-identical"
        );
    }
}
