//! G-Buffer: Multiple Render Targets for deferred rendering.
//!
//! The G-Buffer stores per-pixel geometric and material data written during
//! the geometry pass. The deferred lighting pass then reads these textures
//! to compute lighting in screen space, decoupling geometry complexity from
//! lighting complexity.
//!
//! # Render targets
//!
//! | Target   | Format        | Contents                          |
//! |----------|---------------|-----------------------------------|
//! | Position | Rgba32Float   | World-space XYZ + spare            |
//! | Normal   | Rgba16Float   | World-space normal XYZ + roughness |
//! | Albedo   | Rgba8Unorm    | Base color RGB + metallic          |
//! | Depth    | Depth24Plus   | Shared depth buffer                |

use crate::shader::{DrawUniforms, LitVertex, TexturedVertex};
use std::num::NonZeroU64;

/// G-Buffer format constants.
pub const POSITION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
pub const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const ALBEDO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// G-Buffer render targets.
pub struct GBuffer {
    pub(crate) position_texture: wgpu::Texture,
    pub(crate) position_view: wgpu::TextureView,
    pub(crate) normal_texture: wgpu::Texture,
    pub(crate) normal_view: wgpu::TextureView,
    pub(crate) albedo_texture: wgpu::Texture,
    pub(crate) albedo_view: wgpu::TextureView,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl GBuffer {
    /// Create G-Buffer textures at the given dimensions.
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let make_texture =
            |label, format, usage: wgpu::TextureUsages| -> (wgpu::Texture, wgpu::TextureView) {
                let tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                });
                let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                (tex, view)
            };

        let color_usage =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let depth_usage = wgpu::TextureUsages::RENDER_ATTACHMENT;

        let (position_texture, position_view) =
            make_texture("GBuffer Position", POSITION_FORMAT, color_usage);
        let (normal_texture, normal_view) =
            make_texture("GBuffer Normal", NORMAL_FORMAT, color_usage);
        let (albedo_texture, albedo_view) =
            make_texture("GBuffer Albedo", ALBEDO_FORMAT, color_usage);
        let (depth_texture, depth_view) = make_texture("GBuffer Depth", DEPTH_FORMAT, depth_usage);

        Self {
            position_texture,
            position_view,
            normal_texture,
            normal_view,
            albedo_texture,
            albedo_view,
            depth_texture,
            depth_view,
            width,
            height,
        }
    }
}

/// WGSL shader for the G-Buffer geometry pass.
///
/// Writes world position, normal+roughness, and albedo+metallic to 3 render targets.
/// No lighting computation — that happens in the deferred lighting pass.
pub const GBUFFER_GEOMETRY_SHADER: &str = r"
struct DrawUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    shininess: f32,
    specular_strength: f32,
    metallic: f32,
    roughness: f32,
};

struct FrameUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(0) var<uniform> draw: DrawUniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

struct GBufferOutput {
    @location(0) position: vec4<f32>,
    @location(1) normal_roughness: vec4<f32>,
    @location(2) albedo_metallic: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let world_pos = draw.model * vec4<f32>(input.position, 1.0);
    out.clip_position = frame.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = (draw.model * vec4<f32>(input.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> GBufferOutput {
    var out: GBufferOutput;
    let N = normalize(input.world_normal);
    out.position = vec4<f32>(input.world_pos, 1.0);
    out.normal_roughness = vec4<f32>(N, draw.roughness);
    out.albedo_metallic = vec4<f32>(draw.color.rgb, draw.metallic);
    return out;
}
";

/// WGSL shader for the textured G-Buffer geometry pass (with UV sampling).
pub const GBUFFER_TEXTURED_GEOMETRY_SHADER: &str = r"
struct DrawUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    shininess: f32,
    specular_strength: f32,
    metallic: f32,
    roughness: f32,
};

struct FrameUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(0) var<uniform> draw: DrawUniforms;
@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct GBufferOutput {
    @location(0) position: vec4<f32>,
    @location(1) normal_roughness: vec4<f32>,
    @location(2) albedo_metallic: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let world_pos = draw.model * vec4<f32>(input.position, 1.0);
    out.clip_position = frame.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = (draw.model * vec4<f32>(input.normal, 0.0)).xyz;
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> GBufferOutput {
    var out: GBufferOutput;
    let N = normalize(input.world_normal);
    let tex_color = textureSample(albedo_texture, albedo_sampler, input.uv);
    out.position = vec4<f32>(input.world_pos, 1.0);
    out.normal_roughness = vec4<f32>(N, draw.roughness);
    out.albedo_metallic = vec4<f32>(tex_color.rgb * draw.color.rgb, draw.metallic);
    return out;
}
";

/// G-Buffer geometry pipeline (non-textured).
pub struct GBufferGeometryPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) frame_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) draw_bind_group_layout: wgpu::BindGroupLayout,
}

/// G-Buffer textured geometry pipeline.
pub struct GBufferTexturedPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
}

impl GBufferGeometryPipeline {
    /// Create the G-Buffer geometry pipeline.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GBuffer Geometry Shader"),
            source: wgpu::ShaderSource::Wgsl(GBUFFER_GEOMETRY_SHADER.into()),
        });

        // Group 0: per-frame (only view_proj needed, reuse FrameUniforms)
        let frame_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GBuffer Frame Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<
                            crate::shader::FrameUniforms,
                        >() as u64),
                    },
                    count: None,
                }],
            });

        // Group 1: per-draw (model + material)
        let draw_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GBuffer Draw Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: NonZeroU64::new(
                            std::mem::size_of::<DrawUniforms>() as u64
                        ),
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GBuffer Geometry Pipeline Layout"),
            bind_group_layouts: &[
                Some(&frame_bind_group_layout),
                Some(&draw_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GBuffer Geometry Pipeline"),
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
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: POSITION_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: NORMAL_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: ALBEDO_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            frame_bind_group_layout,
            draw_bind_group_layout,
        }
    }
}

impl GBufferTexturedPipeline {
    /// Create the textured G-Buffer geometry pipeline.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        draw_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GBuffer Textured Geometry Shader"),
            source: wgpu::ShaderSource::Wgsl(GBUFFER_TEXTURED_GEOMETRY_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GBuffer Textured Pipeline Layout"),
            bind_group_layouts: &[Some(frame_layout), Some(draw_layout), Some(texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GBuffer Textured Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[TexturedVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: POSITION_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: NORMAL_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: ALBEDO_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        Self { pipeline }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbuffer_geometry_shader_valid() {
        assert!(GBUFFER_GEOMETRY_SHADER.contains("fn vs_main"));
        assert!(GBUFFER_GEOMETRY_SHADER.contains("fn fs_main"));
        assert!(GBUFFER_GEOMETRY_SHADER.contains("GBufferOutput"));
        assert!(GBUFFER_GEOMETRY_SHADER.contains("@location(0)"));
        assert!(GBUFFER_GEOMETRY_SHADER.contains("@location(1)"));
        assert!(GBUFFER_GEOMETRY_SHADER.contains("@location(2)"));
    }

    #[test]
    fn test_gbuffer_textured_shader_valid() {
        assert!(GBUFFER_TEXTURED_GEOMETRY_SHADER.contains("fn vs_main"));
        assert!(GBUFFER_TEXTURED_GEOMETRY_SHADER.contains("fn fs_main"));
        assert!(GBUFFER_TEXTURED_GEOMETRY_SHADER.contains("albedo_texture"));
        assert!(GBUFFER_TEXTURED_GEOMETRY_SHADER.contains("textureSample"));
    }
}
