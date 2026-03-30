//! Image-Based Lighting (IBL) precomputation.
//!
//! Generates three GPU textures from an environment cubemap:
//! 1. **Irradiance map** — diffuse ambient (cosine-weighted hemisphere average)
//! 2. **Prefiltered environment map** — specular ambient (GGX-filtered per roughness mip)
//! 3. **BRDF integration LUT** — split-sum lookup (`NdotV` × roughness → scale + bias)

use wgpu::util::DeviceExt;

/// Irradiance map resolution (per face).
const IRRADIANCE_SIZE: u32 = 32;
/// Prefiltered environment map resolution (per face, mip 0).
const PREFILTERED_SIZE: u32 = 128;
/// Number of roughness mip levels for the prefiltered map.
const PREFILTERED_MIP_LEVELS: u32 = 5;
/// BRDF LUT resolution.
const BRDF_LUT_SIZE: u32 = 256;

// ---------------------------------------------------------------------------
// Compute shaders
// ---------------------------------------------------------------------------

const IRRADIANCE_SHADER: &str = r"
@group(0) @binding(0) var env_map: texture_cube<f32>;
@group(0) @binding(1) var env_sampler: sampler;
@group(0) @binding(2) var output: texture_storage_2d_array<rgba16float, write>;

const PI: f32 = 3.14159265359;

// Convert face + UV to cubemap direction
fn face_uv_to_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let u = uv.x * 2.0 - 1.0;
    let v = uv.y * 2.0 - 1.0;
    switch face {
        case 0u: { return normalize(vec3( 1.0,   -v,   -u)); } // +X
        case 1u: { return normalize(vec3(-1.0,   -v,    u)); } // -X
        case 2u: { return normalize(vec3(   u,  1.0,    v)); } // +Y
        case 3u: { return normalize(vec3(   u, -1.0,   -v)); } // -Y
        case 4u: { return normalize(vec3(   u,   -v,  1.0)); } // +Z
        default: { return normalize(vec3(  -u,   -v, -1.0)); } // -Z
    }
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(output).xy;
    if gid.x >= size.x || gid.y >= size.y { return; }
    let face = gid.z;
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(size);
    let N = face_uv_to_dir(face, uv);

    // Build tangent frame
    var up = vec3(0.0, 1.0, 0.0);
    if abs(N.y) > 0.999 { up = vec3(0.0, 0.0, 1.0); }
    let right = normalize(cross(up, N));
    let up2 = cross(N, right);

    // Cosine-weighted hemisphere sampling (uniform grid approximation)
    var irradiance = vec3(0.0);
    let samples = 64u;
    let inv_samples = 1.0 / f32(samples);
    for (var i = 0u; i < samples; i = i + 1u) {
        for (var j = 0u; j < samples; j = j + 1u) {
            let phi = 2.0 * PI * (f32(i) + 0.5) * inv_samples;
            let cos_theta = (f32(j) + 0.5) * inv_samples;
            let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

            // Spherical to cartesian (tangent space)
            let t = vec3(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
            // Tangent to world
            let sample_dir = t.x * right + t.y * up2 + t.z * N;

            let color = textureSampleLevel(env_map, env_sampler, sample_dir, 0.0).rgb;
            irradiance = irradiance + color * cos_theta * sin_theta;
        }
    }
    irradiance = PI * irradiance / f32(samples * samples);

    textureStore(output, gid.xy, i32(face), vec4(irradiance, 1.0));
}
";

const PREFILTER_SHADER: &str = r"
@group(0) @binding(0) var env_map: texture_cube<f32>;
@group(0) @binding(1) var env_sampler: sampler;
@group(0) @binding(2) var output: texture_storage_2d_array<rgba16float, write>;

struct PrefilterParams {
    roughness: f32,
    _pad: vec3<f32>,
};
@group(0) @binding(3) var<uniform> params: PrefilterParams;

const PI: f32 = 3.14159265359;

fn face_uv_to_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let u = uv.x * 2.0 - 1.0;
    let v = uv.y * 2.0 - 1.0;
    switch face {
        case 0u: { return normalize(vec3( 1.0,   -v,   -u)); }
        case 1u: { return normalize(vec3(-1.0,   -v,    u)); }
        case 2u: { return normalize(vec3(   u,  1.0,    v)); }
        case 3u: { return normalize(vec3(   u, -1.0,   -v)); }
        case 4u: { return normalize(vec3(   u,   -v,  1.0)); }
        default: { return normalize(vec3(  -u,   -v, -1.0)); }
    }
}

// Radical inverse (Van der Corput sequence)
fn radical_inverse(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10; // / 0x100000000
}

fn hammersley(i: u32, n: u32) -> vec2<f32> {
    return vec2(f32(i) / f32(n), radical_inverse(i));
}

// GGX importance sampling
fn importance_sample_ggx(xi: vec2<f32>, N: vec3<f32>, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    let H = vec3(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);

    var up = vec3(0.0, 1.0, 0.0);
    if abs(N.y) > 0.999 { up = vec3(0.0, 0.0, 1.0); }
    let tangent = normalize(cross(up, N));
    let bitangent = cross(N, tangent);

    return normalize(tangent * H.x + bitangent * H.y + N * H.z);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(output).xy;
    if gid.x >= size.x || gid.y >= size.y { return; }
    let face = gid.z;
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(size);
    let N = face_uv_to_dir(face, uv);
    let R = N;
    let V = R;

    let sample_count = 256u;
    var color = vec3(0.0);
    var weight = 0.0;

    for (var i = 0u; i < sample_count; i = i + 1u) {
        let xi = hammersley(i, sample_count);
        let H = importance_sample_ggx(xi, N, params.roughness);
        let L = normalize(2.0 * dot(V, H) * H - V);
        let NdotL = max(dot(N, L), 0.0);
        if NdotL > 0.0 {
            color = color + textureSampleLevel(env_map, env_sampler, L, 0.0).rgb * NdotL;
            weight = weight + NdotL;
        }
    }
    color = color / max(weight, 0.001);

    textureStore(output, gid.xy, i32(face), vec4(color, 1.0));
}
";

const BRDF_LUT_SHADER: &str = r"
@group(0) @binding(0) var output: texture_storage_2d<rg16float, write>;

const PI: f32 = 3.14159265359;

fn radical_inverse(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(i: u32, n: u32) -> vec2<f32> {
    return vec2(f32(i) / f32(n), radical_inverse(i));
}

fn importance_sample_ggx(xi: vec2<f32>, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    return vec3(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let a = roughness;
    let k = (a * a) / 2.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

fn geometry_smith(NdotV: f32, NdotL: f32, roughness: f32) -> f32 {
    return geometry_schlick_ggx(NdotV, roughness) * geometry_schlick_ggx(NdotL, roughness);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(output);
    if gid.x >= size.x || gid.y >= size.y { return; }

    let NdotV = max((f32(gid.x) + 0.5) / f32(size.x), 0.001);
    let roughness = max((f32(gid.y) + 0.5) / f32(size.y), 0.001);

    let V = vec3(sqrt(1.0 - NdotV * NdotV), 0.0, NdotV);
    let N = vec3(0.0, 0.0, 1.0);

    var A = 0.0; // scale
    var B = 0.0; // bias

    let sample_count = 1024u;
    for (var i = 0u; i < sample_count; i = i + 1u) {
        let xi = hammersley(i, sample_count);
        let H = importance_sample_ggx(xi, roughness);
        let L = normalize(2.0 * dot(V, H) * H - V);

        let NdotL = max(L.z, 0.0);
        let NdotH = max(H.z, 0.0);
        let VdotH = max(dot(V, H), 0.0);

        if NdotL > 0.0 {
            let G = geometry_smith(NdotV, NdotL, roughness);
            let G_vis = (G * VdotH) / (NdotH * NdotV);
            let Fc = pow(1.0 - VdotH, 5.0);
            A = A + (1.0 - Fc) * G_vis;
            B = B + Fc * G_vis;
        }
    }
    A = A / f32(sample_count);
    B = B / f32(sample_count);

    textureStore(output, gid.xy, vec4(A, B, 0.0, 1.0));
}
";

/// Prefilter params uniform for the prefiltered env map compute shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PrefilterParams {
    roughness: f32,
    _pad: [f32; 3],
}

/// Precomputed IBL textures ready for sampling in the deferred lighting pass.
pub struct IblTextures {
    /// Irradiance cubemap (diffuse ambient).
    pub irradiance_view: wgpu::TextureView,
    /// Prefiltered environment cubemap with roughness mips (specular ambient).
    pub prefiltered_view: wgpu::TextureView,
    /// BRDF integration LUT (`NdotV` × roughness → scale + bias).
    pub brdf_lut_view: wgpu::TextureView,

    // Keep textures alive
    _irradiance: wgpu::Texture,
    _prefiltered: wgpu::Texture,
    _brdf_lut: wgpu::Texture,
}

impl IblTextures {
    /// Run IBL precomputation on the GPU from an environment cubemap.
    ///
    /// Generates irradiance map, prefiltered env map (5 mip levels), and BRDF LUT.
    #[must_use]
    pub fn precompute(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        env_view: &wgpu::TextureView,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("IBL Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 1. Irradiance convolution
        let irradiance_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("IBL Irradiance"),
            size: wgpu::Extent3d {
                width: IRRADIANCE_SIZE,
                height: IRRADIANCE_SIZE,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let irradiance_storage_view =
            irradiance_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let irradiance_cube_view = irradiance_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        Self::dispatch_irradiance(device, queue, env_view, &sampler, &irradiance_storage_view);

        // 2. Prefiltered environment map (mip per roughness)
        let prefiltered_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("IBL Prefiltered"),
            size: wgpu::Extent3d {
                width: PREFILTERED_SIZE,
                height: PREFILTERED_SIZE,
                depth_or_array_layers: 6,
            },
            mip_level_count: PREFILTERED_MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let prefiltered_cube_view = prefiltered_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        Self::dispatch_prefilter(device, queue, env_view, &sampler, &prefiltered_tex);

        // 3. BRDF LUT
        let brdf_lut_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("IBL BRDF LUT"),
            size: wgpu::Extent3d {
                width: BRDF_LUT_SIZE,
                height: BRDF_LUT_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let brdf_lut_view = brdf_lut_tex.create_view(&wgpu::TextureViewDescriptor::default());

        Self::dispatch_brdf_lut(device, queue, &brdf_lut_view);

        Self {
            irradiance_view: irradiance_cube_view,
            prefiltered_view: prefiltered_cube_view,
            brdf_lut_view,
            _irradiance: irradiance_tex,
            _prefiltered: prefiltered_tex,
            _brdf_lut: brdf_lut_tex,
        }
    }

    /// Create default black IBL textures (no environment — pure black ambient).
    ///
    /// Used as a fallback when no environment cubemap has been set.
    /// Creates tiny 1×1 textures with zeros — no compute shaders needed.
    #[must_use]
    pub fn default_black(device: &wgpu::Device) -> Self {
        let make_cube = |label| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 6,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
            (tex, view)
        };

        let (irr_tex, irr_view) = make_cube("Default Black Irradiance");
        let (pf_tex, pf_view) = make_cube("Default Black Prefiltered");

        let brdf_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default BRDF LUT"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let brdf_view = brdf_tex.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            irradiance_view: irr_view,
            prefiltered_view: pf_view,
            brdf_lut_view: brdf_view,
            _irradiance: irr_tex,
            _prefiltered: pf_tex,
            _brdf_lut: brdf_tex,
        }
    }

    fn dispatch_irradiance(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        env_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        output_view: &wgpu::TextureView,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("IBL Irradiance Compute"),
            source: wgpu::ShaderSource::Wgsl(IRRADIANCE_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("IBL Irradiance Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("IBL Irradiance BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(env_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(output_view),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("IBL Irradiance Pipeline Layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("IBL Irradiance Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("IBL Irradiance Encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("IBL Irradiance Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            // Dispatch 6 faces, each IRRADIANCE_SIZE × IRRADIANCE_SIZE
            pass.dispatch_workgroups(IRRADIANCE_SIZE.div_ceil(8), IRRADIANCE_SIZE.div_ceil(8), 6);
        }
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
    }

    fn dispatch_prefilter(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        env_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        output_tex: &wgpu::Texture,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("IBL Prefilter Compute"),
            source: wgpu::ShaderSource::Wgsl(PREFILTER_SHADER.into()),
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("IBL Prefilter Params"),
            contents: bytemuck::bytes_of(&PrefilterParams {
                roughness: 0.0,
                _pad: [0.0; 3],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = Self::create_prefilter_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("IBL Prefilter Pipeline Layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("IBL Prefilter Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Dispatch per mip level
        for mip in 0..PREFILTERED_MIP_LEVELS {
            let mip_size = PREFILTERED_SIZE >> mip;
            let roughness = mip as f32 / (PREFILTERED_MIP_LEVELS - 1) as f32;

            queue.write_buffer(
                &params_buffer,
                0,
                bytemuck::bytes_of(&PrefilterParams {
                    roughness,
                    _pad: [0.0; 3],
                }),
            );

            let mip_view = output_tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some("IBL Prefilter Mip View"),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            });

            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("IBL Prefilter BG"),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(env_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&mip_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IBL Prefilter Encoder"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("IBL Prefilter Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(mip_size.div_ceil(8), mip_size.div_ceil(8), 6);
            }
            queue.submit(Some(encoder.finish()));
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }
    }

    fn create_prefilter_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("IBL Prefilter Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            PrefilterParams,
                        >()
                            as u64),
                    },
                    count: None,
                },
            ],
        })
    }

    fn dispatch_brdf_lut(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("IBL BRDF LUT Compute"),
            source: wgpu::ShaderSource::Wgsl(BRDF_LUT_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("IBL BRDF LUT Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rg16Float,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            }],
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("IBL BRDF LUT BG"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(output_view),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("IBL BRDF LUT Pipeline Layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("IBL BRDF LUT Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("IBL BRDF LUT Encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("IBL BRDF LUT Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(BRDF_LUT_SIZE.div_ceil(8), BRDF_LUT_SIZE.div_ceil(8), 1);
        }
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irradiance_shader_valid() {
        assert!(IRRADIANCE_SHADER.contains("@compute"));
        assert!(IRRADIANCE_SHADER.contains("texture_cube"));
        assert!(IRRADIANCE_SHADER.contains("textureStore"));
    }

    #[test]
    fn test_prefilter_shader_valid() {
        assert!(PREFILTER_SHADER.contains("@compute"));
        assert!(PREFILTER_SHADER.contains("importance_sample_ggx"));
        assert!(PREFILTER_SHADER.contains("roughness"));
    }

    #[test]
    fn test_brdf_lut_shader_valid() {
        assert!(BRDF_LUT_SHADER.contains("@compute"));
        assert!(BRDF_LUT_SHADER.contains("geometry_smith"));
        assert!(BRDF_LUT_SHADER.contains("hammersley"));
    }

    #[test]
    fn test_prefilter_params_size() {
        assert_eq!(std::mem::size_of::<PrefilterParams>(), 16);
    }
}
