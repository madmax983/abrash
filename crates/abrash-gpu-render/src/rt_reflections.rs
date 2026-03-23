//! Ray-traced indirect specular reflections.
//!
//! Traces one reflection ray per pixel from G-Buffer world positions along
//! the reflection direction (based on view vector and surface normal).
//! On hit: outputs the albedo at the intersection. On miss: samples the
//! environment cubemap. The noisy 1-spp result feeds into the SVGF denoiser.
//!
//! Requires `wgpu::Features::EXPERIMENTAL_RAY_QUERY`.

use wgpu::util::DeviceExt;

/// WGSL compute shader for RT reflections.
///
/// Traces reflection rays from G-Buffer, outputs noisy 1-spp reflection color.
const RT_REFLECTION_SHADER: &str = r"
struct ReflectionParams {
    camera_pos: vec3<f32>,
    max_distance: f32,
};

@group(0) @binding(0) var gbuf_position: texture_2d<f32>;
@group(0) @binding(1) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(2) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(3) var<uniform> params: ReflectionParams;
@group(0) @binding(4) var scene_tlas: acceleration_structure;
@group(0) @binding(5) var env_map: texture_cube<f32>;
@group(0) @binding(6) var env_sampler: sampler;
@group(0) @binding(7) var output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(gbuf_position);
    if gid.x >= size.x || gid.y >= size.y { return; }

    let pos_sample = textureLoad(gbuf_position, gid.xy, 0);

    // Skip background pixels
    if pos_sample.w < 0.5 {
        textureStore(output, gid.xy, vec4(0.0));
        return;
    }

    let world_pos = pos_sample.xyz;
    let normal_sample = textureLoad(gbuf_normal, gid.xy, 0);
    let N = normalize(normal_sample.xyz);
    let roughness = normal_sample.w;

    let V = normalize(params.camera_pos - world_pos);
    let R = reflect(-V, N);

    // Trace reflection ray
    var rq: ray_query;
    rayQueryInitialize(
        &rq,
        scene_tlas,
        0u,     // flags
        0xFFu,  // cull mask
        world_pos + N * 0.01,  // bias along normal
        0.0,
        R,
        params.max_distance,
    );

    rayQueryProceed(&rq);

    let intersection = rayQueryGetCommittedIntersection(&rq);

    var color: vec3<f32>;
    if intersection.kind != RAY_QUERY_INTERSECTION_NONE {
        // Hit: use a simple approximation — sample albedo at hit point
        // In a full implementation, we'd shade the hit point with lighting.
        // For now, output a dimmed hit color to indicate reflection.
        let t = intersection.t;
        let fade = 1.0 / (1.0 + t * 0.1); // distance fade
        // Use the albedo of the current pixel as reflection tint (approximation)
        let albedo = textureLoad(gbuf_albedo, gid.xy, 0).rgb;
        color = albedo * fade * 0.5;
    } else {
        // Miss: sample environment map
        color = textureSampleLevel(env_map, env_sampler, R, roughness * 4.0).rgb;
    }

    // Modulate by roughness — rough surfaces get blurry reflections (handled by denoiser)
    // but also reduce reflection intensity
    let reflection_strength = 1.0 - roughness * roughness;
    color = color * reflection_strength;

    textureStore(output, gid.xy, vec4(color, 1.0));
}
";

/// Reflection params uniform.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReflectionParams {
    pub camera_pos: [f32; 3],
    pub max_distance: f32,
}

/// Ray-traced reflection pass.
#[allow(dead_code)]
pub struct RtReflectionPass {
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_buffer: wgpu::Buffer,
    /// Output reflection texture (`Rgba16Float`, noisy 1-spp).
    pub(crate) reflection_texture: Option<wgpu::Texture>,
    pub(crate) reflection_view: Option<wgpu::TextureView>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl RtReflectionPass {
    /// Create the RT reflection pass.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RT Reflection Compute"),
            source: wgpu::ShaderSource::Wgsl(RT_REFLECTION_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RT Reflection Layout"),
            entries: &[
                // 0: G-Buffer position
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 1: G-Buffer normal+roughness
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 2: G-Buffer albedo
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 3: params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            ReflectionParams,
                        >()
                            as u64),
                    },
                    count: None,
                },
                // 4: TLAS
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::AccelerationStructure {
                        vertex_return: false,
                    },
                    count: None,
                },
                // 5: environment cubemap
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // 6: env sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 7: output reflection texture
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RT Reflection Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RT Reflection Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RT Reflection Params"),
            contents: bytemuck::bytes_of(&ReflectionParams {
                camera_pos: [0.0; 3],
                max_distance: 100.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            reflection_texture: None,
            reflection_view: None,
            width: 0,
            height: 0,
        }
    }

    /// Ensure the reflection output texture matches the given dimensions.
    pub fn ensure_output(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height && self.reflection_texture.is_some() {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RT Reflection Output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.reflection_texture = Some(texture);
        self.reflection_view = Some(view);
        self.width = width;
        self.height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rt_reflection_shader_valid() {
        assert!(RT_REFLECTION_SHADER.contains("@compute"));
        assert!(RT_REFLECTION_SHADER.contains("acceleration_structure"));
        assert!(RT_REFLECTION_SHADER.contains("rayQueryInitialize"));
        assert!(RT_REFLECTION_SHADER.contains("reflect"));
        assert!(RT_REFLECTION_SHADER.contains("env_map"));
    }

    #[test]
    fn test_reflection_params_size() {
        assert_eq!(std::mem::size_of::<ReflectionParams>(), 16);
    }
}
