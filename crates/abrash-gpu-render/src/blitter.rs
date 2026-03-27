//! GPU-accelerated 2D sprite blitting.
//!
//! Types for batched sprite rendering on the GPU, matching the CPU blitter's
//! three-tier model (opaque, color-key, alpha) but expressed as instance data
//! suitable for a single instanced draw call.

#[allow(unused_imports)]
use abrash_core::blitter::SrcRect;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::capture::aligned_bytes_per_row;
use crate::device::GpuDevice;

/// Opaque handle to an atlas texture uploaded to the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtlasHandle(pub(crate) u32);

/// Blit transparency mode, mirroring the CPU blitter tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlitMode {
    /// Straight copy, no transparency.
    Opaque,
    /// Skip pixels matching the given color key (ARGB packed u32).
    ColorKey(u32),
    /// Per-pixel alpha blending from the source alpha channel.
    Alpha,
}

impl BlitMode {
    /// Numeric ID for GPU storage buffer.
    #[must_use]
    pub const fn as_u32(&self) -> u32 {
        match self {
            Self::Opaque => 0,
            Self::ColorKey(_) => 1,
            Self::Alpha => 2,
        }
    }

    /// Color key value (0 for non-colorkey modes).
    #[must_use]
    pub const fn color_key(&self) -> u32 {
        match self {
            Self::ColorKey(key) => *key,
            _ => 0,
        }
    }

    /// Whether this mode uses the alpha blend pipeline.
    #[must_use]
    pub const fn uses_alpha_pipeline(&self) -> bool {
        matches!(self, Self::Alpha)
    }
}

/// Coordinate interpretation for sprite positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum CoordMode {
    /// Coordinates are in framebuffer pixels.
    Pixel,
    /// Coordinates are normalized \[0.0, 1.0\].
    Normalized,
}

/// Per-sprite instance data uploaded to the GPU vertex/instance buffer.
///
/// Layout is `#[repr(C)]` and `Pod` so it can be memcpy'd straight into a
/// mapped GPU buffer. Total size: **48 bytes**.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpriteInstance {
    /// Source rectangle X in texels.
    pub src_x: f32,
    /// Source rectangle Y in texels.
    pub src_y: f32,
    /// Source rectangle width in texels.
    pub src_w: f32,
    /// Source rectangle height in texels.
    pub src_h: f32,
    /// Destination X in pixels (or normalized).
    pub dst_x: f32,
    /// Destination Y in pixels (or normalized).
    pub dst_y: f32,
    /// Destination width in pixels (or normalized).
    pub dst_w: f32,
    /// Destination height in pixels (or normalized).
    pub dst_h: f32,
    /// Atlas texture width in texels (for UV computation in shader).
    pub atlas_w: f32,
    /// Atlas texture height in texels (for UV computation in shader).
    pub atlas_h: f32,
    /// Blend mode: 0 = opaque, 1 = color-key, 2 = alpha.
    pub blend_mode: u32,
    /// Packed ARGB color key (only meaningful when `blend_mode == 1`).
    pub color_key: u32,
}

/// A single sprite draw command binding an atlas to instance data.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct SpriteCommand {
    /// Which atlas texture to sample.
    pub atlas: AtlasHandle,
    /// The instance data for this sprite.
    pub instance: SpriteInstance,
}

/// Embedded WGSL shader source for the GPU batched 2D blitter.
///
/// Contains a vertex shader (`vs_main`) that generates fullscreen quads from
/// instance data, plus two fragment shaders:
/// - `fs_opaque`: opaque blit with optional color-key discard
/// - `fs_alpha`: alpha-blended blit (relies on GPU blend state)
#[allow(dead_code)]
pub(crate) const BLITTER_SHADER_SRC: &str = r"
// ── Uniform / storage types ────────────────────────────────────────────
struct ScreenUniforms {
    width:  f32,
    height: f32,
}

struct SpriteInstance {
    src_x:      f32,
    src_y:      f32,
    src_w:      f32,
    src_h:      f32,
    dst_x:      f32,
    dst_y:      f32,
    dst_w:      f32,
    dst_h:      f32,
    atlas_w:    f32,
    atlas_h:    f32,
    blend_mode: u32,
    color_key:  u32,
}

// ── Bindings ───────────────────────────────────────────────────────────
@group(0) @binding(0) var<uniform>       screen:        ScreenUniforms;
@group(0) @binding(1) var<storage, read> sprites:       array<SpriteInstance>;
@group(1) @binding(0) var               atlas_tex:      texture_2d<f32>;
@group(1) @binding(1) var               atlas_sampler:  sampler;

// ── Vertex → Fragment interface ────────────────────────────────────────
struct VsOut {
    @builtin(position)                          pos:         vec4<f32>,
    @location(0)                                uv:          vec2<f32>,
    @location(1) @interpolate(flat)             instance_id: u32,
}

// ── Vertex shader ──────────────────────────────────────────────────────
@vertex
fn vs_main(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> VsOut {
    // Index-based quad: two triangles from four corners.
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );
    var indices = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);

    let idx    = indices[vid];
    let corner = corners[idx];
    let sprite = sprites[iid];

    // Pixel position of this vertex.
    let px = sprite.dst_x + corner.x * sprite.dst_w;
    let py = sprite.dst_y + corner.y * sprite.dst_h;

    // Pixel coords → NDC.
    let ndc_x = (px / screen.width)  * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.height) * 2.0;

    // Atlas UV from source rect.
    let u = (sprite.src_x + corner.x * sprite.src_w) / sprite.atlas_w;
    let v = (sprite.src_y + corner.y * sprite.src_h) / sprite.atlas_h;

    var out: VsOut;
    out.pos         = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv          = vec2<f32>(u, v);
    out.instance_id = iid;
    return out;
}

// ── Helper: pack an f32 RGBA sample into a 0xAARRGGBB u32 ─────────────
fn pack_argb(c: vec4<f32>) -> u32 {
    let a = u32(c.a * 255.0) << 24u;
    let r = u32(c.r * 255.0) << 16u;
    let g = u32(c.g * 255.0) << 8u;
    let b = u32(c.b * 255.0);
    return a | r | g | b;
}

// ── Fragment shader: opaque / color-key ────────────────────────────────
@fragment
fn fs_opaque(in: VsOut) -> @location(0) vec4<f32> {
    let color  = textureSample(atlas_tex, atlas_sampler, in.uv);
    let sprite = sprites[in.instance_id];

    // Color-key discard (blend_mode == 1).
    if sprite.blend_mode == 1u {
        let packed = pack_argb(color);
        if packed == sprite.color_key {
            discard;
        }
    }

    return vec4<f32>(color.rgb, 1.0);
}

// ── Fragment shader: alpha blend ───────────────────────────────────────
@fragment
fn fs_alpha(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(atlas_tex, atlas_sampler, in.uv);
}
";

/// An atlas texture uploaded to the GPU, with its own bind group for sampling.
#[allow(dead_code)]
pub(crate) struct GpuAtlas {
    /// The GPU texture holding atlas pixel data.
    pub texture: wgpu::Texture,
    /// Default view into the atlas texture.
    pub view: wgpu::TextureView,
    /// Bind group pairing this atlas's view with the shared sampler.
    pub bind_group: wgpu::BindGroup,
    /// Atlas width in texels.
    pub width: u32,
    /// Atlas height in texels.
    pub height: u32,
}

/// GPU-accelerated batched 2D sprite blitter.
///
/// Owns the full wgpu pipeline state needed to render sprites via instanced
/// draw calls. Sprites are queued with [`SpriteCommand`] and flushed in a
/// single (or few) draw call(s), sorted by atlas and blend mode.
#[allow(dead_code)]
pub struct GpuBlitter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,
    frame_bind_group_layout: wgpu::BindGroupLayout,
    atlas_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    screen_uniform_buffer: wgpu::Buffer,
    render_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    readback_buffer: wgpu::Buffer,
    commands: Vec<SpriteCommand>,
    atlases: Vec<GpuAtlas>,
}

/// Create the frame bind group layout (group 0): screen uniforms + sprite storage.
fn create_frame_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Blitter Frame BGL"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Create the atlas bind group layout (group 1): texture + sampler.
fn create_atlas_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Blitter Atlas BGL"),
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

/// Build a render pipeline for a specific fragment entry point and blend state.
fn create_blit_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &str,
    fs_entry: &str,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl GpuBlitter {
    /// Create a new GPU blitter targeting the given framebuffer dimensions.
    ///
    /// Clones the wgpu `Device` and `Queue` from the provided [`GpuDevice`]
    /// (both are `Arc`-based, so cloning is cheap).
    #[must_use]
    pub fn new(gpu: &GpuDevice, width: u32, height: u32) -> Self {
        let device = gpu.device().clone();
        let queue = gpu.queue().clone();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blitter Shader"),
            source: wgpu::ShaderSource::Wgsl(BLITTER_SHADER_SRC.into()),
        });

        let frame_bind_group_layout = create_frame_bind_group_layout(&device);
        let atlas_bind_group_layout = create_atlas_bind_group_layout(&device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blitter Pipeline Layout"),
            bind_group_layouts: &[
                Some(&frame_bind_group_layout),
                Some(&atlas_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let opaque_pipeline = create_blit_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            "Blitter Opaque Pipeline",
            "fs_opaque",
            Some(wgpu::BlendState::REPLACE),
        );

        let alpha_pipeline = create_blit_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            "Blitter Alpha Pipeline",
            "fs_alpha",
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            }),
        );

        let screen_data: [f32; 2] = [width as f32, height as f32];
        let screen_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blitter Screen Uniforms"),
            contents: bytemuck::cast_slice(&screen_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blitter Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Blitter Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let padded_bytes_per_row = aligned_bytes_per_row(width);
        let readback_size = u64::from(padded_bytes_per_row) * u64::from(height);
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blitter Readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            opaque_pipeline,
            alpha_pipeline,
            frame_bind_group_layout,
            atlas_bind_group_layout,
            sampler,
            width,
            height,
            screen_uniform_buffer,
            render_texture,
            render_view,
            readback_buffer,
            commands: Vec::new(),
            atlases: Vec::new(),
        }
    }

    /// Framebuffer width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Framebuffer height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Number of sprite commands currently queued.
    #[must_use]
    pub fn queued_count(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_handle_is_copy_and_eq() {
        let a = AtlasHandle(0);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn blit_mode_variants() {
        let opaque = BlitMode::Opaque;
        let key = BlitMode::ColorKey(0xFF00_FF00);
        let alpha = BlitMode::Alpha;
        assert_ne!(opaque, key);
        assert_ne!(key, alpha);
    }

    #[test]
    fn sprite_instance_is_pod() {
        assert_eq!(std::mem::size_of::<SpriteInstance>(), 48);
    }

    #[test]
    fn sprite_instance_zeroed() {
        let inst = SpriteInstance::zeroed();
        assert_eq!(inst.src_x, 0.0);
        assert_eq!(inst.blend_mode, 0);
    }

    #[test]
    fn shader_source_contains_entry_points() {
        assert!(BLITTER_SHADER_SRC.contains("fn vs_main"));
        assert!(BLITTER_SHADER_SRC.contains("fn fs_opaque"));
        assert!(BLITTER_SHADER_SRC.contains("fn fs_alpha"));
        assert!(BLITTER_SHADER_SRC.contains("struct SpriteInstance"));
    }

    #[test]
    fn blit_mode_to_u32() {
        assert_eq!(BlitMode::Opaque.as_u32(), 0);
        assert_eq!(BlitMode::ColorKey(0xFF00FF).as_u32(), 1);
        assert_eq!(BlitMode::Alpha.as_u32(), 2);
    }

    #[test]
    fn blit_mode_color_key_value() {
        assert_eq!(BlitMode::Opaque.color_key(), 0);
        assert_eq!(BlitMode::ColorKey(0xFF00FF).color_key(), 0xFF00FF);
        assert_eq!(BlitMode::Alpha.color_key(), 0);
    }

    #[test]
    fn blit_mode_uses_alpha_pipeline() {
        assert!(!BlitMode::Opaque.uses_alpha_pipeline());
        assert!(!BlitMode::ColorKey(0).uses_alpha_pipeline());
        assert!(BlitMode::Alpha.uses_alpha_pipeline());
    }

    #[test]
    fn sort_commands_by_atlas_then_blend() {
        let mut commands = vec![
            SpriteCommand {
                atlas: AtlasHandle(1),
                instance: SpriteInstance {
                    blend_mode: 2,
                    ..SpriteInstance::zeroed()
                },
            },
            SpriteCommand {
                atlas: AtlasHandle(0),
                instance: SpriteInstance {
                    blend_mode: 0,
                    ..SpriteInstance::zeroed()
                },
            },
            SpriteCommand {
                atlas: AtlasHandle(0),
                instance: SpriteInstance {
                    blend_mode: 2,
                    ..SpriteInstance::zeroed()
                },
            },
            SpriteCommand {
                atlas: AtlasHandle(1),
                instance: SpriteInstance {
                    blend_mode: 0,
                    ..SpriteInstance::zeroed()
                },
            },
        ];

        commands.sort_by(|a, b| {
            a.atlas
                .0
                .cmp(&b.atlas.0)
                .then(a.instance.blend_mode.cmp(&b.instance.blend_mode))
        });

        assert_eq!(commands[0].atlas.0, 0);
        assert_eq!(commands[0].instance.blend_mode, 0);
        assert_eq!(commands[1].atlas.0, 0);
        assert_eq!(commands[1].instance.blend_mode, 2);
        assert_eq!(commands[2].atlas.0, 1);
        assert_eq!(commands[2].instance.blend_mode, 0);
        assert_eq!(commands[3].atlas.0, 1);
        assert_eq!(commands[3].instance.blend_mode, 2);
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use crate::device::{GpuDevice, GpuDeviceConfig};

    fn headless_device() -> GpuDevice {
        GpuDevice::new_headless(&GpuDeviceConfig::headless())
            .expect("headless GPU device required for tests")
    }

    #[test]
    fn gpu_blitter_new_succeeds() {
        let gpu = headless_device();
        let blitter = GpuBlitter::new(&gpu, 800, 600);
        assert_eq!(blitter.width(), 800);
        assert_eq!(blitter.height(), 600);
        assert_eq!(blitter.queued_count(), 0);
    }
}
