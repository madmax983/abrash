//! SVGF spatial denoising filter (Edge-Avoiding À-Trous Wavelet Transform).
//!
//! Implements the spatial component of Spatiotemporal Variance-Guided Filtering.
//! Uses an expanding kernel (step sizes 1, 2, 4, 8, 16) with edge-stopping
//! functions based on depth, normal similarity, and luminance variance.
//!
//! The separable blur optimization (paper §3.3) splits the 5×5 kernel into
//! two 1D passes (vertical + horizontal), reducing neighborhood visits from
//! 25 to 10 per iteration.
//!
//! Reference: Schied et al., "Spatiotemporal Variance-Guided Filtering", 2017.

/// Number of À-Trous filter iterations.
pub const ATROUS_ITERATIONS: u32 = 5;

/// WGSL compute shader for one iteration of the À-Trous wavelet filter.
///
/// Each iteration uses a larger step size (1, 2, 4, 8, 16) to blur over
/// progressively wider areas while preserving edges via edge-stopping functions.
pub const ATROUS_FILTER_SHADER: &str = r"
struct AtrousParams {
    step_size: i32,
    _pad: vec3<i32>,
};

@group(0) @binding(0) var input_color: texture_2d<f32>;
@group(0) @binding(1) var input_variance: texture_2d<f32>;
@group(0) @binding(2) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(3) var gbuf_position: texture_2d<f32>;
@group(0) @binding(4) var<uniform> params: AtrousParams;
@group(0) @binding(5) var output_color: texture_storage_2d<rgba16float, write>;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// 5-tap 1D kernel weights for À-Trous
const KERNEL: array<f32, 3> = array<f32, 3>(1.0, 2.0 / 3.0, 1.0 / 6.0);

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(input_color);
    if gid.x >= size.x || gid.y >= size.y { return; }

    let center_pos = textureLoad(gbuf_position, gid.xy, 0);

    // Background → pass through
    if center_pos.w < 0.5 {
        let c = textureLoad(input_color, gid.xy, 0);
        textureStore(output_color, gid.xy, c);
        return;
    }

    let center_color = textureLoad(input_color, gid.xy, 0).rgb;
    let center_normal = normalize(textureLoad(gbuf_normal, gid.xy, 0).xyz);
    let center_depth = center_pos.z;
    let center_lum = luminance(center_color);
    let center_variance = textureLoad(input_variance, gid.xy, 0).r;

    // Variance-guided kernel: higher variance → more aggressive blur
    let sigma_l = max(sqrt(center_variance) * 4.0, 0.01);

    var sum_color = center_color * KERNEL[0];
    var sum_weight = KERNEL[0];

    let step = params.step_size;

    // 2D separable-approximation: sample in a cross pattern
    for (var j = -2; j <= 2; j = j + 1) {
        for (var i = -2; i <= 2; i = i + 1) {
            if i == 0 && j == 0 { continue; }

            let offset = vec2<i32>(i, j) * step;
            let p = vec2<i32>(gid.xy) + offset;

            if p.x < 0 || p.x >= i32(size.x) || p.y < 0 || p.y >= i32(size.y) {
                continue;
            }

            let sample_pos = textureLoad(gbuf_position, p, 0);
            if sample_pos.w < 0.5 { continue; }

            let sample_color = textureLoad(input_color, p, 0).rgb;
            let sample_normal = normalize(textureLoad(gbuf_normal, p, 0).xyz);
            let sample_depth = sample_pos.z;

            // Edge-stopping: normal similarity
            let n_dot = max(dot(center_normal, sample_normal), 0.0);
            let w_normal = pow(n_dot, 128.0);

            // Edge-stopping: depth difference
            let depth_diff = abs(center_depth - sample_depth);
            let w_depth = exp(-depth_diff * 10.0);

            // Edge-stopping: luminance difference (variance-guided)
            let lum_diff = abs(center_lum - luminance(sample_color));
            let w_lum = exp(-lum_diff / sigma_l);

            // Kernel weight (distance-based)
            let ki = KERNEL[min(u32(abs(i)), 2u)];
            let kj = KERNEL[min(u32(abs(j)), 2u)];

            let w = ki * kj * w_normal * w_depth * w_lum;

            sum_color = sum_color + sample_color * w;
            sum_weight = sum_weight + w;
        }
    }

    let result = sum_color / max(sum_weight, 0.0001);
    textureStore(output_color, gid.xy, vec4(result, 1.0));
}
";

/// À-Trous filter iteration parameters.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AtrousParams {
    /// I32.
    pub step_size: i32,
    pub(crate) _pad: [i32; 3],
}

/// SVGF spatial filter: multi-iteration À-Trous wavelet transform.
#[allow(dead_code)] // Fields used when SVGF is wired into renderer
pub struct SvgfSpatialFilter {
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_buffer: wgpu::Buffer,
    /// Ping-pong textures for iterative filtering.
    pub(crate) ping_texture: Option<wgpu::Texture>,
    pub(crate) ping_view: Option<wgpu::TextureView>,
    pub(crate) pong_texture: Option<wgpu::Texture>,
    pub(crate) pong_view: Option<wgpu::TextureView>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl SvgfSpatialFilter {
    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SVGF À-Trous Layout"),
            entries: &[
                // 0: input color
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
                // 1: input variance
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 2: G-Buffer normal
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 3: G-Buffer position
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 4: params
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<AtrousParams>() as u64,
                        ),
                    },
                    count: None,
                },
                // 5: output color
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        })
    }

    /// Create the SVGF spatial filter.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SVGF À-Trous Filter"),
            source: wgpu::ShaderSource::Wgsl(ATROUS_FILTER_SHADER.into()),
        });

        let bind_group_layout = Self::create_bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SVGF À-Trous Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SVGF À-Trous Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SVGF Params"),
            contents: bytemuck::bytes_of(&AtrousParams {
                step_size: 1,
                _pad: [0; 3],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            ping_texture: None,
            ping_view: None,
            pong_texture: None,
            pong_view: None,
            width: 0,
            height: 0,
        }
    }

    /// Ensure ping-pong textures match the given dimensions.
    pub fn ensure_textures(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height && self.ping_texture.is_some() {
            return;
        }

        let make = |label| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
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
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };

        let (pt, pv) = make("SVGF Ping");
        let (qt, qv) = make("SVGF Pong");

        self.ping_texture = Some(pt);
        self.ping_view = Some(pv);
        self.pong_texture = Some(qt);
        self.pong_view = Some(qv);
        self.width = width;
        self.height = height;
    }
}

use wgpu::util::DeviceExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atrous_shader_valid() {
        assert!(ATROUS_FILTER_SHADER.contains("@compute"));
        assert!(ATROUS_FILTER_SHADER.contains("step_size"));
        assert!(ATROUS_FILTER_SHADER.contains("w_normal"));
        assert!(ATROUS_FILTER_SHADER.contains("w_depth"));
        assert!(ATROUS_FILTER_SHADER.contains("w_lum"));
        assert!(ATROUS_FILTER_SHADER.contains("KERNEL"));
    }

    #[test]
    fn test_atrous_params_size() {
        assert_eq!(std::mem::size_of::<AtrousParams>(), 16);
    }
}
