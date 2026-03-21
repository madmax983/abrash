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
    @location(1) normal: vec3<f32>,
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
        // Mat4 is repr(C) with [[f32; 4]; 4] — identical layout to [f32; 16].
        let flat: [f32; 16] = bytemuck::cast(mvp.m);

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

/// Position + normal vertex for lit rendering.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LitVertex {
    /// Local-space position.
    pub position: [f32; 3],
    /// Vertex normal (unit-length).
    pub normal: [f32; 3],
}

impl LitVertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    #[must_use]
    pub(crate) const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Position + normal + UV vertex for textured lit rendering.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TexturedVertex {
    /// Local-space position.
    pub position: [f32; 3],
    /// Vertex normal (unit-length).
    pub normal: [f32; 3],
    /// Texture coordinate (u, v).
    pub uv: [f32; 2],
}

impl TexturedVertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    #[must_use]
    pub(crate) const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

// ---------------------------------------------------------------------------
// Lit rendering: per-frame + per-draw uniforms with light support
// ---------------------------------------------------------------------------

/// Maximum number of lights supported per frame.
pub const MAX_LIGHTS: usize = 8;

/// Per-frame uniform data: camera + light metadata.
///
/// Bound at group 0, binding 0.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrameUniforms {
    /// View-projection matrix (row-major).
    pub view_proj: [f32; 16],
    /// Camera world-space position (xyz) + padding.
    pub camera_pos: [f32; 4],
    /// Number of active lights (u32 stored as f32 for uniform alignment).
    pub light_count: u32,
    pub _pad: [u32; 3],
}

/// GPU-side light data. Each light is 48 bytes (3 × vec4).
///
/// Stored in a uniform buffer at group 0, binding 1.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuLightData {
    /// For directional: normalized direction. For point: world position.
    pub position_or_direction: [f32; 3],
    /// 0 = directional, 1 = point.
    pub light_type: u32,
    /// Linear RGB color.
    pub color: [f32; 3],
    /// Intensity multiplier.
    pub intensity: f32,
    /// Attenuation radius (point lights only).
    pub radius: f32,
    pub _pad: [f32; 3],
}

/// Per-draw uniform data: model matrix + material properties.
///
/// Bound at group 1, binding 0 with dynamic offsets.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DrawUniforms {
    /// Model matrix (row-major).
    pub model: [f32; 16],
    /// Linear RGBA color.
    pub color: [f32; 4],
    /// Phong shininess exponent.
    pub shininess: f32,
    /// Specular intensity (0.0–1.0).
    pub specular_strength: f32,
    /// PBR metallic (0.0–1.0).
    pub metallic: f32,
    /// PBR roughness (0.0–1.0).
    pub roughness: f32,
}

impl DrawUniforms {
    /// Build draw uniforms from a model matrix and material properties.
    #[must_use]
    pub fn new(model: &Mat4, argb: u32, shininess: f32, specular_strength: f32) -> Self {
        let flat: [f32; 16] = bytemuck::cast(model.m);
        let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
        let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
        let b = (argb & 0xFF) as f32 / 255.0;

        Self {
            model: flat,
            color: [r, g, b, a],
            shininess,
            specular_strength,
            metallic: 0.0,
            roughness: 0.5,
        }
    }
}

/// WGSL shader source for Blinn-Phong lit rendering.
pub const LIT_SHADER_SRC: &str = r"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct LightData {
    position_or_direction: vec3<f32>,
    light_type: u32,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct DrawUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    shininess: f32,
    specular_strength: f32,
    metallic: f32,
    roughness: f32,
};

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(0) @binding(1) var<uniform> lights: array<LightData, 8>;
@group(1) @binding(0) var<uniform> draw: DrawUniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let world_pos = draw.model * vec4<f32>(input.position, 1.0);
    out.clip_position = frame.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    // For uniform scale, model matrix transforms normals correctly.
    // For non-uniform scale, use transpose(inverse(model)) — deferred to Phase 5.
    out.world_normal = (draw.model * vec4<f32>(input.normal, 0.0)).xyz;
    out.color = draw.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let N = normalize(input.world_normal);
    let V = normalize(frame.camera_pos.xyz - input.world_pos);
    let base_color = input.color.rgb;

    // Ambient
    var result = base_color * 0.08;

    for (var i = 0u; i < frame.light_count; i = i + 1u) {
        let light = lights[i];

        var L: vec3<f32>;
        var attenuation: f32 = 1.0;

        if (light.light_type == 0u) {
            // Directional light
            L = normalize(-light.position_or_direction);
        } else {
            // Point light
            let to_light = light.position_or_direction - input.world_pos;
            let dist = length(to_light);
            L = to_light / max(dist, 0.0001);
            let r = max(light.radius, 0.0001);
            attenuation = 1.0 / (1.0 + (dist * dist) / (r * r));
        }

        // Diffuse (Lambertian)
        let NdotL = max(dot(N, L), 0.0);
        let diffuse = base_color * NdotL;

        // Specular (Blinn-Phong)
        let H = normalize(L + V);
        let NdotH = max(dot(N, H), 0.0);
        let specular = vec3<f32>(draw.specular_strength) * pow(NdotH, draw.shininess);

        result = result + (diffuse + specular) * light.color * light.intensity * attenuation;
    }

    return vec4<f32>(result, input.color.a);
}
";

/// Reusable MVP render pipeline shared by headless and windowed paths.
pub struct MvpPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) uniform_bind_group_layout: wgpu::BindGroupLayout,
}

/// Lit render pipeline with per-frame + per-draw bind groups.
pub struct LitPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    /// Group 0: per-frame (view_proj, camera_pos, lights).
    pub(crate) frame_bind_group_layout: wgpu::BindGroupLayout,
    /// Group 1: per-draw with dynamic offset (model, material).
    pub(crate) draw_bind_group_layout: wgpu::BindGroupLayout,
}

impl LitPipeline {
    /// Create a lit render pipeline for the given color target format.
    #[must_use]
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Abrash Lit Shader"),
            source: wgpu::ShaderSource::Wgsl(LIT_SHADER_SRC.into()),
        });

        // Group 0: per-frame uniforms + lights array
        let frame_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Lit Frame Layout"),
                entries: &[
                    // binding 0: FrameUniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                std::mem::size_of::<FrameUniforms>() as u64
                            ),
                        },
                        count: None,
                    },
                    // binding 1: lights array
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                (std::mem::size_of::<GpuLightData>() * MAX_LIGHTS) as u64,
                            ),
                        },
                        count: None,
                    },
                ],
            });

        // Group 1: per-draw uniforms (dynamic offset)
        let draw_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Lit Draw Layout"),
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
            label: Some("Abrash Lit Pipeline Layout"),
            bind_group_layouts: &[
                Some(&frame_bind_group_layout),
                Some(&draw_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Abrash Lit Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[LitVertex::layout()],
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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

/// WGSL shader source for textured + lit rendering.
pub const TEXTURED_LIT_SHADER_SRC: &str = r"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct LightData {
    position_or_direction: vec3<f32>,
    light_type: u32,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct DrawUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    shininess: f32,
    specular_strength: f32,
    metallic: f32,
    roughness: f32,
};

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(0) @binding(1) var<uniform> lights: array<LightData, 8>;
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
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let world_pos = draw.model * vec4<f32>(input.position, 1.0);
    out.clip_position = frame.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = (draw.model * vec4<f32>(input.normal, 0.0)).xyz;
    out.uv = input.uv;
    out.color = draw.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let N = normalize(input.world_normal);
    let V = normalize(frame.camera_pos.xyz - input.world_pos);

    // Sample texture and multiply with vertex/material color
    let tex_color = textureSample(albedo_texture, albedo_sampler, input.uv);
    let base_color = tex_color.rgb * input.color.rgb;

    // Ambient
    var result = base_color * 0.08;

    for (var i = 0u; i < frame.light_count; i = i + 1u) {
        let light = lights[i];

        var L: vec3<f32>;
        var attenuation: f32 = 1.0;

        if (light.light_type == 0u) {
            L = normalize(-light.position_or_direction);
        } else {
            let to_light = light.position_or_direction - input.world_pos;
            let dist = length(to_light);
            L = to_light / max(dist, 0.0001);
            let r = max(light.radius, 0.0001);
            attenuation = 1.0 / (1.0 + (dist * dist) / (r * r));
        }

        let NdotL = max(dot(N, L), 0.0);
        let diffuse = base_color * NdotL;

        let H = normalize(L + V);
        let NdotH = max(dot(N, H), 0.0);
        let specular = vec3<f32>(draw.specular_strength) * pow(NdotH, draw.shininess);

        result = result + (diffuse + specular) * light.color * light.intensity * attenuation;
    }

    return vec4<f32>(result, tex_color.a * input.color.a);
}
";

/// Textured + lit render pipeline using all 3 bind groups.
pub struct TexturedLitPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) frame_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) draw_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl TexturedLitPipeline {
    /// Create a textured lit render pipeline.
    ///
    /// Takes an externally-created texture bind group layout so it can be shared
    /// with the renderer's texture upload code.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Abrash Textured Lit Shader"),
            source: wgpu::ShaderSource::Wgsl(TEXTURED_LIT_SHADER_SRC.into()),
        });

        // Reuse the same group 0 and group 1 layouts as LitPipeline
        let frame_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Textured Frame Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                std::mem::size_of::<FrameUniforms>() as u64
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                (std::mem::size_of::<GpuLightData>() * MAX_LIGHTS) as u64,
                            ),
                        },
                        count: None,
                    },
                ],
            });

        let draw_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Textured Draw Layout"),
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
            label: Some("Abrash Textured Lit Pipeline Layout"),
            bind_group_layouts: &[
                Some(&frame_bind_group_layout),
                Some(&draw_bind_group_layout),
                Some(texture_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Abrash Textured Lit Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[TexturedVertex::layout()],
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            frame_bind_group_layout,
            draw_bind_group_layout,
            texture_bind_group_layout: texture_bind_group_layout.clone(),
        }
    }
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
            bind_group_layouts: &[Some(&uniform_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Abrash MVP Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[LitVertex::layout()],
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
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

    #[test]
    fn test_lit_vertex_size_and_layout() {
        assert_eq!(std::mem::size_of::<LitVertex>(), 24);
        let layout = LitVertex::layout();
        assert_eq!(layout.array_stride, 24);
        assert_eq!(layout.attributes.len(), 2);
    }

    #[test]
    fn test_frame_uniforms_size() {
        // 16×f32 (view_proj) + 4×f32 (camera_pos) + u32 + 3×u32 pad = 64 + 16 + 16 = 96
        let size = std::mem::size_of::<FrameUniforms>();
        assert_eq!(size, 96);
        assert_eq!(size % 16, 0, "FrameUniforms must be 16-byte aligned");
    }

    #[test]
    fn test_gpu_light_data_size() {
        let size = std::mem::size_of::<GpuLightData>();
        assert_eq!(size, 48); // 3+1+3+1+1+3 = 12 floats × 4 = 48
        assert_eq!(size % 16, 0, "GpuLightData must be 16-byte aligned");
    }

    #[test]
    fn test_draw_uniforms_size() {
        let size = std::mem::size_of::<DrawUniforms>();
        // 16×f32 (model) + 4×f32 (color) + 4×f32 (material) = 96
        assert_eq!(size % 16, 0, "DrawUniforms must be 16-byte aligned");
    }

    #[test]
    fn test_lit_shader_source_is_valid_wgsl() {
        assert!(LIT_SHADER_SRC.contains("fn vs_main"));
        assert!(LIT_SHADER_SRC.contains("fn fs_main"));
        assert!(LIT_SHADER_SRC.contains("FrameUniforms"));
        assert!(LIT_SHADER_SRC.contains("LightData"));
        assert!(LIT_SHADER_SRC.contains("DrawUniforms"));
    }

    #[test]
    fn test_draw_uniforms_new() {
        let model = Mat4::identity();
        let u = DrawUniforms::new(&model, 0xFFFF_0000, 32.0, 0.5);
        assert!((u.color[0] - 1.0).abs() < f32::EPSILON); // red
        assert!(u.color[1].abs() < f32::EPSILON); // no green
        assert!((u.shininess - 32.0).abs() < f32::EPSILON);
        assert!((u.specular_strength - 0.5).abs() < f32::EPSILON);
    }
}
