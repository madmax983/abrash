//! Deferred lighting pass: reads G-Buffer textures and computes lighting in screen space.
//!
//! This pass runs as a fullscreen triangle after the geometry pass has populated the
//! G-Buffer. It reads position, normal+roughness, and albedo+metallic from the G-Buffer
//! textures, then loops over all lights to compute Blinn-Phong shading with shadow mapping.

use crate::shader::{FrameUniforms, GpuLightData, MAX_LIGHTS};
use std::num::NonZeroU64;

/// WGSL shader for the deferred lighting pass.
///
/// Reads 3 G-Buffer textures + shadow map, outputs HDR lit color.
const DEFERRED_LIGHTING_SHADER: &str = r"
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

struct ShadowData {
    light_vp: mat4x4<f32>,
};

// Group 0: G-Buffer textures
@group(0) @binding(0) var gbuf_position: texture_2d<f32>;
@group(0) @binding(1) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(2) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(3) var gbuf_sampler: sampler;

// Group 1: Frame uniforms + lights
@group(1) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(1) var<uniform> lights: array<LightData, 8>;

// Group 2: Shadow map
// Group 2: Shadow map
@group(2) @binding(0) var shadow_depth: texture_depth_2d;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;
@group(2) @binding(2) var<uniform> shadow: ShadowData;

// Group 3: IBL (image-based lighting)
@group(3) @binding(0) var irradiance_map: texture_cube<f32>;
@group(3) @binding(1) var prefiltered_map: texture_cube<f32>;
@group(3) @binding(2) var brdf_lut: texture_2d<f32>;
@group(3) @binding(3) var ibl_sampler: sampler;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, -y * 0.5 + 0.5);
    return out;
}

fn compute_shadow(world_pos: vec3<f32>) -> f32 {
    let light_clip = shadow.light_vp * vec4<f32>(world_pos, 1.0);
    let ndc = light_clip.xyz / light_clip.w;
    let shadow_uv = vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 {
        return 1.0;
    }
    return textureSampleCompareLevel(shadow_depth, shadow_sampler, shadow_uv, ndc.z);
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let pos_sample = textureSample(gbuf_position, gbuf_sampler, input.uv);
    let normal_sample = textureSample(gbuf_normal, gbuf_sampler, input.uv);
    let albedo_sample = textureSample(gbuf_albedo, gbuf_sampler, input.uv);

    // Skip background pixels (position.w == 0 means no geometry was written)
    if pos_sample.w < 0.5 {
        discard;
    }

    let world_pos = pos_sample.xyz;
    let N = normalize(normal_sample.xyz);
    let roughness = normal_sample.w;
    let base_color = albedo_sample.rgb;
    let metallic = albedo_sample.a;

    let V = normalize(frame.camera_pos.xyz - world_pos);
    let NdotV = max(dot(N, V), 0.0);
    let shadow_val = compute_shadow(world_pos);

    // Fresnel at normal incidence: dielectric = 0.04, metal = base_color
    let F0 = mix(vec3(0.04), base_color, metallic);

    // ----- IBL ambient -----
    // Diffuse IBL: irradiance map provides pre-convolved cosine-weighted hemisphere
    // Minimum ambient ensures geometry is visible even without an environment map
    let irradiance = textureSample(irradiance_map, ibl_sampler, N).rgb;
    let min_ambient = vec3(0.03);
    let kD = (1.0 - metallic); // metals have no diffuse
    let ambient_diffuse = kD * base_color * max(irradiance, min_ambient);

    // Specular IBL: prefiltered env map (roughness → mip) + BRDF LUT (split-sum)
    let R = reflect(-V, N);
    let max_mip = 4.0; // PREFILTERED_MIP_LEVELS - 1
    let prefiltered = textureSampleLevel(prefiltered_map, ibl_sampler, R, roughness * max_mip).rgb;
    let brdf = textureSample(brdf_lut, ibl_sampler, vec2(NdotV, roughness)).rg;
    let ambient_specular = prefiltered * (F0 * brdf.x + brdf.y);

    var result = ambient_diffuse + ambient_specular;

    // ----- Direct lighting -----
    for (var i = 0u; i < frame.light_count; i = i + 1u) {
        let light = lights[i];

        var L: vec3<f32>;
        var attenuation: f32 = 1.0;
        var light_shadow: f32 = 1.0;

        if (light.light_type == 0u) {
            L = normalize(-light.position_or_direction);
            if (i == 0u) { light_shadow = shadow_val; }
        } else {
            let to_light = light.position_or_direction - world_pos;
            let dist = length(to_light);
            L = to_light / max(dist, 0.0001);
            let r = max(light.radius, 0.0001);
            attenuation = 1.0 / (1.0 + (dist * dist) / (r * r));
        }

        let NdotL = max(dot(N, L), 0.0);
        let diffuse = base_color * NdotL;

        // Specular: derive shininess from roughness
        let shininess = max(2.0 / max(roughness * roughness * roughness * roughness, 0.001) - 2.0, 1.0);
        let spec_strength = mix(0.04, 1.0, metallic);
        let H = normalize(L + V);
        let NdotH = max(dot(N, H), 0.0);
        let specular = vec3<f32>(spec_strength) * pow(NdotH, shininess);

        result = result + (diffuse + specular) * light.color * light.intensity * attenuation * light_shadow;
    }

    return vec4<f32>(result, 1.0);
}
";

/// Deferred lighting pass: fullscreen triangle reading G-Buffer + lights + IBL.
pub struct DeferredLightingPass {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) gbuffer_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) frame_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) ibl_bind_group_layout: wgpu::BindGroupLayout,
}

impl DeferredLightingPass {
    /// Create the deferred lighting pass.
    ///
    /// Output format should be the HDR format (`Rgba16Float`) since tone mapping
    /// happens in a separate pass.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        shadow_sample_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Deferred Lighting Shader"),
            source: wgpu::ShaderSource::Wgsl(DEFERRED_LIGHTING_SHADER.into()),
        });

        let gbuffer_bind_group_layout = Self::create_gbuffer_layout(device);
        let frame_bind_group_layout = Self::create_frame_layout(device);
        let ibl_bind_group_layout = Self::create_ibl_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Deferred Lighting Pipeline Layout"),
            bind_group_layouts: &[
                Some(&gbuffer_bind_group_layout),
                Some(&frame_bind_group_layout),
                Some(shadow_sample_layout),
                Some(&ibl_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Deferred Lighting Pipeline"),
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
                    format: output_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            gbuffer_bind_group_layout,
            frame_bind_group_layout,
            ibl_bind_group_layout,
        }
    }

    fn create_gbuffer_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Deferred GBuffer Layout"),
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        })
    }

    fn create_frame_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Deferred Frame Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
        })
    }

    fn create_ibl_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Deferred IBL Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Create a bind group for IBL textures.
    #[must_use]
    pub fn create_ibl_bind_group(
        &self,
        device: &wgpu::Device,
        ibl: &crate::ibl::IblTextures,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred IBL Bind Group"),
            layout: &self.ibl_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ibl.irradiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&ibl.prefiltered_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&ibl.brdf_lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Create a bind group for reading the G-Buffer textures.
    #[must_use]
    pub fn create_gbuffer_bind_group(
        &self,
        device: &wgpu::Device,
        gbuffer: &crate::gbuffer::GBuffer,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred GBuffer Bind Group"),
            layout: &self.gbuffer_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Create a bind group for frame uniforms + lights (reusing existing buffers).
    #[must_use]
    pub fn create_frame_bind_group(
        &self,
        device: &wgpu::Device,
        frame_buffer: &wgpu::Buffer,
        light_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred Frame Bind Group"),
            layout: &self.frame_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Encode the deferred lighting pass.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        gbuffer_bg: &wgpu::BindGroup,
        frame_bg: &wgpu::BindGroup,
        shadow_bg: &wgpu::BindGroup,
        ibl_bg: &wgpu::BindGroup,
        output_view: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Deferred Lighting Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
        pass.set_bind_group(0, gbuffer_bg, &[]);
        pass.set_bind_group(1, frame_bg, &[]);
        pass.set_bind_group(2, shadow_bg, &[]);
        pass.set_bind_group(3, ibl_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deferred_lighting_shader_valid() {
        assert!(DEFERRED_LIGHTING_SHADER.contains("fn vs_main"));
        assert!(DEFERRED_LIGHTING_SHADER.contains("fn fs_main"));
        assert!(DEFERRED_LIGHTING_SHADER.contains("gbuf_position"));
        assert!(DEFERRED_LIGHTING_SHADER.contains("gbuf_normal"));
        assert!(DEFERRED_LIGHTING_SHADER.contains("gbuf_albedo"));
        assert!(DEFERRED_LIGHTING_SHADER.contains("compute_shadow"));
        assert!(DEFERRED_LIGHTING_SHADER.contains("LightData"));
    }
}
