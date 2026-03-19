//! MVP WGSL shader and render pipeline construction.

use abrash_core::math::Mat4;
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU64;

/// WGSL shader source for flat-color MVP rendering.
pub const MVP_SHADER_SRC: &str = r"
struct Uniforms {
    mvp: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_position = uniforms.mvp * vec4<f32>(input.position, 1.0);
    out.color = uniforms.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    return input.color;
}
";

/// GPU uniform data for a single draw: MVP matrix plus flat RGBA color.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MvpUniform {
    /// Combined model-view-projection matrix flattened in row-major order.
    pub mvp: [f32; 16],
    /// Linear RGBA color.
    pub color: [f32; 4],
}

impl MvpUniform {
    /// Build a uniform packet from a matrix and `0xAARRGGBB` color.
    #[must_use]
    pub fn new(mvp: &Mat4, argb: u32) -> Self {
        let mut flat = [0.0; 16];
        for (row_index, row) in mvp.m.iter().enumerate() {
            for (column_index, value) in row.iter().copied().enumerate() {
                flat[row_index * 4 + column_index] = value;
            }
        }

        let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
        let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
        let b = (argb & 0xFF) as f32 / 255.0;

        Self {
            mvp: flat,
            color: [r, g, b, a],
        }
    }
}

/// Position-only vertex used by the MVP shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MvpVertex {
    /// Local-space position.
    pub position: [f32; 3],
}

impl MvpVertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 1] =
        wgpu::vertex_attr_array![0 => Float32x3];

    #[must_use]
    pub(crate) const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Reusable MVP render pipeline shared by headless and windowed paths.
pub struct MvpPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl MvpPipeline {
    /// Create a render pipeline for the given color target format.
    #[must_use]
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Abrash MVP Shader"),
            source: wgpu::ShaderSource::Wgsl(MVP_SHADER_SRC.into()),
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MVP Uniform Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<MvpUniform>() as u64),
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Abrash MVP Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Abrash MVP Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[MvpVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
        });

        Self {
            pipeline,
            uniform_bind_group_layout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvp_uniform_size() {
        assert_eq!(std::mem::size_of::<MvpUniform>(), 80);
    }

    #[test]
    fn test_mvp_uniform_from_mat4() {
        use abrash_core::math::Mat4;

        let identity = Mat4::identity();
        let uniform = MvpUniform::new(&identity, 0xFFFF_0000);

        assert!((uniform.mvp[0] - 1.0).abs() < f32::EPSILON);
        assert!(uniform.mvp[1].abs() < f32::EPSILON);
        assert!((uniform.color[0] - 1.0).abs() < f32::EPSILON);
        assert!(uniform.color[1].abs() < f32::EPSILON);
        assert!(uniform.color[2].abs() < f32::EPSILON);
        assert!((uniform.color[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_shader_source_is_valid_wgsl() {
        assert!(MVP_SHADER_SRC.contains("fn vs_main"));
        assert!(MVP_SHADER_SRC.contains("fn fs_main"));
        assert!(MVP_SHADER_SRC.contains("mvp"));
    }

    #[test]
    fn test_mvp_vertex_layout_stride() {
        let layout = MvpVertex::layout();
        assert_eq!(layout.array_stride, 12);
        assert_eq!(layout.attributes.len(), 1);
    }
}
