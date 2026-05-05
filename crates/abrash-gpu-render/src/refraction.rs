//! Screen-space refraction via Newton's method.
//!
//! Implements "Ultrafast Screen-Space Refractions and Caustics via Newton's Method"
//! (JCGT Vol. 15, No. 1, 2026).
//!
//! # Pipeline
//!
//! 1. **Refraction surface pass** — renders refractive objects (glass, water) to a
//!    slim two-target G-buffer storing world position and world normal + IOR ratio.
//! 2. **Newton resolve pass** — fullscreen triangle pass that, for each refractive
//!    pixel, iteratively intersects the Snell-refracted ray with tangent planes
//!    sampled from the opaque G-buffer until convergence (≤ 1 screen-space pixel of
//!    error).  Non-refractive pixels pass through the scene colour unchanged.
//!
//! # Convergence
//!
//! Each Newton step:
//! ```text
//! s_{n+1} = dot(n, p_n - W) / dot(n, r̂)
//! q_{n+1} = W + s_{n+1} * r̂
//! ```
//! where W is the refractive surface world position, r̂ is the unit refracted
//! direction, and (`p_n`, n) are the world position and normal sampled from the opaque
//! G-buffer at the screen-space projection of the current estimate.  Convergence is
//! declared when the screen-space pixel error between q_{n+1} and the re-sampled
//! G-buffer point p_{n+1} is below 1 pixel.

use crate::gbuffer::{DEPTH_FORMAT, NORMAL_FORMAT, POSITION_FORMAT};
use crate::shader::LitVertex;
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

/// WGSL shader that renders refractive geometry to the refraction surface buffer.
///
/// Outputs two render targets:
/// - `position` (Rgba16Float): world XYZ + w=1.0 geometry marker
/// - `normal_ior` (Rgba16Float): world normal XYZ + IOR ratio n1/n2 in w
///
/// The IOR ratio is read from `draw.shininess` (repurposed for refractive materials).
pub const REFRACTION_SURFACE_SHADER: &str = r"
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
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

struct SurfaceOutput {
    @location(0) position: vec4<f32>,
    @location(1) normal_ior: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let wp = draw.model * vec4<f32>(in.position, 1.0);
    out.clip_pos = frame.view_proj * wp;
    out.world_pos = wp.xyz;
    out.world_normal = (draw.model * vec4<f32>(in.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> SurfaceOutput {
    var out: SurfaceOutput;
    let N = normalize(in.world_normal);
    out.position = vec4<f32>(in.world_pos, 1.0);
    out.normal_ior = vec4<f32>(N, draw.shininess);
    return out;
}
";

/// WGSL shader implementing Newton's method screen-space refraction.
///
/// For each refractive pixel the shader:
/// 1. Reads surface position W and normal N from the refraction surface buffer.
/// 2. Computes the refracted ray direction r̂ via Snell's law.
/// 3. Iterates Newton steps against the opaque G-buffer until converged.
/// 4. Samples the scene colour at the converged screen-space UV.
///    Non-refractive pixels pass through unchanged.
pub const REFRACTION_RESOLVE_SHADER: &str = r"
// Newton's method screen-space refraction resolve pass.
// Reference: 'Ultrafast Screen-Space Refractions and Caustics via Newton's
// Method', JCGT Vol. 15, No. 1, 2026.

struct RefractionParams {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    screen_size: vec2<f32>,
    _ior_fallback: f32,
    max_iterations: u32,
};

// Group 0: refraction surface G-buffer
@group(0) @binding(0) var refr_pos:  texture_2d<f32>;
@group(0) @binding(1) var refr_norm: texture_2d<f32>;

// Group 1: opaque scene G-buffer (background geometry)
@group(1) @binding(0) var opaque_pos:  texture_2d<f32>;
@group(1) @binding(1) var opaque_norm: texture_2d<f32>;

// Group 2: scene colour (HDR / post-TAA)
@group(2) @binding(0) var scene_color: texture_2d<f32>;
@group(2) @binding(1) var scene_sampler: sampler;

// Group 3: parameters
@group(3) @binding(0) var<uniform> params: RefractionParams;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(idx & 1u) * 4 - 1);
    let y = f32(i32(idx >> 1u) * 4 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, -y * 0.5 + 0.5);
    return out;
}

fn uv_to_px(uv: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(
        i32(uv.x * params.screen_size.x),
        i32(uv.y * params.screen_size.y),
    );
}

fn world_to_uv(wp: vec3<f32>) -> vec2<f32> {
    let clip = params.view_proj * vec4<f32>(wp, 1.0);
    if clip.w <= 0.0 { return vec2<f32>(-1.0, -1.0); }
    let ndc = clip.xyz / clip.w;
    return vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
}

fn in_screen(uv: vec2<f32>) -> bool {
    return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let px = uv_to_px(in.uv);

    // ── Refraction surface ────────────────────────────────────────────────────
    let surf_p = textureLoad(refr_pos, px, 0);
    if surf_p.w < 0.5 {
        return textureSample(scene_color, scene_sampler, in.uv);
    }

    let W   = surf_p.xyz;
    let sn  = textureLoad(refr_norm, px, 0);
    let N_s = normalize(sn.xyz);
    let eta = select(0.667, sn.w, sn.w > 0.01);

    // Visibility: skip if opaque geometry is in front of the refractive surface
    let opaque_at_px = textureLoad(opaque_pos, px, 0);
    if opaque_at_px.w >= 0.5 {
        let clip_surf   = params.view_proj * vec4<f32>(W, 1.0);
        let clip_opaque = params.view_proj * vec4<f32>(opaque_at_px.xyz, 1.0);
        let d_surf   = clip_surf.z   / clip_surf.w;
        let d_opaque = clip_opaque.z / clip_opaque.w;
        if d_surf >= d_opaque {
            return textureSample(scene_color, scene_sampler, in.uv);
        }
    }

    // ── Refracted ray (Snell's law) ────────────────────────────────────────────
    let V = normalize(W - params.camera_pos.xyz);
    let r = refract(V, N_s, eta);
    if dot(r, r) < 0.01 {
        return textureSample(scene_color, scene_sampler, in.uv);
    }
    let r_n = normalize(r);

    // ── Newton iterations ──────────────────────────────────────────────────────
    var current_uv = in.uv;
    var final_uv   = in.uv;

    for (var i = 0u; i < params.max_iterations; i++) {
        // Sample opaque G-buffer at current UV estimate
        let opq_px = uv_to_px(current_uv);
        let p_samp = textureLoad(opaque_pos, opq_px, 0);
        if p_samp.w < 0.5 { break; }
        let p = p_samp.xyz;

        let n_samp = textureLoad(opaque_norm, opq_px, 0);
        let n = normalize(n_samp.xyz);

        // Newton step: intersect refracted ray with tangent plane at p
        // s = dot(n, p - W) / dot(n, r_n)
        let denom = dot(n, r_n);
        if abs(denom) < 0.0001 { break; }

        let s = dot(n, p - W) / denom;
        if s < 0.0001 { break; }

        let q = W + s * r_n;

        // Project q to screen UV
        let q_uv = world_to_uv(q);
        if !in_screen(q_uv) { break; }

        final_uv = q_uv;

        // Convergence check: pixel error between q and re-sampled G-buffer point
        let q_px     = uv_to_px(q_uv);
        let p_new    = textureLoad(opaque_pos, q_px, 0);
        if p_new.w >= 0.5 {
            let p_new_uv  = world_to_uv(p_new.xyz);
            let pixel_err = length((q_uv - p_new_uv) * params.screen_size);
            if pixel_err < 1.0 {
                final_uv = q_uv;
                break;
            }
        }

        current_uv = q_uv;
    }

    return textureSample(scene_color, scene_sampler, final_uv);
}
";

// ---------------------------------------------------------------------------
// Refraction surface G-buffer
// ---------------------------------------------------------------------------

/// Slim G-buffer storing the front-face geometry of refractive objects.
pub struct RefractionSurface {
    pub(crate) _position_texture: wgpu::Texture,
    pub(crate) position_view: wgpu::TextureView,
    pub(crate) _normal_texture: wgpu::Texture,
    pub(crate) normal_view: wgpu::TextureView,
    pub(crate) _depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl RefractionSurface {
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let make = |label, format, usage: wgpu::TextureUsages| {
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

        let (pos_tex, position_view) = make("Refr Surface Position", POSITION_FORMAT, color_usage);
        let (norm_tex, normal_view) = make("Refr Surface Normal", NORMAL_FORMAT, color_usage);
        let (depth_tex, depth_view) = make("Refr Surface Depth", DEPTH_FORMAT, depth_usage);

        Self {
            _position_texture: pos_tex,
            position_view,
            _normal_texture: norm_tex,
            normal_view,
            _depth_texture: depth_tex,
            depth_view,
            width,
            height,
        }
    }
}

// ---------------------------------------------------------------------------
// Refraction surface pipeline
// ---------------------------------------------------------------------------

/// Pipeline that renders refractive geometry to the [`RefractionSurface`] buffer.
pub struct RefractionSurfacePipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
}

impl RefractionSurfacePipeline {
    /// Create the pipeline, sharing bind group layouts with the main G-buffer pipeline.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        draw_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Refraction Surface Shader"),
            source: wgpu::ShaderSource::Wgsl(REFRACTION_SURFACE_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Refraction Surface Pipeline Layout"),
            bind_group_layouts: &[Some(frame_layout), Some(draw_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Refraction Surface Pipeline"),
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
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        Self { pipeline }
    }
}

// ---------------------------------------------------------------------------
// Refraction resolve output texture
// ---------------------------------------------------------------------------

/// `Rgba16Float` texture that the Newton resolve pass writes its output into.
pub struct RefractionOutput {
    pub(crate) texture: wgpu::Texture,
    pub(crate) color_view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl RefractionOutput {
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Refraction Output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            color_view,
            width,
            height,
        }
    }
}

// ---------------------------------------------------------------------------
// Refraction resolve pass (Newton's method)
// ---------------------------------------------------------------------------

/// Per-frame uniform buffer for the Newton resolve pass.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct RefractionParams {
    pub view_proj: [f32; 16],
    pub camera_pos: [f32; 4],
    pub screen_size: [f32; 2],
    pub ior_fallback: f32,
    pub max_iterations: u32,
}

/// Fullscreen-triangle pass that resolves screen-space refractions via Newton's method.
pub struct RefractionResolvePass {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) surface_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) opaque_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) scene_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_buffer: wgpu::Buffer,
    pub(crate) linear_sampler: wgpu::Sampler,
}

impl RefractionResolvePass {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Refraction Resolve Shader"),
            source: wgpu::ShaderSource::Wgsl(REFRACTION_RESOLVE_SHADER.into()),
        });

        let surface_bind_group_layout =
            Self::two_nonfilterable_tex_layout(device, "Refraction Surface Layout");
        let opaque_bind_group_layout =
            Self::two_nonfilterable_tex_layout(device, "Refraction Opaque GBuf Layout");
        let scene_bind_group_layout = Self::color_tex_layout(device);
        let params_bind_group_layout = Self::params_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Refraction Resolve Pipeline Layout"),
            bind_group_layouts: &[
                Some(&surface_bind_group_layout),
                Some(&opaque_bind_group_layout),
                Some(&scene_bind_group_layout),
                Some(&params_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Refraction Resolve Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Refraction Params Buffer"),
            contents: &[0u8; std::mem::size_of::<RefractionParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Refraction Linear Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            pipeline,
            surface_bind_group_layout,
            opaque_bind_group_layout,
            scene_bind_group_layout,
            params_bind_group_layout,
            params_buffer,
            linear_sampler,
        }
    }

    /// Encode the resolve pass.
    ///
    /// Reads from `surface`, `opaque_gbuffer`, and `scene_view` (scene colour
    /// after deferred lighting + TAA), writes refracted output to `output_view`.
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface: &RefractionSurface,
        opaque_position_view: &wgpu::TextureView,
        opaque_normal_view: &wgpu::TextureView,
        scene_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        params: RefractionParams,
    ) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        let surface_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Refraction Surface BG"),
            layout: &self.surface_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&surface.position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&surface.normal_view),
                },
            ],
        });

        let opaque_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Refraction Opaque BG"),
            layout: &self.opaque_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(opaque_position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(opaque_normal_view),
                },
            ],
        });

        let scene_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Refraction Scene BG"),
            layout: &self.scene_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });

        let params_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Refraction Params BG"),
            layout: &self.params_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.params_buffer.as_entire_binding(),
            }],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Refraction Resolve Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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
        pass.set_bind_group(0, &surface_bg, &[]);
        pass.set_bind_group(1, &opaque_bg, &[]);
        pass.set_bind_group(2, &scene_bg, &[]);
        pass.set_bind_group(3, &params_bg, &[]);
        pass.draw(0..3, 0..1);
    }

    // ── Bind group layout helpers ────────────────────────────────────────────

    fn two_nonfilterable_tex_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        })
    }

    fn color_tex_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Refraction Scene Color Layout"),
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
        })
    }

    fn params_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Refraction Params Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        std::mem::size_of::<RefractionParams>() as u64
                    ),
                },
                count: None,
            }],
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refraction_surface_shader_has_entry_points() {
        assert!(REFRACTION_SURFACE_SHADER.contains("fn vs_main"));
        assert!(REFRACTION_SURFACE_SHADER.contains("fn fs_main"));
        assert!(REFRACTION_SURFACE_SHADER.contains("SurfaceOutput"));
        assert!(REFRACTION_SURFACE_SHADER.contains("normal_ior"));
    }

    #[test]
    fn refraction_resolve_shader_has_entry_points() {
        assert!(REFRACTION_RESOLVE_SHADER.contains("fn vs_main"));
        assert!(REFRACTION_RESOLVE_SHADER.contains("fn fs_main"));
        assert!(REFRACTION_RESOLVE_SHADER.contains("refract("));
        assert!(REFRACTION_RESOLVE_SHADER.contains("max_iterations"));
    }

    #[test]
    fn refraction_params_is_pod_and_aligned() {
        let size = std::mem::size_of::<RefractionParams>();
        assert_eq!(size % 16, 0, "RefractionParams must be 16-byte aligned");
    }
}
