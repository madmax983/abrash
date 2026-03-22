//! Temporal accumulation for SVGF denoising.
//!
//! Reprojects the current frame's noisy input to the previous frame's screen space
//! using motion vectors (derived from current and previous view-projection matrices).
//! Blends the current sample with accumulated history using an exponential moving average.
//! Computes per-pixel variance for the spatial filter's edge-stopping functions.
//! Applies variance color clamping (paper §3.1) to reduce ghosting artifacts.

/// WGSL compute shader for temporal accumulation with variance color clamping.
pub const TEMPORAL_ACCUMULATION_SHADER: &str = r"
struct TemporalParams {
    prev_view_proj: mat4x4<f32>,
    alpha: f32,           // blend factor (0.05 = 95% history)
    _pad: vec3<f32>,
};

@group(0) @binding(0) var current_input: texture_2d<f32>;     // noisy 1-spp
@group(0) @binding(1) var history: texture_2d<f32>;           // previous accumulated
@group(0) @binding(2) var gbuf_position: texture_2d<f32>;     // world pos for reprojection
@group(0) @binding(3) var<uniform> params: TemporalParams;
@group(0) @binding(4) var output_color: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var output_variance: texture_storage_2d<r16float, write>;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(current_input);
    if gid.x >= size.x || gid.y >= size.y { return; }

    let current = textureLoad(current_input, gid.xy, 0).rgb;
    let pos_sample = textureLoad(gbuf_position, gid.xy, 0);

    // No geometry → pass through
    if pos_sample.w < 0.5 {
        textureStore(output_color, gid.xy, vec4(current, 1.0));
        textureStore(output_variance, gid.xy, vec4(0.0));
        return;
    }

    // Reproject to previous frame
    let world_pos = pos_sample.xyz;
    let prev_clip = params.prev_view_proj * vec4(world_pos, 1.0);
    let prev_ndc = prev_clip.xyz / prev_clip.w;
    let prev_uv = vec2(prev_ndc.x * 0.5 + 0.5, -prev_ndc.y * 0.5 + 0.5);

    var history_color = current; // fallback: no history
    var valid_history = false;

    if prev_uv.x >= 0.0 && prev_uv.x <= 1.0 && prev_uv.y >= 0.0 && prev_uv.y <= 1.0 {
        let prev_pixel = vec2<i32>(vec2(prev_uv.x * f32(size.x), prev_uv.y * f32(size.y)));
        if prev_pixel.x >= 0 && prev_pixel.x < i32(size.x) &&
           prev_pixel.y >= 0 && prev_pixel.y < i32(size.y) {
            history_color = textureLoad(history, prev_pixel, 0).rgb;
            valid_history = true;
        }
    }

    // Variance color clamping (paper §3.1)
    // Compute neighborhood mean and variance for clamping
    var mean = vec3(0.0);
    var mean_sq = vec3(0.0);
    var count = 0.0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let p = vec2<i32>(gid.xy) + vec2(dx, dy);
            if p.x >= 0 && p.x < i32(size.x) && p.y >= 0 && p.y < i32(size.y) {
                let s = textureLoad(current_input, p, 0).rgb;
                mean = mean + s;
                mean_sq = mean_sq + s * s;
                count = count + 1.0;
            }
        }
    }
    mean = mean / count;
    mean_sq = mean_sq / count;
    let variance = max(mean_sq - mean * mean, vec3(0.0));
    let std_dev = sqrt(variance);

    // Clamp history to neighborhood bounds
    if valid_history {
        let clamp_min = mean - std_dev * 1.5;
        let clamp_max = mean + std_dev * 1.5;
        history_color = clamp(history_color, clamp_min, clamp_max);
    }

    // Blend
    let alpha = select(1.0, params.alpha, valid_history);
    let accumulated = mix(history_color, current, alpha);

    // Output variance (scalar luminance variance)
    let lum_variance = max(luminance(variance), 0.0);

    textureStore(output_color, gid.xy, vec4(accumulated, 1.0));
    textureStore(output_variance, gid.xy, vec4(lum_variance));
}
";

/// Temporal accumulation parameters.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TemporalParams {
    /// Previous frame's view-projection matrix (for reprojection).
    pub prev_view_proj: [f32; 16],
    /// Blend factor (0.05 = 95% history, 1.0 = no history).
    pub alpha: f32,
    pub _pad: [f32; 3],
}

/// Temporal accumulation pass for SVGF denoising.
pub struct TemporalAccumulationPass {
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_buffer: wgpu::Buffer,
    /// History buffer (previous frame's accumulated result).
    pub(crate) history_texture: Option<wgpu::Texture>,
    pub(crate) history_view: Option<wgpu::TextureView>,
    /// Accumulated output.
    pub(crate) output_texture: Option<wgpu::Texture>,
    pub(crate) output_view: Option<wgpu::TextureView>,
    /// Variance output.
    pub(crate) variance_texture: Option<wgpu::Texture>,
    pub(crate) variance_view: Option<wgpu::TextureView>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl TemporalAccumulationPass {
    /// Create the temporal accumulation pass.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Temporal Accumulation Compute"),
            source: wgpu::ShaderSource::Wgsl(TEMPORAL_ACCUMULATION_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Temporal Accumulation Layout"),
            entries: &[
                // 0: current noisy input
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
                // 1: history
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
                // 2: G-Buffer position
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
                // 3: params
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            TemporalParams,
                        >()
                            as u64),
                    },
                    count: None,
                },
                // 4: output accumulated color
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // 5: output variance
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::R16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Temporal Accumulation Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Temporal Accumulation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Temporal Params"),
            contents: &[0u8; std::mem::size_of::<TemporalParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            history_texture: None,
            history_view: None,
            output_texture: None,
            output_view: None,
            variance_texture: None,
            variance_view: None,
            width: 0,
            height: 0,
        }
    }

    /// Ensure all temporal textures match the given dimensions.
    pub fn ensure_textures(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height && self.output_texture.is_some() {
            return;
        }

        let make = |label, format: wgpu::TextureFormat| {
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
                format,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };

        let (ht, hv) = make("Temporal History", wgpu::TextureFormat::Rgba16Float);
        let (ot, ov) = make("Temporal Output", wgpu::TextureFormat::Rgba16Float);
        let (vt, vv) = make("Temporal Variance", wgpu::TextureFormat::R16Float);

        self.history_texture = Some(ht);
        self.history_view = Some(hv);
        self.output_texture = Some(ot);
        self.output_view = Some(ov);
        self.variance_texture = Some(vt);
        self.variance_view = Some(vv);
        self.width = width;
        self.height = height;
    }

    /// Swap output → history for the next frame.
    pub fn swap_history(&mut self) {
        std::mem::swap(&mut self.history_texture, &mut self.output_texture);
        std::mem::swap(&mut self.history_view, &mut self.output_view);
    }
}

use wgpu::util::DeviceExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_shader_valid() {
        assert!(TEMPORAL_ACCUMULATION_SHADER.contains("@compute"));
        assert!(TEMPORAL_ACCUMULATION_SHADER.contains("prev_view_proj"));
        assert!(TEMPORAL_ACCUMULATION_SHADER.contains("variance"));
        assert!(TEMPORAL_ACCUMULATION_SHADER.contains("clamp")); // variance color clamping
        assert!(TEMPORAL_ACCUMULATION_SHADER.contains("luminance"));
    }

    #[test]
    fn test_temporal_params_size() {
        let size = std::mem::size_of::<TemporalParams>();
        assert_eq!(size, 80); // 64 (mat4) + 4 (alpha) + 12 (pad) = 80
        assert_eq!(size % 16, 0);
    }
}
