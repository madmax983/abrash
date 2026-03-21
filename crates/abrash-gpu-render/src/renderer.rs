//! `GpuRenderer` - the GPU rendering backend.

use crate::capture::{BatchStats, FrameStats, GpuCaptureTarget, GpuDebugCapture};
use crate::device::{GpuDevice, GpuDeviceConfig};
use crate::mesh_buffer::GpuMeshBuffer;
use crate::shader::{
    DrawUniforms, FrameUniforms, GpuLightData, LitPipeline, MAX_LIGHTS, MvpPipeline, MvpUniform,
};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::frame::{Frame, Light};
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle};
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
    lit: bool,
    shininess: f32,
    specular_strength: f32,
}

struct PreparedDraw {
    command_index: usize,
    mesh_index: usize,
    uniform_offset: u32,
    triangle_count: u32,
    color: u32,
    lit: bool,
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

/// GPU rendering backend for shared `Frame` submission.
pub struct GpuRenderer {
    gpu: GpuDevice,
    // Flat-color pipeline (existing)
    flat_pipeline: MvpPipeline,
    flat_uniform_buffer: wgpu::Buffer,
    flat_uniform_bind_group: wgpu::BindGroup,
    flat_uniform_stride: u64,
    flat_uniform_capacity: usize,
    // Lit pipeline (new)
    lit_pipeline: LitPipeline,
    frame_uniform_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    draw_uniform_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    draw_uniform_stride: u64,
    draw_uniform_capacity: usize,
    // Shared resources
    meshes: Vec<Option<GpuMeshBuffer>>,
    materials: Vec<Option<GpuMaterial>>,
}

impl GpuRenderer {
    /// Construct a renderer from an already-created GPU device.
    #[must_use]
    pub fn from_gpu(gpu: GpuDevice, color_format: wgpu::TextureFormat) -> Self {
        let device = gpu.device();
        let min_align = u64::from(device.limits().min_uniform_buffer_offset_alignment).max(1);

        // Flat pipeline (existing)
        let flat_pipeline = MvpPipeline::new(device, color_format);
        let flat_uniform_stride = align_to(std::mem::size_of::<MvpUniform>() as u64, min_align);
        let (flat_uniform_buffer, flat_uniform_bind_group) =
            Self::create_flat_uniform_resources(device, &flat_pipeline, flat_uniform_stride, 1);

        // Lit pipeline
        let lit_pipeline = LitPipeline::new(device, color_format);
        let draw_uniform_stride = align_to(std::mem::size_of::<DrawUniforms>() as u64, min_align);

        // Per-frame buffers (group 0)
        let frame_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lit Frame Uniforms"),
            contents: &[0u8; std::mem::size_of::<FrameUniforms>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lit Light Buffer"),
            contents: &[0u8; std::mem::size_of::<GpuLightData>() * MAX_LIGHTS],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lit Frame Bind Group"),
            layout: &lit_pipeline.frame_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        // Per-draw buffer (group 1)
        let (draw_uniform_buffer, draw_bind_group) =
            Self::create_draw_uniform_resources(device, &lit_pipeline, draw_uniform_stride, 1);

        Self {
            gpu,
            flat_pipeline,
            flat_uniform_buffer,
            flat_uniform_bind_group,
            flat_uniform_stride,
            flat_uniform_capacity: 1,
            lit_pipeline,
            frame_uniform_buffer,
            light_buffer,
            frame_bind_group,
            draw_uniform_buffer,
            draw_bind_group,
            draw_uniform_stride,
            draw_uniform_capacity: 1,
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

    /// Register a material and return a typed handle.
    #[must_use]
    pub fn create_material(&mut self, material: Material) -> MaterialHandle {
        let Material {
            shading,
            color,
            receive_light,
        } = material;
        let (color, lit, shininess, specular_strength) = match shading {
            ShadingMode::Flat { color } => (color, false, 1.0, 0.0),
            ShadingMode::Phong {
                shininess,
                specular_strength,
            } => (color, receive_light, shininess, specular_strength),
            ShadingMode::Pbr { .. } => (color, receive_light, 32.0, 0.5),
            _ => (color, receive_light, 32.0, 0.3),
        };
        let index = self.materials.len() as u32;
        self.materials.push(Some(GpuMaterial {
            color,
            lit,
            shininess,
            specular_strength,
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

    /// Render a frame into an offscreen capture target and read it back.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame references stale mesh/material handles or if
    /// readback fails.
    #[allow(clippy::too_many_lines)]
    pub fn capture(
        &mut self,
        frame: &Frame,
        target: &mut GpuCaptureTarget,
    ) -> Result<GpuDebugCapture, String> {
        let start = Instant::now();
        let (prepared_draws, flat_uniform_bytes, draw_uniform_bytes, total_triangles) =
            self.prepare_draws(frame)?;
        self.ensure_uniform_capacity(prepared_draws.len());

        // Upload flat uniforms
        if !flat_uniform_bytes.is_empty() {
            self.gpu
                .queue()
                .write_buffer(&self.flat_uniform_buffer, 0, &flat_uniform_bytes);
        }

        // Upload lit per-frame + per-draw uniforms
        let has_lit_draws = prepared_draws.iter().any(|d| d.lit);
        if has_lit_draws {
            let (frame_uniforms, gpu_lights) = self.prepare_frame_uniforms(frame);
            self.gpu.queue().write_buffer(
                &self.frame_uniform_buffer,
                0,
                bytemuck::bytes_of(&frame_uniforms),
            );
            self.gpu
                .queue()
                .write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&gpu_lights));
            if !draw_uniform_bytes.is_empty() {
                self.gpu
                    .queue()
                    .write_buffer(&self.draw_uniform_buffer, 0, &draw_uniform_bytes);
            }
        }

        let clear_color = frame
            .clear_color
            .map_or(wgpu::Color::BLACK, argb_to_wgpu_color);
        let background = argb_to_rgba_bytes(frame.clear_color.unwrap_or(0xFF00_0000));

        let mut encoder =
            self.gpu
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("GpuRenderer Capture Encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GpuRenderer Capture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        // Depth is never read back from capture targets — discard to
                        // skip the writeback and save bandwidth.
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            for draw in &prepared_draws {
                let gpu_mesh = self.meshes[draw.mesh_index].as_ref().ok_or_else(|| {
                    format!("stale mesh handle at command {}", draw.command_index)
                })?;

                if draw.lit {
                    pass.set_pipeline(&self.lit_pipeline.pipeline);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(1, &self.draw_bind_group, &[draw.uniform_offset]);
                } else {
                    pass.set_pipeline(&self.flat_pipeline.pipeline);
                    pass.set_bind_group(0, &self.flat_uniform_bind_group, &[draw.uniform_offset]);
                }

                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

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
                    rows_per_image: Some(target.config.height),
                },
            },
            wgpu::Extent3d {
                width: target.config.width,
                height: target.config.height,
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
        let height = target.config.height as usize;
        let total_bytes = unpadded_stride * height;
        let mut pixels_rgba = vec![0u8; total_bytes];
        if padded_stride == unpadded_stride {
            // No row padding — bulk copy the entire buffer at once.
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
                width: target.config.width,
                height: target.config.height,
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
        let (prepared_draws, flat_uniform_bytes, draw_uniform_bytes, _) =
            self.prepare_draws(frame)?;
        self.ensure_uniform_capacity(prepared_draws.len());

        if !flat_uniform_bytes.is_empty() {
            self.gpu
                .queue()
                .write_buffer(&self.flat_uniform_buffer, 0, &flat_uniform_bytes);
        }

        let has_lit_draws = prepared_draws.iter().any(|d| d.lit);
        if has_lit_draws {
            let (frame_uniforms, gpu_lights) = self.prepare_frame_uniforms(frame);
            self.gpu.queue().write_buffer(
                &self.frame_uniform_buffer,
                0,
                bytemuck::bytes_of(&frame_uniforms),
            );
            self.gpu
                .queue()
                .write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&gpu_lights));
            if !draw_uniform_bytes.is_empty() {
                self.gpu
                    .queue()
                    .write_buffer(&self.draw_uniform_buffer, 0, &draw_uniform_bytes);
            }
        }

        let clear_color = frame
            .clear_color
            .map_or(wgpu::Color::BLACK, argb_to_wgpu_color);
        let mut encoder =
            self.gpu
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("GpuRenderer Surface Encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GpuRenderer Surface Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &surface.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        // Depth is consumed in-pass for correct ordering; the final
                        // depth values are not needed after presentation.
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            for draw in &prepared_draws {
                let gpu_mesh = self.meshes[draw.mesh_index].as_ref().ok_or_else(|| {
                    format!("stale mesh handle at command {}", draw.command_index)
                })?;

                if draw.lit {
                    pass.set_pipeline(&self.lit_pipeline.pipeline);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(1, &self.draw_bind_group, &[draw.uniform_offset]);
                } else {
                    pass.set_pipeline(&self.flat_pipeline.pipeline);
                    pass.set_bind_group(0, &self.flat_uniform_bind_group, &[draw.uniform_offset]);
                }

                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        self.gpu.queue().submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }

    fn create_flat_uniform_resources(
        device: &wgpu::Device,
        pipeline: &MvpPipeline,
        uniform_stride: u64,
        slot_count: usize,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let initial_contents = vec![0; (uniform_stride * slot_count.max(1) as u64) as usize];
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flat Uniform Buffer"),
            contents: &initial_contents,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flat Uniform Bind Group"),
            layout: &pipeline.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: NonZeroU64::new(std::mem::size_of::<MvpUniform>() as u64),
                }),
            }],
        });

        (uniform_buffer, uniform_bind_group)
    }

    fn create_draw_uniform_resources(
        device: &wgpu::Device,
        pipeline: &LitPipeline,
        uniform_stride: u64,
        slot_count: usize,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let initial_contents = vec![0; (uniform_stride * slot_count.max(1) as u64) as usize];
        let draw_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lit Draw Uniform Buffer"),
            contents: &initial_contents,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let draw_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lit Draw Bind Group"),
            layout: &pipeline.draw_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &draw_buffer,
                    offset: 0,
                    size: NonZeroU64::new(std::mem::size_of::<DrawUniforms>() as u64),
                }),
            }],
        });

        (draw_buffer, draw_bind_group)
    }

    fn ensure_uniform_capacity(&mut self, draw_count: usize) {
        let required = draw_count.max(1);

        // Flat pipeline uniforms
        if required > self.flat_uniform_capacity {
            let new_capacity = required.next_power_of_two();
            let (buf, bg) = Self::create_flat_uniform_resources(
                self.gpu.device(),
                &self.flat_pipeline,
                self.flat_uniform_stride,
                new_capacity,
            );
            self.flat_uniform_buffer = buf;
            self.flat_uniform_bind_group = bg;
            self.flat_uniform_capacity = new_capacity;
        }

        // Lit pipeline draw uniforms
        if required > self.draw_uniform_capacity {
            let new_capacity = required.next_power_of_two();
            let (buf, bg) = Self::create_draw_uniform_resources(
                self.gpu.device(),
                &self.lit_pipeline,
                self.draw_uniform_stride,
                new_capacity,
            );
            self.draw_uniform_buffer = buf;
            self.draw_bind_group = bg;
            self.draw_uniform_capacity = new_capacity;
        }
    }

    /// Prepare per-frame uniform data (camera + lights) for the lit pipeline.
    fn prepare_frame_uniforms(&self, frame: &Frame) -> (FrameUniforms, Vec<GpuLightData>) {
        let view_proj = frame.camera.view * frame.camera.projection;
        let vp_flat: [f32; 16] = bytemuck::cast(view_proj.m);

        // Extract camera position from view matrix inverse
        // For a look-at view matrix, the camera position is embedded in the last row.
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

    fn prepare_draws(
        &self,
        frame: &Frame,
    ) -> Result<(Vec<PreparedDraw>, Vec<u8>, Vec<u8>, u32), String> {
        let view_proj = frame.camera.view * frame.camera.projection;
        let flat_uniform_size = std::mem::size_of::<MvpUniform>();
        let draw_uniform_size = std::mem::size_of::<DrawUniforms>();
        let mut prepared_draws = Vec::with_capacity(frame.commands.len());
        let mut flat_uniform_bytes =
            vec![0u8; (self.flat_uniform_stride * frame.commands.len().max(1) as u64) as usize];
        let mut draw_uniform_bytes =
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

            // Always prepare flat uniforms (needed for flat-shaded draws)
            let flat_uniform = MvpUniform::new(&(command.transform * view_proj), material.color);
            let flat_offset = command_index as u64 * self.flat_uniform_stride;
            let flat_dest = &mut flat_uniform_bytes
                [flat_offset as usize..flat_offset as usize + flat_uniform_size];
            flat_dest.copy_from_slice(bytemuck::bytes_of(&flat_uniform));

            // Always prepare draw uniforms (needed for lit draws)
            let draw_uniform = DrawUniforms::new(
                &command.transform,
                material.color,
                material.shininess,
                material.specular_strength,
            );
            let draw_offset = command_index as u64 * self.draw_uniform_stride;
            let draw_dest = &mut draw_uniform_bytes
                [draw_offset as usize..draw_offset as usize + draw_uniform_size];
            draw_dest.copy_from_slice(bytemuck::bytes_of(&draw_uniform));

            let uniform_offset = u32::try_from(flat_offset)
                .map_err(|_| "uniform buffer offset exceeds u32".to_string())?;
            let draw_uniform_offset = u32::try_from(draw_offset)
                .map_err(|_| "draw uniform buffer offset exceeds u32".to_string())?;
            prepared_draws.push(PreparedDraw {
                command_index,
                mesh_index,
                uniform_offset,
                triangle_count: gpu_mesh.triangle_count,
                color: material.color,
                lit: material.lit,
            });
            // Store the draw uniform offset in the same field for lit draws
            if material.lit {
                prepared_draws.last_mut().unwrap().uniform_offset = draw_uniform_offset;
            }
            total_triangles = total_triangles.saturating_add(gpu_mesh.triangle_count);
        }

        if frame.commands.is_empty() {
            flat_uniform_bytes.clear();
            draw_uniform_bytes.clear();
        }

        Ok((
            prepared_draws,
            flat_uniform_bytes,
            draw_uniform_bytes,
            total_triangles,
        ))
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
