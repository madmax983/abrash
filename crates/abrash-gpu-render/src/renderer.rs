//! `GpuRenderer` - deferred rendering backend.
//!
//! All geometry passes through the G-Buffer. Lighting is computed in a single
//! fullscreen deferred pass. No forward pipeline — deferred is the only path.

use crate::capture::{BatchStats, FrameStats, GpuCaptureTarget, GpuDebugCapture};
use crate::deferred::DeferredLightingPass;
use crate::device::{GpuDevice, GpuDeviceConfig};
use crate::gbuffer::{GBuffer, GBufferGeometryPipeline, GBufferTexturedPipeline};
use crate::mesh_buffer::GpuMeshBuffer;
use crate::shader::{DrawUniforms, FrameUniforms, GpuLightData, MAX_LIGHTS};
use abrash_core::mesh::Mesh;
use abrash_core::texture::Texture;
use abrash_render::render_api::frame::{Frame, Light};
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle, TextureHandle};
use abrash_render::render_api::material::{Material, ShadingMode};
use bytemuck::Zeroable;
use std::num::NonZeroU64;
#[cfg(feature = "windowed")]
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
#[cfg(feature = "windowed")]
use winit::window::Window;

struct GpuMaterial {
    color: u32,
    shininess: f32,
    specular_strength: f32,
    metallic: f32,
    roughness: f32,
    texture: Option<u32>,
}

/// A texture uploaded to the GPU with its view and bind group.
struct GpuTexture {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

struct PreparedDraw {
    command_index: usize,
    mesh_index: usize,
    uniform_offset: u32,
    triangle_count: u32,
    color: u32,
    texture_index: Option<u32>,
}

/// Convert `0xAARRGGBB` into a wgpu clear color.
#[must_use]
pub fn argb_to_wgpu_color(argb: u32) -> wgpu::Color {
    let a = f64::from((argb >> 24) & 0xFF) / 255.0;
    let r = f64::from((argb >> 16) & 0xFF) / 255.0;
    let g = f64::from((argb >> 8) & 0xFF) / 255.0;
    let b = f64::from(argb & 0xFF) / 255.0;

    wgpu::Color { r, g, b, a }
}

#[must_use]
const fn align_to(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

#[must_use]
const fn argb_to_rgba_bytes(argb: u32) -> [u8; 4] {
    [
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
        ((argb >> 24) & 0xFF) as u8,
    ]
}

/// GPU deferred rendering backend.
///
/// Pipeline: Shadow depth → G-Buffer geometry → Deferred lighting → Tone mapping.
pub struct GpuRenderer {
    gpu: GpuDevice,
    // G-Buffer geometry pipelines
    gbuffer_pipeline: GBufferGeometryPipeline,
    gbuffer_textured_pipeline: GBufferTexturedPipeline,
    // Deferred lighting
    deferred_pass: DeferredLightingPass,
    // Per-frame uniforms (shared by geometry + lighting passes)
    frame_uniform_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    // Per-draw uniforms (dynamic offset)
    draw_uniform_buffer: wgpu::Buffer,
    draw_uniform_stride: u64,
    draw_uniform_capacity: usize,
    // Texture support
    texture_bind_group_layout: wgpu::BindGroupLayout,
    default_sampler: wgpu::Sampler,
    point_sampler: wgpu::Sampler,
    textures: Vec<Option<GpuTexture>>,
    // Shadow mapping
    shadow_map: crate::shadow::ShadowMap,
    // Environment / skybox / IBL
    skybox_pass: crate::environment::SkyboxPass,
    skybox_cubemap: Option<crate::environment::GpuCubemap>,
    skybox_bind_group: Option<wgpu::BindGroup>,
    ibl_textures: Option<crate::ibl::IblTextures>,
    ibl_bind_group: Option<wgpu::BindGroup>,
    // Render targets (lazily created/resized)
    gbuffer: Option<GBuffer>,
    hdr_target: Option<crate::postprocess::HdrTarget>,
    tone_map_pass: crate::postprocess::ToneMapPass,
    // Resources
    meshes: Vec<Option<GpuMeshBuffer>>,
    materials: Vec<Option<GpuMaterial>>,
}

impl GpuRenderer {
    /// Construct a renderer from an already-created GPU device.
    #[must_use]
    pub fn from_gpu(gpu: GpuDevice, color_format: wgpu::TextureFormat) -> Self {
        let device = gpu.device();
        let min_align = u64::from(device.limits().min_uniform_buffer_offset_alignment).max(1);
        let hdr_format = wgpu::TextureFormat::Rgba16Float;

        // Shadow map
        let shadow_map = crate::shadow::ShadowMap::new(device);

        // G-Buffer geometry pipelines
        let gbuffer_pipeline = GBufferGeometryPipeline::new(device);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let gbuffer_textured_pipeline = GBufferTexturedPipeline::new(
            device,
            &gbuffer_pipeline.frame_bind_group_layout,
            &gbuffer_pipeline.draw_bind_group_layout,
            &texture_bind_group_layout,
        );

        // Deferred lighting pass (outputs to HDR)
        let deferred_pass =
            DeferredLightingPass::new(device, hdr_format, &shadow_map.sample_bind_group_layout);

        // Per-frame buffers
        let frame_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Uniforms"),
            contents: &[0u8; std::mem::size_of::<FrameUniforms>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: &[0u8; std::mem::size_of::<GpuLightData>() * MAX_LIGHTS],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Per-draw uniforms
        let draw_uniform_stride = align_to(std::mem::size_of::<DrawUniforms>() as u64, min_align);
        let draw_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Draw Uniform Buffer"),
            contents: &vec![0u8; draw_uniform_stride as usize],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Default Bilinear Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let point_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Point Sampler (GBuffer)"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Tone mapping (HDR → LDR)
        let tone_map_pass = crate::postprocess::ToneMapPass::new(device, color_format);

        // Skybox
        let skybox_pass = crate::environment::SkyboxPass::new(device, hdr_format);

        // Default IBL (black environment — replaced when set_environment is called)
        let default_ibl = crate::ibl::IblTextures::default_black(device);
        let default_ibl_bg =
            deferred_pass.create_ibl_bind_group(device, &default_ibl, &default_sampler);

        Self {
            gpu,
            gbuffer_pipeline,
            gbuffer_textured_pipeline,
            deferred_pass,
            frame_uniform_buffer,
            light_buffer,
            draw_uniform_buffer,
            draw_uniform_stride,
            draw_uniform_capacity: 1,
            texture_bind_group_layout,
            default_sampler,
            point_sampler,
            textures: Vec::new(),
            shadow_map,
            skybox_pass,
            skybox_cubemap: None,
            skybox_bind_group: None,
            ibl_textures: Some(default_ibl),
            ibl_bind_group: Some(default_ibl_bg),
            gbuffer: None,
            hdr_target: None,
            tone_map_pass,
            meshes: Vec::new(),
            materials: Vec::new(),
        }
    }

    /// Create a headless renderer suitable for capture.
    ///
    /// # Errors
    ///
    /// Returns an error if no GPU adapter/device can be created.
    pub fn new_headless() -> Result<Self, String> {
        let gpu = GpuDevice::new_headless(&GpuDeviceConfig::headless())?;
        Ok(Self::from_gpu(gpu, wgpu::TextureFormat::Rgba8Unorm))
    }

    /// Create a renderer and configured presentation surface for a window.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface, adapter, or device cannot be created.
    #[cfg(feature = "windowed")]
    pub fn new_windowed(window: Arc<Window>) -> Result<(Self, crate::surface::GpuSurface), String> {
        let (gpu, surface) = GpuDevice::new_windowed(window, &GpuDeviceConfig::default())?;
        let renderer = Self::from_gpu(gpu, surface.format());
        Ok((renderer, surface))
    }

    /// Access the underlying device.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        self.gpu.device()
    }

    /// Upload a mesh and return a typed handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh data is invalid.
    pub fn create_mesh(&mut self, mesh: &Mesh) -> Result<MeshHandle, String> {
        let gpu_mesh = GpuMeshBuffer::from_mesh(self.gpu.device(), mesh)?;
        let index = u32::try_from(self.meshes.len())
            .map_err(|_| "mesh pool index exceeds u32".to_string())?;
        self.meshes.push(Some(gpu_mesh));
        Ok(MeshHandle::from_raw_parts(index, 0))
    }

    /// Upload a mesh with UV coordinates to GPU buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh data is invalid.
    pub fn create_mesh_textured(&mut self, mesh: &Mesh) -> Result<MeshHandle, String> {
        let gpu_mesh = GpuMeshBuffer::from_mesh_textured(self.gpu.device(), mesh)?;
        let index = u32::try_from(self.meshes.len())
            .map_err(|_| "mesh pool index exceeds u32".to_string())?;
        self.meshes.push(Some(gpu_mesh));
        Ok(MeshHandle::from_raw_parts(index, 0))
    }

    /// Upload a texture to the GPU and return a typed handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the texture dimensions are zero.
    pub fn create_texture(&mut self, texture: &Texture) -> Result<TextureHandle, String> {
        if texture.width == 0 || texture.height == 0 {
            return Err("texture dimensions must be positive".to_string());
        }

        let mut rgba = Vec::with_capacity(texture.pixels.len() * 4);
        for &argb in &texture.pixels {
            rgba.push(((argb >> 16) & 0xFF) as u8);
            rgba.push(((argb >> 8) & 0xFF) as u8);
            rgba.push((argb & 0xFF) as u8);
            rgba.push(((argb >> 24) & 0xFF) as u8);
        }

        let device = self.gpu.device();
        let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("User Texture"),
            size: wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.gpu.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width * 4),
                rows_per_image: Some(texture.height),
            },
            wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
        );

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                },
            ],
        });

        let index = u32::try_from(self.textures.len())
            .map_err(|_| "texture pool index exceeds u32".to_string())?;
        self.textures.push(Some(GpuTexture {
            _texture: gpu_texture,
            bind_group,
        }));
        Ok(TextureHandle::from_raw_parts(index, 0))
    }

    /// Register a material and return a typed handle.
    #[must_use]
    pub fn create_material(&mut self, material: Material) -> MaterialHandle {
        let Material {
            shading,
            color,
            receive_light: _,
        } = material;
        let (color, shininess, spec, metallic, roughness, texture) = match shading {
            ShadingMode::Flat { color } => (color, 1.0, 0.0, 0.0, 1.0, None),
            ShadingMode::Phong {
                shininess,
                specular_strength,
            } => (color, shininess, specular_strength, 0.0, 0.5, None),
            ShadingMode::Textured { texture } | ShadingMode::TexturedGouraud { texture } => {
                (color, 32.0, 0.3, 0.0, 0.5, Some(texture.index()))
            }
            ShadingMode::Pbr {
                albedo,
                roughness,
                metallic,
            } => (color, 32.0, 0.5, metallic, roughness, Some(albedo.index())),
            _ => (color, 32.0, 0.3, 0.0, 0.5, None),
        };
        let index = self.materials.len() as u32;
        self.materials.push(Some(GpuMaterial {
            color,
            shininess,
            specular_strength: spec,
            metallic,
            roughness,
            texture,
        }));
        MaterialHandle::from_raw_parts(index, 0)
    }

    /// Destroy a previously uploaded mesh.
    pub fn destroy_mesh(&mut self, handle: MeshHandle) {
        if let Some(slot) = self.meshes.get_mut(handle.index() as usize) {
            *slot = None;
        }
    }

    /// Destroy a previously registered material.
    pub fn destroy_material(&mut self, handle: MaterialHandle) {
        if let Some(slot) = self.materials.get_mut(handle.index() as usize) {
            *slot = None;
        }
    }

    /// Set the environment cubemap for skybox rendering.
    ///
    /// The skybox is rendered after deferred lighting, filling background pixels
    /// where no geometry was written to the G-Buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if cubemap faces have mismatched dimensions.
    pub fn set_environment(
        &mut self,
        cubemap: &abrash_render::skybox::Cubemap,
    ) -> Result<(), String> {
        let gpu_cubemap = crate::environment::GpuCubemap::from_faces(
            self.gpu.device(),
            self.gpu.queue(),
            &cubemap.faces,
        )?;
        let skybox_bg = self
            .skybox_pass
            .create_bind_group(self.gpu.device(), &gpu_cubemap);

        // Run IBL precomputation (irradiance, prefiltered env, BRDF LUT)
        let ibl = crate::ibl::IblTextures::precompute(
            self.gpu.device(),
            self.gpu.queue(),
            &gpu_cubemap.view,
        );
        let ibl_bg = self.deferred_pass.create_ibl_bind_group(
            self.gpu.device(),
            &ibl,
            &self.default_sampler,
        );

        self.skybox_cubemap = Some(gpu_cubemap);
        self.skybox_bind_group = Some(skybox_bg);
        self.ibl_textures = Some(ibl);
        self.ibl_bind_group = Some(ibl_bg);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Render a frame into an offscreen capture target and read it back.
    ///
    /// Pipeline: Shadow → G-Buffer → Deferred Lighting → Tone Map → Readback.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame references stale handles or readback fails.
    #[allow(clippy::too_many_lines)]
    pub fn capture(
        &mut self,
        frame: &Frame,
        target: &mut GpuCaptureTarget,
    ) -> Result<GpuDebugCapture, String> {
        let start = Instant::now();
        let w = target.config.width;
        let h = target.config.height;
        let (prepared_draws, draw_uniform_bytes, total_triangles) = self.prepare_draws(frame)?;

        self.upload_uniforms(frame, &draw_uniform_bytes, prepared_draws.len());

        let background = argb_to_rgba_bytes(frame.clear_color.unwrap_or(0xFF00_0000));

        let mut encoder =
            self.gpu
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Deferred Capture Encoder"),
                });

        // Pass 1: Shadow depth
        self.encode_shadow_pass(&mut encoder, frame, &prepared_draws)?;

        // Pass 2: G-Buffer geometry
        self.ensure_gbuffer(w, h);
        self.encode_gbuffer_pass(&mut encoder, &prepared_draws)?;

        // Pass 3: Deferred lighting → HDR
        self.ensure_hdr_target(w, h);
        self.encode_deferred_lighting(&mut encoder);

        // Pass 3.5: Skybox (fills background pixels in HDR target)
        self.encode_skybox(&mut encoder, frame);

        // Pass 4: Tone mapping → LDR capture target
        self.encode_tone_map(&mut encoder, &target.color_view);

        // Readback
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &target.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(target.config.padded_bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        self.gpu.queue().submit(Some(encoder.finish()));

        let buffer_slice = target.readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.gpu.device().poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|error| format!("readback channel error: {error}"))?
            .map_err(|error| format!("readback map error: {error}"))?;

        let mapped = buffer_slice.get_mapped_range();
        let padded_stride = target.config.padded_bytes_per_row as usize;
        let unpadded_stride = target.config.unpadded_bytes_per_row as usize;
        let height = h as usize;
        let total_bytes = unpadded_stride * height;
        let mut pixels_rgba = vec![0u8; total_bytes];
        if padded_stride == unpadded_stride {
            pixels_rgba.copy_from_slice(&mapped[..total_bytes]);
        } else {
            for row in 0..height {
                let src = row * padded_stride;
                let dst = row * unpadded_stride;
                pixels_rgba[dst..dst + unpadded_stride]
                    .copy_from_slice(&mapped[src..src + unpadded_stride]);
            }
        }
        drop(mapped);
        target.readback_buffer.unmap();

        let visible_pixel_count = pixels_rgba
            .chunks_exact(4)
            .filter(|pixel| pixel != &&background[..])
            .count();

        let batches = prepared_draws
            .iter()
            .map(|draw| BatchStats {
                index: draw.command_index,
                triangle_count: draw.triangle_count,
                color: draw.color,
            })
            .collect();

        Ok(GpuDebugCapture {
            stats: FrameStats {
                width: w,
                height: h,
                batch_count: prepared_draws.len(),
                total_triangles,
                render_time: start.elapsed(),
                batches,
            },
            pixels_rgba,
            visible_pixel_count,
        })
    }

    /// Render a frame to a configured presentation surface.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface is unavailable or the frame references stale handles.
    #[cfg(feature = "windowed")]
    pub fn render_to_surface(
        &mut self,
        frame: &Frame,
        surface: &crate::surface::GpuSurface,
    ) -> Result<(), String> {
        let output = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            e => return Err(format!("surface unavailable: {e:?}")),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (w, h) = (surface.width, surface.height);
        let (prepared_draws, draw_uniform_bytes, _) = self.prepare_draws(frame)?;
        self.upload_uniforms(frame, &draw_uniform_bytes, prepared_draws.len());

        let mut encoder =
            self.gpu
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Deferred Surface Encoder"),
                });

        // Pass 1: Shadow depth
        self.encode_shadow_pass(&mut encoder, frame, &prepared_draws)?;

        // Pass 2: G-Buffer geometry
        self.ensure_gbuffer(w, h);
        self.encode_gbuffer_pass(&mut encoder, &prepared_draws)?;

        // Pass 3: Deferred lighting → HDR
        self.ensure_hdr_target(w, h);
        self.encode_deferred_lighting(&mut encoder);

        // Pass 4: Tone mapping → surface
        self.encode_tone_map(&mut encoder, &view);

        self.gpu.queue().submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal: uniform management
    // -----------------------------------------------------------------------

    fn upload_uniforms(&mut self, frame: &Frame, draw_bytes: &[u8], draw_count: usize) {
        // Frame uniforms + lights
        let (frame_uniforms, gpu_lights) = Self::prepare_frame_uniforms(frame);
        self.gpu.queue().write_buffer(
            &self.frame_uniform_buffer,
            0,
            bytemuck::bytes_of(&frame_uniforms),
        );
        self.gpu
            .queue()
            .write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&gpu_lights));

        // Per-draw uniforms
        self.ensure_draw_capacity(draw_count);
        if !draw_bytes.is_empty() {
            self.gpu
                .queue()
                .write_buffer(&self.draw_uniform_buffer, 0, draw_bytes);
        }
    }

    fn ensure_draw_capacity(&mut self, draw_count: usize) {
        let required = draw_count.max(1);
        if required <= self.draw_uniform_capacity {
            return;
        }
        let new_capacity = required.next_power_of_two();
        let contents = vec![0u8; (self.draw_uniform_stride * new_capacity as u64) as usize];
        let draw_uniform_buffer =
            self.gpu
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Draw Uniform Buffer"),
                    contents: &contents,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        self.draw_uniform_buffer = draw_uniform_buffer;
        self.draw_uniform_capacity = new_capacity;
    }

    fn ensure_gbuffer(&mut self, width: u32, height: u32) {
        let needs = self
            .gbuffer
            .as_ref()
            .is_none_or(|g| g.width != width || g.height != height);
        if needs {
            self.gbuffer = Some(GBuffer::new(self.gpu.device(), width, height));
        }
    }

    fn ensure_hdr_target(&mut self, width: u32, height: u32) {
        let needs = self
            .hdr_target
            .as_ref()
            .is_none_or(|t| t.width != width || t.height != height);
        if needs {
            self.hdr_target = Some(crate::postprocess::HdrTarget::new(
                self.gpu.device(),
                width,
                height,
            ));
        }
    }

    fn prepare_frame_uniforms(frame: &Frame) -> (FrameUniforms, Vec<GpuLightData>) {
        let view_proj = frame.camera.view * frame.camera.projection;
        let vp_flat: [f32; 16] = bytemuck::cast(view_proj.m);
        let inv_view = frame.camera.view.inverse();
        let cam_pos = [inv_view.m[3][0], inv_view.m[3][1], inv_view.m[3][2], 0.0];

        let light_count = frame.lights.len().min(MAX_LIGHTS) as u32;
        let frame_uniforms = FrameUniforms {
            view_proj: vp_flat,
            camera_pos: cam_pos,
            light_count,
            _pad: [0; 3],
        };

        let mut gpu_lights = vec![GpuLightData::zeroed(); MAX_LIGHTS];
        for (i, light) in frame.lights.iter().take(MAX_LIGHTS).enumerate() {
            gpu_lights[i] = match light {
                Light::Directional(d) => {
                    let r = ((d.color >> 16) & 0xFF) as f32 / 255.0;
                    let g = ((d.color >> 8) & 0xFF) as f32 / 255.0;
                    let b = (d.color & 0xFF) as f32 / 255.0;
                    GpuLightData {
                        position_or_direction: [d.direction.x, d.direction.y, d.direction.z],
                        light_type: 0,
                        color: [r, g, b],
                        intensity: d.intensity,
                        radius: 0.0,
                        _pad: [0.0; 3],
                    }
                }
                Light::Point(p) => {
                    let r = ((p.color >> 16) & 0xFF) as f32 / 255.0;
                    let g = ((p.color >> 8) & 0xFF) as f32 / 255.0;
                    let b = (p.color & 0xFF) as f32 / 255.0;
                    GpuLightData {
                        position_or_direction: [p.position.x, p.position.y, p.position.z],
                        light_type: 1,
                        color: [r, g, b],
                        intensity: p.intensity,
                        radius: p.radius,
                        _pad: [0.0; 3],
                    }
                }
            };
        }

        (frame_uniforms, gpu_lights)
    }

    fn prepare_draws(&self, frame: &Frame) -> Result<(Vec<PreparedDraw>, Vec<u8>, u32), String> {
        let draw_uniform_size = std::mem::size_of::<DrawUniforms>();
        let mut prepared_draws = Vec::with_capacity(frame.commands.len());
        let mut draw_bytes =
            vec![0u8; (self.draw_uniform_stride * frame.commands.len().max(1) as u64) as usize];
        let mut total_triangles = 0u32;

        for (command_index, command) in frame.commands.iter().enumerate() {
            let mesh_index = command.mesh.index() as usize;
            let gpu_mesh = self
                .meshes
                .get(mesh_index)
                .and_then(Option::as_ref)
                .ok_or_else(|| format!("stale mesh handle at command {command_index}"))?;
            let material = self
                .materials
                .get(command.material.index() as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| format!("stale material handle at command {command_index}"))?;

            let mut draw_uniform = DrawUniforms::new(
                &command.transform,
                material.color,
                material.shininess,
                material.specular_strength,
            );
            draw_uniform.metallic = material.metallic;
            draw_uniform.roughness = material.roughness;

            let offset = command_index as u64 * self.draw_uniform_stride;
            let dest = &mut draw_bytes[offset as usize..offset as usize + draw_uniform_size];
            dest.copy_from_slice(bytemuck::bytes_of(&draw_uniform));

            let uniform_offset =
                u32::try_from(offset).map_err(|_| "draw uniform offset exceeds u32".to_string())?;

            prepared_draws.push(PreparedDraw {
                command_index,
                mesh_index,
                uniform_offset,
                triangle_count: gpu_mesh.triangle_count,
                color: material.color,
                texture_index: material.texture,
            });
            total_triangles = total_triangles.saturating_add(gpu_mesh.triangle_count);
        }

        if frame.commands.is_empty() {
            draw_bytes.clear();
        }

        Ok((prepared_draws, draw_bytes, total_triangles))
    }

    // -----------------------------------------------------------------------
    // Internal: pass encoding
    // -----------------------------------------------------------------------

    fn encode_shadow_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        prepared_draws: &[PreparedDraw],
    ) -> Result<(), String> {
        use crate::shadow::ShadowMap;

        let dir_light = frame.lights.iter().find_map(|l| match l {
            Light::Directional(d) => Some(d),
            Light::Point(_) => None,
        });

        let Some(dir_light) = dir_light else {
            return Ok(());
        };

        let light_vp = ShadowMap::compute_directional_light_vp(
            abrash_core::math::Vec3::new(
                dir_light.direction.x,
                dir_light.direction.y,
                dir_light.direction.z,
            ),
            100.0,
        );
        self.shadow_map.light_vp = light_vp;

        let light_vp_flat: [f32; 16] = bytemuck::cast(light_vp.m);
        self.gpu.queue().write_buffer(
            &self.shadow_map.light_vp_buffer,
            0,
            bytemuck::cast_slice(&light_vp_flat),
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Depth Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.shadow_map.pipeline);

            for draw in prepared_draws {
                let gpu_mesh = self.meshes[draw.mesh_index].as_ref().ok_or_else(|| {
                    format!("stale mesh in shadow pass cmd {}", draw.command_index)
                })?;

                let command = &frame.commands[draw.command_index];
                let model_flat: [f32; 16] = bytemuck::cast(command.transform.m);
                let shadow_uniform = crate::shadow::ShadowUniforms {
                    light_vp: light_vp_flat,
                    model: model_flat,
                };
                self.gpu.queue().write_buffer(
                    &self.shadow_map.uniform_buffer,
                    0,
                    bytemuck::bytes_of(&shadow_uniform),
                );

                pass.set_bind_group(0, &self.shadow_map.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        Ok(())
    }

    fn encode_gbuffer_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        prepared_draws: &[PreparedDraw],
    ) -> Result<(), String> {
        let gbuffer = self.gbuffer.as_ref().unwrap();

        // Create bind groups for G-Buffer pass
        let frame_bg = self
            .gpu
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GBuffer Frame BG"),
                layout: &self.gbuffer_pipeline.frame_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.frame_uniform_buffer.as_entire_binding(),
                }],
            });

        let draw_bg = self
            .gpu
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GBuffer Draw BG"),
                layout: &self.gbuffer_pipeline.draw_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.draw_uniform_buffer,
                        offset: 0,
                        size: NonZeroU64::new(std::mem::size_of::<DrawUniforms>() as u64),
                    }),
                }],
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("G-Buffer Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.position_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.normal_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.albedo_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gbuffer.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            for draw in prepared_draws {
                let gpu_mesh = self.meshes[draw.mesh_index]
                    .as_ref()
                    .ok_or_else(|| format!("stale mesh at command {}", draw.command_index))?;

                if let Some(tex_idx) = draw.texture_index {
                    // Textured G-Buffer pipeline
                    pass.set_pipeline(&self.gbuffer_textured_pipeline.pipeline);
                    pass.set_bind_group(0, &frame_bg, &[]);
                    pass.set_bind_group(1, &draw_bg, &[draw.uniform_offset]);
                    if let Some(Some(gpu_tex)) = self.textures.get(tex_idx as usize) {
                        pass.set_bind_group(2, &gpu_tex.bind_group, &[]);
                    }
                } else {
                    // Standard G-Buffer pipeline
                    pass.set_pipeline(&self.gbuffer_pipeline.pipeline);
                    pass.set_bind_group(0, &frame_bg, &[]);
                    pass.set_bind_group(1, &draw_bg, &[draw.uniform_offset]);
                }

                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        Ok(())
    }

    fn encode_deferred_lighting(&self, encoder: &mut wgpu::CommandEncoder) {
        let gbuffer = self.gbuffer.as_ref().unwrap();
        let hdr = self.hdr_target.as_ref().unwrap();

        let gbuffer_bg = self.deferred_pass.create_gbuffer_bind_group(
            self.gpu.device(),
            gbuffer,
            &self.point_sampler,
        );
        let frame_bg = self.deferred_pass.create_frame_bind_group(
            self.gpu.device(),
            &self.frame_uniform_buffer,
            &self.light_buffer,
        );

        // IBL bind group (uses precomputed IBL if environment is set)
        let ibl_bg = self.ibl_bind_group.as_ref().unwrap();

        self.deferred_pass.encode(
            encoder,
            &gbuffer_bg,
            &frame_bg,
            &self.shadow_map.sample_bind_group,
            ibl_bg,
            &hdr.color_view,
        );
    }

    fn encode_skybox(&self, encoder: &mut wgpu::CommandEncoder, frame: &Frame) {
        let Some(ref bind_group) = self.skybox_bind_group else {
            return; // No environment map set
        };
        let hdr = self.hdr_target.as_ref().unwrap();

        // Upload inverse view-projection for direction reconstruction
        let view_proj = frame.camera.view * frame.camera.projection;
        let inv_vp = view_proj.inverse();
        let inv_vp_flat: [f32; 16] = bytemuck::cast(inv_vp.m);
        let uniform = crate::environment::SkyboxUniforms {
            inv_view_proj: inv_vp_flat,
        };
        self.gpu.queue().write_buffer(
            &self.skybox_pass.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniform),
        );

        self.skybox_pass
            .encode(encoder, bind_group, &hdr.color_view);
    }

    fn encode_tone_map(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let hdr = self.hdr_target.as_ref().unwrap();
        let tonemap_bg = self
            .tone_map_pass
            .create_bind_group(self.gpu.device(), &hdr.color_view);
        self.tone_map_pass.encode(encoder, &tonemap_bg, output_view);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argb_to_clear_color() {
        let color = argb_to_wgpu_color(0xFF80_4020);
        assert!((color.r - 0.502).abs() < 0.01);
        assert!((color.g - 0.251).abs() < 0.01);
        assert!((color.b - 0.125).abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_argb_black() {
        let color = argb_to_wgpu_color(0xFF00_0000);
        assert!(color.r.abs() < f64::EPSILON);
        assert!(color.g.abs() < f64::EPSILON);
        assert!(color.b.abs() < f64::EPSILON);
        assert!((color.a - 1.0).abs() < 0.01);
    }
}
