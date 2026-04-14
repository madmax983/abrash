//! CPU software renderer.
//!
//! Bridges the render API to the existing `TileRenderer` / scanline rasterization.

use crate::mesh::Mesh;
use crate::rasterizer::TileRenderer;
use crate::render_api::borrowed_target::BorrowedRenderTarget;
use crate::render_api::draw_list::{DrawBatch, DrawList};
use crate::render_api::frame::Frame;
use crate::render_api::handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
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

struct CpuMesh {
    mesh: Mesh,
    shared_indices: std::sync::Arc<[[usize; 3]]>,
}

/// Software rasterizer.
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
            // ⚡ Bolt: Pre-allocate standard scene capacities to prevent initial heap resizing
            meshes: ResourcePool::with_capacity(128),
            textures: ResourcePool::with_capacity(64),
            materials: ResourcePool::with_capacity(128),
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
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub fn extract_draw_list(&self, frame: &Frame) -> Result<DrawList, RenderError> {
        let view_proj = frame.camera.view * frame.camera.projection;

        // Pre-calculate total required vertices to avoid dynamic reallocations
        let mut total_vertices = 0;
        for cmd in &frame.commands {
            let cpu_mesh = self
                .meshes
                .get(from_mesh_handle(cmd.mesh))
                .ok_or(RenderError::StaleHandle("mesh"))?;

            // Validate material handles sequentially to eliminate intermediate Result vectors later
            if self
                .materials
                .get(from_material_handle(cmd.material))
                .is_none()
            {
                return Err(RenderError::StaleHandle("material"));
            }

            total_vertices += cpu_mesh.mesh.vertices.len();
        }

        let mut draw_list =
            DrawList::with_capacity(frame.camera, frame.commands.len(), total_vertices, frame.lights.len());
        draw_list.clear_color = frame.clear_color;
        draw_list.lights.clone_from(&frame.lights);

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            // Calculate vertex ranges first to know where each mesh writes
            let mut ranges = Vec::with_capacity(frame.commands.len());
            let mut current_offset = 0;
            for cmd in &frame.commands {
                // Since we already checked handles above, unwraps here are safe
                let cpu_mesh = self.meshes.get(from_mesh_handle(cmd.mesh)).unwrap();
                let len = cpu_mesh.mesh.vertices.len();
                ranges.push((current_offset, current_offset + len));
                current_offset += len;
            }

            // Safety: We ensure `ranges` accurately bounds writes to disjoint sections
            // of the pre-allocated buffer exactly `total_vertices` in length.
            let ptr = draw_list.vertices.as_mut_ptr() as usize; // Cast to usize to make it Send + Sync

            // Use par_extend to directly populate batches without an intermediate Vec allocation.
            draw_list
                .batches
                .par_extend(
                    frame
                        .commands
                        .par_iter()
                        .zip(&ranges)
                        .map(|(cmd, &(start, end))| {
                            let cpu_mesh = self.meshes.get(from_mesh_handle(cmd.mesh)).unwrap();
                            let material = self
                                .materials
                                .get(from_material_handle(cmd.material))
                                .unwrap();

                            let mvp = cmd.transform * view_proj;
                            let mesh = &cpu_mesh.mesh;

                            // SAFETY: `ranges` ensures disjoint segments of the allocated buffer.
                            // The buffer is pre-allocated with `total_vertices` capacity.
                            unsafe {
                                let offset_ptr = (ptr as *mut (crate::math::Vec3, f32)).add(start);
                                // We cast `offset_ptr` to `*mut std::mem::MaybeUninit` to pass into `transform_points_uninit`.
                                let slice = std::slice::from_raw_parts_mut(
                                    offset_ptr
                                        .cast::<std::mem::MaybeUninit<(crate::math::Vec3, f32)>>(),
                                    mesh.vertices.len(),
                                );
                                mvp.transform_points_uninit(&mesh.vertices, slice);
                            }

                            DrawBatch::new(
                                start..end,
                                std::sync::Arc::clone(&cpu_mesh.shared_indices),
                                material.color,
                            )
                        }),
                );

            // SAFETY: All parallel segments initialized elements exactly up to `total_vertices`.
            unsafe {
                draw_list.vertices.set_len(total_vertices);
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
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

                let start_idx = draw_list.vertices.len();
                let end_idx = start_idx + mesh.vertices.len();

                let uninit_slice = draw_list.vertices.spare_capacity_mut();
                let uninit_slice = &mut uninit_slice[..mesh.vertices.len()];

                mvp.transform_points_uninit(&mesh.vertices, uninit_slice);

                unsafe {
                    draw_list.vertices.set_len(end_idx);
                }

                draw_list.push(DrawBatch::new(
                    start_idx..end_idx,
                    std::sync::Arc::clone(&cpu_mesh.shared_indices),
                    material.color,
                ));
            }
        }

        Ok(draw_list)
    }

    fn execute_draw_list_owned(&mut self, draw_list: &DrawList, target: &mut RenderTarget) {
        if let Some(color) = draw_list.clear_color {
            target.framebuffer.clear(color);
            target.zbuffer.clear();
        }

        self.tile_renderer.set_clear_color(None); // Disable integrated clearing since we just did a full clear

        self.tile_renderer.begin_frame();
        for batch in &draw_list.batches {
            self.tile_renderer.submit_mesh(
                &batch.indices,
                &draw_list.vertices[batch.vertex_range.start..batch.vertex_range.end],
                batch.color,
            );
        }
        self.tile_renderer
            .end_frame(&mut target.framebuffer, &mut target.zbuffer);
    }

    /// Execute a pre-built [`DrawList`] into caller-owned buffers.
    pub fn execute_draw_list_into(
        &mut self,
        draw_list: &DrawList,
        target: &mut BorrowedRenderTarget<'_>,
    ) {
        let width = target.width();
        let height = target.height();
        let (pixels, depths) = target.split_mut();

        if let Some(color) = draw_list.clear_color {
            pixels.fill(color);
            depths.fill(f32::INFINITY);
        }

        self.tile_renderer.set_clear_color(None); // Disable integrated clearing since we just did a full clear

        self.tile_renderer.begin_frame();
        for batch in &draw_list.batches {
            self.tile_renderer.submit_mesh(
                &batch.indices,
                &draw_list.vertices[batch.vertex_range.start..batch.vertex_range.end],
                batch.color,
            );
        }
        self.tile_renderer
            .end_frame_into_slices(width, height, pixels, depths);
    }

    /// Execute a pre-built [`DrawList`] into the given render target.
    ///
    /// Uses tile-integrated clearing when `draw_list.clear_color` is set,
    /// eliminating the separate full-frame memset by writing clear color
    /// per-tile during `end_frame`.
    pub fn execute_draw_list(&mut self, draw_list: &DrawList, target: &mut RenderTarget) {
        self.execute_draw_list_owned(draw_list, target);
    }

    /// Upload a mesh and return a handle.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidMesh`] if the mesh data is malformed.
    pub fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, RenderError> {
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
        let shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
        Ok(to_mesh_handle(self.meshes.insert(CpuMesh {
            mesh: mesh.clone(),
            shared_indices,
        })))
    }

    /// Upload a mesh taking ownership of the data, preventing a `clone()`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidMesh`] if triangle indices point out of bounds.
    pub fn create_mesh_owned(&mut self, mesh: Mesh) -> Result<MeshHandle, RenderError> {
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
        let shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
        Ok(to_mesh_handle(self.meshes.insert(CpuMesh {
            mesh,
            shared_indices,
        })))
    }

    /// Update an existing mesh resource with new data.
    ///
    /// This is used for per-frame updates like vertex skinning.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::StaleHandle`] if the handle is invalid.
    /// Returns [`RenderError::InvalidMesh`] if the mesh data is malformed.
    pub fn update_mesh(&mut self, handle: MeshHandle, mesh: &Mesh) -> Result<(), RenderError> {
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

        let cpu_mesh = self
            .meshes
            .get_mut(from_mesh_handle(handle))
            .ok_or(RenderError::StaleHandle("mesh"))?;

        // ⚡ Bolt: Only reallocate the Arc block if the actual topology (indices) has changed.
        // This eliminates costly atomic reference allocations during vertex-only updates (e.g., skeletal animations).
        if cpu_mesh.mesh.indices != mesh.indices {
            cpu_mesh.shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
        }
        // ⚡ Bolt: Use `clone_from` instead of `clone()` to reuse the destination Mesh's pre-allocated
        // Vec capacities, entirely eliminating O(N) heap deallocations and re-allocations per update.
        cpu_mesh.mesh.clone_from(mesh);
        Ok(())
    }

    /// Upload a texture and return a handle.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidTexture`] if the texture data is malformed.
    pub fn create_texture(&mut self, texture: &Texture) -> Result<TextureHandle, RenderError> {
        Ok(to_texture_handle(self.textures.insert(texture.clone())))
    }

    /// Upload a texture taking ownership of the data, preventing a `clone()`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidTexture`] if the texture data is malformed.
    pub fn create_texture_owned(&mut self, texture: Texture) -> Result<TextureHandle, RenderError> {
        Ok(to_texture_handle(self.textures.insert(texture)))
    }

    /// Register a material and return a handle.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Internal`] if the material cannot be registered.
    pub fn create_material(&mut self, material: Material) -> Result<MaterialHandle, RenderError> {
        Ok(to_material_handle(self.materials.insert(material)))
    }

    /// Render a frame into caller-owned buffers.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::StaleHandle`] if any handle in the frame is invalid,
    /// or [`RenderError::Internal`] for backend-specific failures.
    pub fn render_frame_into(
        &mut self,
        frame: &Frame,
        target: &mut BorrowedRenderTarget<'_>,
    ) -> Result<(), RenderError> {
        let draw_list = self.extract_draw_list(frame)?;
        self.execute_draw_list_into(&draw_list, target);
        Ok(())
    }

    /// Render a frame into the render target.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::StaleHandle`] if any handle in the frame is invalid,
    /// or [`RenderError::Internal`] for backend-specific failures.
    pub fn render_frame(
        &mut self,
        frame: &Frame,
        target: &mut RenderTarget,
    ) -> Result<(), RenderError> {
        let draw_list = self.extract_draw_list(frame)?;
        self.execute_draw_list_owned(&draw_list, target);
        Ok(())
    }

    /// Release a mesh resource.
    pub fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes.remove(from_mesh_handle(handle));
    }

    /// Release a texture resource.
    pub fn destroy_texture(&mut self, handle: TextureHandle) {
        self.textures.remove(from_texture_handle(handle));
    }

    /// Release a material resource.
    pub fn destroy_material(&mut self, handle: MaterialHandle) {
        self.materials.remove(from_material_handle(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use crate::mesh::Mesh;
    use crate::render_api::borrowed_target::BorrowedRenderTarget;
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
    fn test_update_mesh_then_render() {
        let mut renderer = CpuRenderer::new(200, 200);
        let mut target = RenderTarget::new(200, 200).unwrap();

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFFF_0000))
            .unwrap();

        // Update the mesh with new geometry (a smaller cube)
        let updated = Mesh::cube(0.5);
        assert!(renderer.update_mesh(mesh_h, &updated).is_ok());

        // Render should succeed after update
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
        assert!(renderer.render_frame(&frame, &mut target).is_ok());
    }

    #[test]
    fn test_update_mesh_stale_handle() {
        let mut renderer = CpuRenderer::new(100, 100);

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        renderer.destroy_mesh(mesh_h);

        let updated = Mesh::cube(0.5);
        let result = renderer.update_mesh(mesh_h, &updated);
        assert!(result.is_err());
        match result.unwrap_err() {
            RenderError::StaleHandle(kind) => assert_eq!(kind, "mesh"),
            other => panic!("Expected StaleHandle, got {other}"),
        }
    }

    #[test]
    fn test_update_mesh_invalid_indices() {
        let mut renderer = CpuRenderer::new(100, 100);

        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();

        let mut bad_mesh = Mesh::new();
        bad_mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        bad_mesh.indices.push([0, 1, 2]); // indices 1, 2 are OOB

        let result = renderer.update_mesh(mesh_h, &bad_mesh);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_texture() {
        let mut renderer = CpuRenderer::new(100, 100);
        let tex = Texture::new(2, 2).unwrap();

        let handle1 = renderer.create_texture(&tex);
        assert!(handle1.is_ok());

        let handle2 = renderer.create_texture_owned(tex);
        assert!(handle2.is_ok());
    }

    #[test]
    fn test_destroy_texture() {
        let mut renderer = CpuRenderer::new(100, 100);
        let tex = Texture::new(2, 2).unwrap();

        let handle = renderer.create_texture(&tex).unwrap();

        // Destroy the texture
        renderer.destroy_texture(handle);

        // CpuRenderer doesn't currently expose a way to get a texture or use a texture in a DrawCommand (in this API at least)
        // But we can check that it's no longer in the resource pool directly using internal state
        // To be safe and just test the destruction, we can check `renderer.textures.get(from_texture_handle(handle))` is None.

        let pool_handle = super::from_texture_handle(handle);
        assert!(renderer.textures.get(pool_handle).is_none());
    }

    fn setup_renderer_and_frame(size: u32) -> (CpuRenderer, Frame) {
        let mut renderer = CpuRenderer::new(size, size);
        let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
        let mat_h = renderer
            .create_material(Material::flat(0xFFAA_BBCC))
            .unwrap();
        let mut frame = Frame::new(FrameCamera::new(
            Mat4::look_at(
                Vec3::new(0.0, 0.0, 3.0),
                Vec3::ZERO,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            Mat4::perspective(1.57, 1.0, 0.1, 100.0),
        ));
        frame.draw(mesh_h, mat_h, Mat4::identity());
        (renderer, frame)
    }

    #[test]
    fn test_execute_draw_list_into_borrowed_target_matches_owned_target() {
        let (mut source_renderer, frame) = setup_renderer_and_frame(100);
        let draw_list = source_renderer.extract_draw_list(&frame).unwrap();

        let mut owned_renderer = CpuRenderer::new(100, 100);
        let mut owned_target = RenderTarget::new(100, 100).unwrap();
        owned_renderer.execute_draw_list(&draw_list, &mut owned_target);

        let mut borrowed_renderer = CpuRenderer::new(100, 100);
        let mut pixels = vec![0xDEAD_BEEF; 100 * 100];
        let mut depths = vec![123.0; 100 * 100];
        let mut borrowed =
            BorrowedRenderTarget::new(100, 100, pixels.as_mut_slice(), depths.as_mut_slice())
                .unwrap();
        borrowed_renderer.execute_draw_list_into(&draw_list, &mut borrowed);

        assert_eq!(owned_target.pixels(), borrowed.pixels());
        assert_eq!(owned_target.depths(), borrowed.depths());
    }

    #[test]
    fn test_render_frame_into_borrowed_target_matches_owned_target() {
        let (mut owned_renderer, owned_frame) = setup_renderer_and_frame(100);
        let mut owned_target = RenderTarget::new(100, 100).unwrap();
        owned_renderer
            .render_frame(&owned_frame, &mut owned_target)
            .unwrap();

        let (mut borrowed_renderer, borrowed_frame) = setup_renderer_and_frame(100);
        let mut pixels = vec![0xDEAD_BEEF; 100 * 100];
        let mut depths = vec![123.0; 100 * 100];
        let mut borrowed =
            BorrowedRenderTarget::new(100, 100, pixels.as_mut_slice(), depths.as_mut_slice())
                .unwrap();
        borrowed_renderer
            .render_frame_into(&borrowed_frame, &mut borrowed)
            .unwrap();

        assert_eq!(owned_target.pixels(), borrowed.pixels());
        assert_eq!(owned_target.depths(), borrowed.depths());
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
