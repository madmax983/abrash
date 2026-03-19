//! `GpuRenderer` - the GPU rendering backend.

use crate::capture::{BatchStats, FrameStats, GpuCaptureTarget, GpuDebugCapture};
use crate::device::{GpuDevice, GpuDeviceConfig};
use crate::mesh_buffer::GpuMeshBuffer;
use crate::shader::{MvpPipeline, MvpUniform};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::frame::Frame;
use abrash_render::render_api::handles::{MaterialHandle, MeshHandle};
use abrash_render::render_api::material::{Material, ShadingMode};
use std::num::NonZeroU64;
#[cfg(feature = "windowed")]
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
#[cfg(feature = "windowed")]
use winit::window::Window;

struct GpuMaterial {
    color: u32,
}

struct PreparedDraw {
    command_index: usize,
    mesh_index: usize,
    uniform_offset: u32,
    triangle_count: u32,
    color: u32,
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
    pipeline: MvpPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_stride: u64,
    uniform_capacity: usize,
    meshes: Vec<Option<GpuMeshBuffer>>,
    materials: Vec<Option<GpuMaterial>>,
}

impl GpuRenderer {
    /// Construct a renderer from an already-created GPU device.
    #[must_use]
    pub fn from_gpu(gpu: GpuDevice, color_format: wgpu::TextureFormat) -> Self {
        let pipeline = MvpPipeline::new(gpu.device(), color_format);
        let uniform_stride = align_to(
            std::mem::size_of::<MvpUniform>() as u64,
            u64::from(gpu.device().limits().min_uniform_buffer_offset_alignment).max(1),
        );
        let (uniform_buffer, uniform_bind_group) =
            Self::create_uniform_resources(gpu.device(), &pipeline, uniform_stride, 1);

        Self {
            gpu,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            uniform_stride,
            uniform_capacity: 1,
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
            receive_light: _,
        } = material;
        let color = match shading {
            ShadingMode::Flat { color } => color,
            _ => color,
        };
        let index = self.materials.len() as u32;
        self.materials.push(Some(GpuMaterial { color }));
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
        let (prepared_draws, uniform_bytes, total_triangles) = self.prepare_draws(frame)?;
        self.ensure_uniform_capacity(prepared_draws.len());
        if !uniform_bytes.is_empty() {
            self.gpu
                .queue()
                .write_buffer(&self.uniform_buffer, 0, &uniform_bytes);
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
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline.pipeline);

            for draw in &prepared_draws {
                let gpu_mesh = self.meshes[draw.mesh_index].as_ref().ok_or_else(|| {
                    format!("stale mesh handle at command {}", draw.command_index)
                })?;

                pass.set_bind_group(0, &self.uniform_bind_group, &[draw.uniform_offset]);
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &target.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &target.readback_buffer,
                layout: wgpu::ImageDataLayout {
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
        let _ = self.gpu.device().poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|error| format!("readback channel error: {error}"))?
            .map_err(|error| format!("readback map error: {error}"))?;

        let mapped = buffer_slice.get_mapped_range();
        let padded_stride = target.config.padded_bytes_per_row as usize;
        let unpadded_stride = target.config.unpadded_bytes_per_row as usize;
        let height = target.config.height as usize;
        let mut pixels_rgba = Vec::with_capacity(unpadded_stride * height);
        for row in 0..height {
            let row_start = row * padded_stride;
            pixels_rgba.extend_from_slice(&mapped[row_start..row_start + unpadded_stride]);
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
        let output = surface
            .surface
            .get_current_texture()
            .map_err(|error| format!("surface error: {error}"))?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let (prepared_draws, uniform_bytes, _) = self.prepare_draws(frame)?;
        self.ensure_uniform_capacity(prepared_draws.len());
        if !uniform_bytes.is_empty() {
            self.gpu
                .queue()
                .write_buffer(&self.uniform_buffer, 0, &uniform_bytes);
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
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &surface.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline.pipeline);
            for draw in &prepared_draws {
                let gpu_mesh = self.meshes[draw.mesh_index].as_ref().ok_or_else(|| {
                    format!("stale mesh handle at command {}", draw.command_index)
                })?;

                pass.set_bind_group(0, &self.uniform_bind_group, &[draw.uniform_offset]);
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        self.gpu.queue().submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }

    fn create_uniform_resources(
        device: &wgpu::Device,
        pipeline: &MvpPipeline,
        uniform_stride: u64,
        slot_count: usize,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let initial_contents = vec![0; (uniform_stride * slot_count.max(1) as u64) as usize];
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GpuRenderer Uniform Buffer"),
            contents: &initial_contents,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GpuRenderer Uniform Bind Group"),
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

    fn ensure_uniform_capacity(&mut self, draw_count: usize) {
        let required = draw_count.max(1);
        if required <= self.uniform_capacity {
            return;
        }

        let (uniform_buffer, uniform_bind_group) = Self::create_uniform_resources(
            self.gpu.device(),
            &self.pipeline,
            self.uniform_stride,
            required,
        );
        self.uniform_buffer = uniform_buffer;
        self.uniform_bind_group = uniform_bind_group;
        self.uniform_capacity = required;
    }

    fn prepare_draws(&self, frame: &Frame) -> Result<(Vec<PreparedDraw>, Vec<u8>, u32), String> {
        let view_proj = frame.camera.view * frame.camera.projection;
        let uniform_size = std::mem::size_of::<MvpUniform>();
        let mut prepared_draws = Vec::with_capacity(frame.commands.len());
        let mut uniform_bytes =
            vec![0u8; (self.uniform_stride * frame.commands.len().max(1) as u64) as usize];
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

            let uniform = MvpUniform::new(&(command.transform * view_proj), material.color);
            let offset = command_index as u64 * self.uniform_stride;
            let destination = &mut uniform_bytes[offset as usize..offset as usize + uniform_size];
            destination.copy_from_slice(bytemuck::bytes_of(&uniform));

            let uniform_offset = u32::try_from(offset)
                .map_err(|_| "uniform buffer offset exceeds u32".to_string())?;
            prepared_draws.push(PreparedDraw {
                command_index,
                mesh_index,
                uniform_offset,
                triangle_count: gpu_mesh.triangle_count,
                color: material.color,
            });
            total_triangles = total_triangles.saturating_add(gpu_mesh.triangle_count);
        }

        if frame.commands.is_empty() {
            uniform_bytes.clear();
        }

        Ok((prepared_draws, uniform_bytes, total_triangles))
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
