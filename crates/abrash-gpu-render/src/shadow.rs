//! Shadow mapping infrastructure: depth-only pass + shadow map texture.

use crate::shader::LitVertex;
use abrash_core::math::{Mat4, Vec3};
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

/// Shadow map resolution (square).
pub const SHADOW_MAP_SIZE: u32 = 2048;

/// Shadow map depth texture + depth-only pipeline + light-space matrix.
pub struct ShadowMap {
    #[allow(dead_code)]
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) uniform_buffer: wgpu::Buffer,
    pub(crate) uniform_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for sampling the shadow map in the main pass.
    pub(crate) sample_bind_group: wgpu::BindGroup,
    pub(crate) sample_bind_group_layout: wgpu::BindGroupLayout,
    /// Buffer holding the light VP matrix for main-pass shadow sampling.
    pub(crate) light_vp_buffer: wgpu::Buffer,
    /// Light-space view-projection matrix (updated per frame).
    pub(crate) light_vp: Mat4,
}

/// Uniform for the shadow depth pass: just a light-space VP matrix.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniforms {
    /// Light-space view-projection matrix (row-major).
    pub light_vp: [f32; 16],
    /// Per-draw model matrix (row-major).
    pub model: [f32; 16],
}

/// WGSL for the depth-only shadow pass. No fragment output — depth is written automatically.
const SHADOW_DEPTH_SHADER: &str = r"
struct ShadowUniforms {
    light_vp: mat4x4<f32>,
    model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> shadow: ShadowUniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(input: VsIn) -> @builtin(position) vec4<f32> {
    let world_pos = shadow.model * vec4<f32>(input.position, 1.0);
    return shadow.light_vp * world_pos;
}
";

impl ShadowMap {
    /// Create a shadow map with depth-only pipeline.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(device: &wgpu::Device) -> Self {
        // Depth texture (Depth32Float so it can be sampled)
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Depth"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Depth Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_DEPTH_SHADER.into()),
        });

        // Uniform layout (light_vp + model per draw)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<ShadowUniforms>() as u64),
                },
                count: None,
            }],
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shadow Uniform Buffer"),
            contents: &[0u8; std::mem::size_of::<ShadowUniforms>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        // Depth-only pipeline (no fragment shader)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Depth Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[LitVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        // Sampling bind group for the main pass (comparison sampler + depth texture)
        let sample_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Sample Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(64), // light_vp mat4
                        },
                        count: None,
                    },
                ],
            });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Comparison Sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Light VP buffer for the main pass (just the 4x4 matrix)
        let light_vp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shadow Light VP"),
            contents: &[0u8; 64],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sample_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Sample Bind Group"),
            layout: &sample_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light_vp_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            depth_texture,
            depth_view,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            bind_group_layout,
            sample_bind_group,
            sample_bind_group_layout,
            light_vp_buffer,
            light_vp: Mat4::identity(),
        }
    }

    /// Compute a light-space view-projection matrix for a directional light.
    ///
    /// Uses an orthographic projection centered on the scene with the given extent.
    #[must_use]
    pub fn compute_directional_light_vp(direction: Vec3, scene_extent: f32) -> Mat4 {
        let light_dir = direction.normalize();
        // Position the light far enough to cover the scene
        let light_pos = light_dir * (-scene_extent * 2.0);
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let light_view = Mat4::look_at(light_pos, Vec3::ZERO, up);
        let light_proj = Mat4::orthographic(
            -scene_extent,
            scene_extent,
            -scene_extent,
            scene_extent,
            0.1,
            scene_extent * 4.0,
        );
        light_view * light_proj
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_uniforms_size() {
        assert_eq!(std::mem::size_of::<ShadowUniforms>(), 128);
    }

    #[test]
    fn test_shadow_depth_shader_valid() {
        assert!(SHADOW_DEPTH_SHADER.contains("vs_main"));
        assert!(SHADOW_DEPTH_SHADER.contains("light_vp"));
        assert!(SHADOW_DEPTH_SHADER.contains("model"));
    }

    #[test]
    fn test_directional_light_vp() {
        let vp = ShadowMap::compute_directional_light_vp(Vec3::new(0.0, -1.0, 0.0), 50.0);
        // Should produce a valid matrix (not NaN/Inf)
        for row in &vp.m {
            for &val in row {
                assert!(val.is_finite(), "light VP should be finite");
            }
        }
    }
}
