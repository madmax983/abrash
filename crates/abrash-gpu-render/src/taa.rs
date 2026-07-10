//! Temporal Anti-Aliasing (TAA).
//!
//! Jitters the projection matrix each frame by a sub-pixel offset (Halton sequence),
//! then resolves the jittered samples over time using temporal reprojection and
//! neighborhood clamping to prevent ghosting.
//!
//! Maps to Pass 8 (Temporal Antialiasing) from the Granja & Pereira 2021 paper.

use wgpu::util::DeviceExt;

/// Halton sequence for sub-pixel jitter offsets.
///
/// Generates quasi-random 2D offsets in [-0.5, 0.5] for projection matrix jittering.
/// Uses base 2 for X and base 3 for Y (standard TAA configuration).
#[must_use]
pub fn halton_jitter(frame_index: u32, width: u32, height: u32) -> (f32, f32) {
    let x = halton(frame_index, 2);
    let y = halton(frame_index, 3);
    // Convert [0,1] to [-0.5, 0.5] sub-pixel offset, scaled to NDC
    let jx = (x - 0.5) * 2.0 / width as f32;
    let jy = (y - 0.5) * 2.0 / height as f32;
    (jx, jy)
}

/// Halton sequence value for index `i` with the given `base`.
fn halton(index: u32, base: u32) -> f32 {
    let mut result = 0.0;
    let mut f = 1.0 / base as f32;
    let mut i: u32 = index;
    while i > 0 {
        result += f * (i % base) as f32;
        i /= base;
        f /= base as f32;
    }
    result
}

/// WGSL compute shader for TAA temporal resolve.
///
/// Reprojects the previous frame's resolved output to the current frame,
/// clamps the history to the current neighborhood, and blends.
const TAA_RESOLVE_SHADER: &str = r"
struct TaaParams {
    prev_view_proj: mat4x4<f32>,
    jitter: vec2<f32>,
    feedback: f32,   // blend factor (0.9 = 90% history)
    _pad: f32,
};

@group(0) @binding(0) var current_color: texture_2d<f32>;    // jittered current frame (HDR)
@group(0) @binding(1) var history_color: texture_2d<f32>;    // previous resolved
@group(0) @binding(2) var gbuf_position: texture_2d<f32>;    // for reprojection
@group(0) @binding(3) var<uniform> params: TaaParams;
@group(0) @binding(4) var output: texture_storage_2d<rgba16float, write>;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(current_color);
    if gid.x >= size.x || gid.y >= size.y { return; }

    let current = textureLoad(current_color, gid.xy, 0).rgb;

    // Neighborhood min/max for clamping (3×3)
    var nmin = vec3(1e10);
    var nmax = vec3(-1e10);
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let p = clamp(
                vec2<i32>(gid.xy) + vec2(dx, dy),
                vec2(0),
                vec2<i32>(size) - vec2(1)
            );
            let s = textureLoad(current_color, p, 0).rgb;
            nmin = min(nmin, s);
            nmax = max(nmax, s);
        }
    }

    // Reproject to previous frame
    let pos_sample = textureLoad(gbuf_position, gid.xy, 0);
    var history = current; // fallback
    var use_history = false;

    if pos_sample.w > 0.5 {
        let world_pos = pos_sample.xyz;
        let prev_clip = params.prev_view_proj * vec4(world_pos, 1.0);
        let prev_ndc = prev_clip.xyz / prev_clip.w;
        let prev_uv = vec2(prev_ndc.x * 0.5 + 0.5, -prev_ndc.y * 0.5 + 0.5);

        let prev_pixel = vec2<i32>(prev_uv * vec2<f32>(size));
        if prev_pixel.x >= 0 && prev_pixel.x < i32(size.x) &&
           prev_pixel.y >= 0 && prev_pixel.y < i32(size.y) {
            history = textureLoad(history_color, prev_pixel, 0).rgb;
            use_history = true;
        }
    }

    // Clamp history to neighborhood bounds (prevents ghosting)
    history = clamp(history, nmin, nmax);

    // Blend
    let alpha = select(1.0, 1.0 - params.feedback, use_history);
    let result = mix(history, current, alpha);

    textureStore(output, gid.xy, vec4(result, 1.0));
}
";

/// TAA parameters uniform.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TaaParams {
    /// Previous frame's view-projection matrix.
    pub prev_view_proj: [f32; 16],
    /// Current frame's sub-pixel jitter offset (NDC space).
    pub jitter: [f32; 2],
    /// History blend factor (0.9 = keep 90% history).
    pub feedback: f32,
    pub(crate) _pad: f32,
}

/// Temporal Anti-Aliasing pass.
#[allow(dead_code)] // Fields used when TAA is wired into renderer
pub struct TaaPass {
    pub(crate) pipeline: wgpu::ComputePipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) params_buffer: wgpu::Buffer,
    /// History buffer (previous frame's resolved result).
    pub(crate) history_texture: Option<wgpu::Texture>,
    pub(crate) history_view: Option<wgpu::TextureView>,
    /// Resolved output.
    pub(crate) output_texture: Option<wgpu::Texture>,
    pub(crate) output_view: Option<wgpu::TextureView>,
    /// Frame counter for Halton sequence.
    pub(crate) frame_index: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl TaaPass {
    /// Create the TAA pass.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TAA Resolve Compute"),
            source: wgpu::ShaderSource::Wgsl(TAA_RESOLVE_SHADER.into()),
        });

        let bind_group_layout = create_taa_bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TAA Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("TAA Resolve Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("TAA Params"),
            contents: &[0u8; std::mem::size_of::<TaaParams>()],
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
            frame_index: 0,
            width: 0,
            height: 0,
        }
    }

    /// Ensure TAA textures match the given dimensions.
    pub fn ensure_textures(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height && self.output_texture.is_some() {
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

        let (ht, hv) = make("TAA History");
        let (ot, ov) = make("TAA Output");

        self.history_texture = Some(ht);
        self.history_view = Some(hv);
        self.output_texture = Some(ot);
        self.output_view = Some(ov);
        self.width = width;
        self.height = height;
        self.frame_index = 0;
    }

    /// Get the current frame's jitter offset for the projection matrix.
    #[must_use]
    pub fn current_jitter(&self) -> (f32, f32) {
        halton_jitter(self.frame_index, self.width.max(1), self.height.max(1))
    }

    /// Advance to the next frame (swap history ↔ output, increment counter).
    pub const fn advance_frame(&mut self) {
        std::mem::swap(&mut self.history_texture, &mut self.output_texture);
        std::mem::swap(&mut self.history_view, &mut self.output_view);
        self.frame_index = self.frame_index.wrapping_add(1);
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn test_halton_sequence() {
        // Base 2: 0.5, 0.25, 0.75, 0.125, ...
        assert!((halton(1, 2) - 0.5).abs() < f32::EPSILON);
        assert!((halton(2, 2) - 0.25).abs() < f32::EPSILON);
        assert!((halton(3, 2) - 0.75).abs() < f32::EPSILON);

        // Base 3: 1/3, 2/3, 1/9, ...
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 0.001);
        assert!((halton(2, 3) - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_halton_jitter_range() {
        for i in 0..16 {
            let (jx, jy) = halton_jitter(i, 1920, 1080);
            // Jitter should be sub-pixel in NDC
            assert!(jx.abs() < 1.0 / 1920.0 * 2.0);
            assert!(jy.abs() < 1.0 / 1080.0 * 2.0);
        }
    }

    #[test]
    fn test_taa_shader_valid() {
        assert!(TAA_RESOLVE_SHADER.contains("@compute"));
        assert!(TAA_RESOLVE_SHADER.contains("prev_view_proj"));
        assert!(TAA_RESOLVE_SHADER.contains("clamp")); // neighborhood clamping
        assert!(TAA_RESOLVE_SHADER.contains("feedback"));
    }

    #[test]
    fn test_taa_params_size() {
        let size = std::mem::size_of::<TaaParams>();
        assert_eq!(size, 80); // 64 (mat4) + 8 (jitter) + 4 (feedback) + 4 (pad)
        assert_eq!(size % 16, 0);
    }
}

fn create_taa_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("TAA Layout"),
        entries: &[
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
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<TaaParams>() as u64
                    ),
                },
                count: None,
            },
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
        ],
    })
}
