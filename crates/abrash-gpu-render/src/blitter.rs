//! GPU-accelerated 2D sprite blitting.
//!
//! Types for batched sprite rendering on the GPU, matching the CPU blitter's
//! three-tier model (opaque, color-key, alpha) but expressed as instance data
//! suitable for a single instanced draw call.

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
/// draw calls. Sprites are queued with `SpriteCommand` and flushed in a
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
    instances: Vec<SpriteInstance>,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
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
            // ⚡ Bolt: Pre-allocate standard blitter capacities to prevent initial heap resizing
            commands: Vec::with_capacity(128),
            atlases: Vec::with_capacity(16),
            instances: Vec::with_capacity(128),
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

    /// Upload a CPU-side [`abrash_core::texture::Texture`] as a GPU atlas.
    ///
    /// Converts the texture's `0xAARRGGBB` pixel data to RGBA8 for wgpu,
    /// creates a GPU texture + bind group, and returns an opaque [`AtlasHandle`]
    /// that can be referenced in future sprite draw commands.
    pub fn upload_atlas(&mut self, texture: &abrash_core::texture::Texture) -> AtlasHandle {
        let width = texture.width();
        let height = texture.height();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let gpu_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Blitter Atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Convert 0xAARRGGBB pixels to RGBA bytes for wgpu.
        let pixels = texture.pixels();
        let rgba: Vec<u8> = pixels.iter().flat_map(|&px| [
                ((px >> 16) & 0xFF) as u8,
                ((px >> 8) & 0xFF) as u8,
                (px & 0xFF) as u8,
                ((px >> 24) & 0xFF) as u8,
            ]).collect();

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blitter Atlas Bind"),
            layout: &self.atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let index = self.atlases.len() as u32;
        self.atlases.push(GpuAtlas {
            texture: gpu_texture,
            view,
            bind_group,
            width,
            height,
        });
        AtlasHandle(index)
    }

    /// Queue a sprite for rendering using pixel coordinates.
    /// Coordinates match the CPU blitter: i32, top-left origin.
    pub fn queue(
        &mut self,
        atlas: AtlasHandle,
        src: SrcRect,
        dst_x: i32,
        dst_y: i32,
        mode: BlitMode,
    ) {
        let ga = &self.atlases[atlas.0 as usize];
        self.commands.push(SpriteCommand {
            atlas,
            instance: SpriteInstance {
                src_x: src.x as f32,
                src_y: src.y as f32,
                src_w: src.w as f32,
                src_h: src.h as f32,
                dst_x: dst_x as f32,
                dst_y: dst_y as f32,
                dst_w: src.w as f32,
                dst_h: src.h as f32,
                atlas_w: ga.width as f32,
                atlas_h: ga.height as f32,
                blend_mode: mode.as_u32(),
                color_key: mode.color_key(),
            },
        });
    }

    /// Queue a sprite using normalized coordinates (0.0..1.0, top-left origin).
    /// Position is scaled to screen pixels. Sprite size remains in pixels.
    pub fn queue_normalized(
        &mut self,
        atlas: AtlasHandle,
        src: SrcRect,
        dst_x: f32,
        dst_y: f32,
        mode: BlitMode,
    ) {
        let px_x = dst_x * self.width as f32;
        let px_y = dst_y * self.height as f32;
        let ga = &self.atlases[atlas.0 as usize];
        self.commands.push(SpriteCommand {
            atlas,
            instance: SpriteInstance {
                src_x: src.x as f32,
                src_y: src.y as f32,
                src_w: src.w as f32,
                src_h: src.h as f32,
                dst_x: px_x,
                dst_y: px_y,
                dst_w: src.w as f32,
                dst_h: src.h as f32,
                atlas_w: ga.width as f32,
                atlas_h: ga.height as f32,
                blend_mode: mode.as_u32(),
                color_key: mode.color_key(),
            },
        });
    }

    /// Discard all queued sprites without rendering.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Internal render method: sort, batch, and draw all queued sprites into
    /// the given render target view.
    ///
    /// `load_op` controls whether the render target is cleared or preserved
    /// before drawing. Use `LoadOp::Clear` for a fresh frame, or
    /// `LoadOp::Load` to composite onto existing content.
    ///
    /// After submission the command queue is cleared.
    fn flush_to_view(
        &mut self,
        target_view: &wgpu::TextureView,
        load_op: wgpu::LoadOp<wgpu::Color>,
    ) {
        if self.commands.is_empty() {
            return;
        }

        // Sort by (atlas, blend_mode) to minimise pipeline/bind-group switches.
        self.commands.sort_by(|a, b| {
            a.atlas
                .0
                .cmp(&b.atlas.0)
                .then(a.instance.blend_mode.cmp(&b.instance.blend_mode))
        });

        // Build the per-sprite storage buffer from the sorted command list.
        self.instances.clear();
        self.instances
            .extend(self.commands.iter().map(|c| c.instance));
        let sprite_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Blitter Sprite Buffer"),
                contents: bytemuck::cast_slice(&self.instances),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // Frame bind group (group 0): screen uniforms + sprite storage.
        let frame_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blitter Frame Bind"),
            layout: &self.frame_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.screen_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sprite_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Blitter Render"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blitter Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Walk the sorted commands and emit one draw call per contiguous
            // (atlas_id, uses_alpha) group.
            let total = self.commands.len();
            let mut cursor = 0_usize;
            while cursor < total {
                let atlas_id = self.commands[cursor].atlas.0 as usize;
                let alpha = BlitMode::Alpha.as_u32() == self.commands[cursor].instance.blend_mode;

                // Find the end of this contiguous group.
                let mut end = cursor + 1;
                while end < total {
                    let same_atlas = self.commands[end].atlas.0 as usize == atlas_id;
                    let same_pipe = (BlitMode::Alpha.as_u32()
                        == self.commands[end].instance.blend_mode)
                        == alpha;
                    if !same_atlas || !same_pipe {
                        break;
                    }
                    end += 1;
                }

                // Bind the pipeline matching this group's blend mode.
                if alpha {
                    render_pass.set_pipeline(&self.alpha_pipeline);
                } else {
                    render_pass.set_pipeline(&self.opaque_pipeline);
                }

                render_pass.set_bind_group(0, Some(&frame_bind_group), &[]);
                render_pass.set_bind_group(1, Some(&self.atlases[atlas_id].bind_group), &[]);

                // 6 vertices per quad (two triangles), instanced.
                render_pass.draw(0..6, cursor as u32..end as u32);

                cursor = end;
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.commands.clear();
    }

    /// Render all queued sprites directly to a window surface.
    ///
    /// This is the fastest output path — no CPU readback.
    /// The command queue is cleared after rendering.
    ///
    /// # Panics
    ///
    /// Panics if the swapchain texture cannot be acquired (e.g. surface lost).
    #[cfg(feature = "windowed")]
    pub fn flush_to_screen(&mut self, surface: &crate::surface::GpuSurface) {
        if self.commands.is_empty() {
            return;
        }

        let frame = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            e => panic!("failed to acquire swapchain texture: {e:?}"),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.flush_to_view(
            &view,
            wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }),
        );

        frame.present();
    }

    /// Render all queued sprites to the internal render texture, then copy the
    /// result into the readback buffer and return the raw RGBA bytes.
    ///
    /// Returns an empty `Vec` if there were no queued sprites.
    #[allow(dead_code)]
    pub(crate) fn flush_and_readback(&mut self) -> Vec<u8> {
        if self.commands.is_empty() {
            return Vec::new();
        }

        // Create a fresh view from render_texture to avoid borrow conflict.
        let target_view = self
            .render_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.flush_to_view(
            &target_view,
            wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }),
        );

        // Copy render texture to readback buffer.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Blitter Readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned_bytes_per_row(self.width)),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        // Map and read back.
        let buffer_slice = self.readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let bytes = data.to_vec();
        drop(data);
        self.readback_buffer.unmap();
        bytes
    }

    /// Upload current framebuffer content to the render texture so that
    /// subsequent rendering composites on top of the existing image.
    fn upload_framebuffer(&self, fb: &abrash_core::framebuffer::Framebuffer) {
        let fb_pixels = fb.as_slice();
        let w = self.width;
        let h = self.height;

        // Convert 0xAARRGGBB → RGBA bytes for wgpu.
        let rgba: Vec<u8> = fb_pixels.iter().flat_map(|&px| [
                ((px >> 16) & 0xFF) as u8,
                ((px >> 8) & 0xFF) as u8,
                (px & 0xFF) as u8,
                ((px >> 24) & 0xFF) as u8,
            ]).collect();

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Render all queued sprites and write the result into a CPU
    /// [`Framebuffer`](abrash_core::framebuffer::Framebuffer).
    ///
    /// The current framebuffer content is uploaded to the GPU first, so
    /// color-key and alpha-blended sprites composite correctly against
    /// the existing background. After rendering the full image is read
    /// back and written into `fb`.
    ///
    /// # Panics
    ///
    /// Panics if the GPU readback buffer mapping fails (e.g. device lost).
    pub fn flush_to_framebuffer(&mut self, fb: &mut abrash_core::framebuffer::Framebuffer) {
        if self.commands.is_empty() {
            return;
        }

        // Upload current framebuffer as the render target background.
        self.upload_framebuffer(fb);

        // Render sprites on top (LoadOp::Load preserves uploaded content).
        let target_view = self
            .render_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.flush_to_view(&target_view, wgpu::LoadOp::Load);

        // Copy render texture to readback buffer.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Blitter FB Readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned_bytes_per_row(self.width)),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        // Map and read back.
        let buffer_slice = self.readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();

        // Convert RGBA readback → 0xAARRGGBB and write into framebuffer.
        // ⚡ Bolt: Iterating directly over the mapped buffer view `data` entirely
        // eliminates the O(N) dynamic heap allocation and memory copy of `.to_vec()`.
        let padded_bpr = aligned_bytes_per_row(self.width) as usize;
        let fb_pixels = fb.as_mut_slice();
        let fb_w = self.width as usize;

        for row in 0..self.height as usize {
            let row_start = row * padded_bpr;
            for col in 0..fb_w {
                let offset = row_start + col * 4;
                let red = u32::from(data[offset]);
                let green = u32::from(data[offset + 1]);
                let blue = u32::from(data[offset + 2]);
                let alpha = u32::from(data[offset + 3]);
                fb_pixels[row * fb_w + col] = (alpha << 24) | (red << 16) | (green << 8) | blue;
            }
        }

        drop(data);
        self.readback_buffer.unmap();
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

    #[test]
    fn queue_and_clear() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 800, 600);
        let tex = abrash_core::texture::Texture::new(64, 64).unwrap();
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };

        blitter.queue(atlas, src, 100, 50, BlitMode::Opaque);
        blitter.queue(atlas, src, 200, 50, BlitMode::Alpha);
        assert_eq!(blitter.queued_count(), 2);

        blitter.clear();
        assert_eq!(blitter.queued_count(), 0);
    }

    #[test]
    fn queue_negative_coords() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 800, 600);
        let tex = abrash_core::texture::Texture::new(64, 64).unwrap();
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };

        blitter.queue(atlas, src, -10, -20, BlitMode::Opaque);
        assert_eq!(blitter.queued_count(), 1);
    }

    #[test]
    fn upload_atlas_returns_sequential_handles() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);

        let tex = abrash_core::texture::Texture::new(16, 16).unwrap();
        let h0 = blitter.upload_atlas(&tex);
        let h1 = blitter.upload_atlas(&tex);
        assert_eq!(h0, AtlasHandle(0));
        assert_eq!(h1, AtlasHandle(1));
    }

    #[test]
    fn flush_renders_opaque_sprite() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);

        // Create a 4x4 solid red texture.
        let mut tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        for y in 0..4_u32 {
            for x in 0..4_u32 {
                tex.set_pixel(x, y, 0xFFFF_0000);
            }
        }
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        blitter.queue(atlas, src, 0, 0, BlitMode::Opaque);

        let rgba = blitter.flush_and_readback();
        assert!(!rgba.is_empty());

        // Check the first pixel in the top-left region is reddish
        // (GPU rounding may not be exact).
        let r = rgba[0];
        let g = rgba[1];
        let b = rgba[2];
        assert!(r > 200, "red channel: {r}");
        assert!(g < 50, "green channel: {g}");
        assert!(b < 50, "blue channel: {b}");
    }

    #[test]
    fn flush_to_framebuffer_writes_pixels() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);
        let mut fb = abrash_core::framebuffer::Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF00_0000);

        let mut tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        for y in 0..4_u32 {
            for x in 0..4_u32 {
                tex.set_pixel(x, y, 0xFF00_FF00); // green
            }
        }
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        blitter.queue(atlas, src, 10, 10, BlitMode::Opaque);
        blitter.flush_to_framebuffer(&mut fb);

        let px = fb.get_pixel(10, 10).unwrap();
        let g = (px >> 8) & 0xFF;
        assert!(g > 200, "green channel: {g}");

        // Background should be unchanged at (0,0).
        let bg = fb.get_pixel(0, 0).unwrap();
        assert_eq!(bg & 0x00FF_FFFF, 0, "background should be black");
    }

    #[test]
    fn flush_zero_sprites_is_noop() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);
        let mut fb = abrash_core::framebuffer::Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFFAA_BBCC);

        blitter.flush_to_framebuffer(&mut fb);

        // Framebuffer unchanged.
        let px = fb.get_pixel(0, 0).unwrap();
        assert_eq!(px, 0xFFAA_BBCC);
    }

    #[test]
    fn flush_colorkey_skips_key_pixels() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);
        let mut fb = abrash_core::framebuffer::Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF00_00FF); // blue background

        // 4x4 texture: top-left 2x2 red, rest magenta (key)
        let mut tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                if x < 2 && y < 2 {
                    tex.set_pixel(x, y, 0xFFFF_0000); // red (keep)
                } else {
                    tex.set_pixel(x, y, 0xFFFF_00FF); // magenta (key)
                }
            }
        }
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        blitter.queue(atlas, src, 0, 0, BlitMode::ColorKey(0xFFFF_00FF));
        blitter.flush_to_framebuffer(&mut fb);

        // (0,0) should be red (not keyed)
        let px00 = fb.get_pixel(0, 0).unwrap();
        let r = (px00 >> 16) & 0xFF;
        assert!(r > 200, "expected red at (0,0), got r={r}");

        // (3,3) should still be blue (keyed pixel preserved background)
        let px33 = fb.get_pixel(3, 3).unwrap();
        let b = px33 & 0xFF;
        assert!(b > 200, "expected blue at (3,3), got b={b}");
    }

    #[test]
    fn flush_alpha_blends_semitransparent() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);
        let mut fb = abrash_core::framebuffer::Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF00_00FF); // blue background

        // 4x4, 50% transparent red
        let mut tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                tex.set_pixel(x, y, 0x80FF_0000); // 50% alpha red
            }
        }
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        blitter.queue(atlas, src, 0, 0, BlitMode::Alpha);
        blitter.flush_to_framebuffer(&mut fb);

        let px = fb.get_pixel(0, 0).unwrap();
        let r = (px >> 16) & 0xFF;
        let b = px & 0xFF;
        // Should be blend of red and blue — both channels mid-range
        assert!(r > 60 && r < 220, "red should be mid-range: {r}");
        assert!(b > 60 && b < 220, "blue should be mid-range: {b}");
    }

    #[test]
    fn flush_retains_instances_capacity() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);

        let tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        blitter.queue(atlas, src, 0, 0, BlitMode::Opaque);
        blitter.queue(atlas, src, 10, 10, BlitMode::Alpha);

        let _ = blitter.flush_and_readback();

        // Internal capacity should be retained, not re-allocated to 0
        assert!(blitter.instances.capacity() >= 2);
    }

    #[test]
    fn flush_multiple_sprites_draw_order() {
        let gpu = headless_device();
        let mut blitter = GpuBlitter::new(&gpu, 64, 64);
        let mut fb = abrash_core::framebuffer::Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF00_0000);

        // Red and green 4x4 textures
        let mut red_tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        let mut green_tex = abrash_core::texture::Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                red_tex.set_pixel(x, y, 0xFFFF_0000);
                green_tex.set_pixel(x, y, 0xFF00_FF00);
            }
        }
        let red_atlas = blitter.upload_atlas(&red_tex);
        let green_atlas = blitter.upload_atlas(&green_tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        // Queue red first, green second at same position
        blitter.queue(red_atlas, src, 0, 0, BlitMode::Opaque);
        blitter.queue(green_atlas, src, 0, 0, BlitMode::Opaque);
        blitter.flush_to_framebuffer(&mut fb);

        // Green should be on top (drawn last)
        let px = fb.get_pixel(0, 0).unwrap();
        let g = (px >> 8) & 0xFF;
        assert!(g > 200, "expected green on top, got g={g}");
    }
}
