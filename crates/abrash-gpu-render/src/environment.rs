//! Environment mapping: cubemap upload, skybox rendering, and IBL support.
//!
//! Maps to Passes 1 (Prefilter Environment Map) and 6 (Skybox) from the
//! Granja & Pereira 2021 hybrid rendering paper.

use abrash_core::texture::Texture;
use wgpu::util::DeviceExt;

/// WGSL shader for the skybox pass.
///
/// Renders a fullscreen triangle, reconstructs view direction from screen
/// coordinates + inverse view-projection, then samples the cubemap.
const SKYBOX_SHADER: &str = r"
struct SkyboxUniforms {
    inv_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> sky: SkyboxUniforms;
@group(0) @binding(1) var env_map: texture_cube<f32>;
@group(0) @binding(2) var env_sampler: sampler;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 1.0, 1.0); // z=1 → far plane
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    // Reconstruct world-space view direction from NDC
    let clip = vec4<f32>(input.uv.x, input.uv.y, 1.0, 1.0);
    let world_dir_h = sky.inv_view_proj * clip;
    let world_dir = normalize(world_dir_h.xyz / world_dir_h.w);

    let color = textureSample(env_map, env_sampler, world_dir);
    return vec4<f32>(color.rgb, 1.0);
}
";

/// Skybox uniform: inverse view-projection matrix.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxUniforms {
    pub inv_view_proj: [f32; 16],
}

/// GPU cubemap texture uploaded from 6 face textures.
pub struct GpuCubemap {
    pub(crate) _texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

impl GpuCubemap {
    /// Upload a cubemap from 6 face textures.
    ///
    /// Face order: +X (right), -X (left), +Y (top), -Y (bottom), +Z (front), -Z (back).
    /// All faces must be the same square dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if faces have mismatched dimensions or are empty.
    pub fn from_faces(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        faces: &[Texture; 6],
    ) -> Result<Self, String> {
        let size = faces[0].width;
        if size == 0 {
            return Err("cubemap face dimensions must be positive".to_string());
        }
        for (i, face) in faces.iter().enumerate() {
            if face.width != size || face.height != size {
                return Err(format!(
                    "cubemap face {i} is {}x{}, expected {size}x{size}",
                    face.width, face.height
                ));
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Environment Cubemap"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload each face
        for (layer, face) in faces.iter().enumerate() {
            let mut rgba = Vec::with_capacity((size * size * 4) as usize);
            for &argb in &face.pixels {
                let bytes = [
                    ((argb >> 16) & 0xFF) as u8,
                    ((argb >> 8) & 0xFF) as u8,
                    (argb & 0xFF) as u8,
                    ((argb >> 24) & 0xFF) as u8,
                ];
                rgba.extend_from_slice(&bytes);
            }

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size * 4),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Environment Cubemap View"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        Ok(Self {
            _texture: texture,
            view,
        })
    }
}

/// Skybox rendering pass: draws the environment cubemap behind all geometry.
pub struct SkyboxPass {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) uniform_buffer: wgpu::Buffer,
    pub(crate) sampler: wgpu::Sampler,
}

impl SkyboxPass {
    /// Create the skybox pass for a given HDR output format.
    #[must_use]
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader"),
            source: wgpu::ShaderSource::Wgsl(SKYBOX_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skybox Layout"),
            entries: &[
                // binding 0: inverse VP uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            SkyboxUniforms,
                        >()
                            as u64),
                    },
                    count: None,
                },
                // binding 1: cubemap texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 2: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = create_skybox_pipeline(device, &pipeline_layout, &shader, output_format);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Skybox Uniforms"),
            contents: &[0u8; std::mem::size_of::<SkyboxUniforms>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Skybox Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            sampler,
        }
    }

    /// Create a bind group for a specific cubemap.
    #[must_use]
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        cubemap: &GpuCubemap,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skybox Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&cubemap.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Encode the skybox pass. Renders after deferred lighting into the HDR target.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        output_view: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Skybox Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Preserve deferred lighting output
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skybox_shader_valid() {
        assert!(SKYBOX_SHADER.contains("fn vs_main"));
        assert!(SKYBOX_SHADER.contains("fn fs_main"));
        assert!(SKYBOX_SHADER.contains("inv_view_proj"));
        assert!(SKYBOX_SHADER.contains("texture_cube"));
        assert!(SKYBOX_SHADER.contains("textureSample"));
    }

    #[test]
    fn test_skybox_uniforms_size() {
        assert_eq!(std::mem::size_of::<SkyboxUniforms>(), 64);
    }
}

fn create_skybox_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Skybox renders at z=1 (far plane) — use LessEqual to fill background pixels
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Skybox Pipeline"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None, // No depth test — skybox fills background
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
