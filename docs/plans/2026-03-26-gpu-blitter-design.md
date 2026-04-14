# GPU Batched 2D Blitter Design

**Date**: 2026-03-26
**Status**: Approved
**Author**: Mark + Claude
**Depends on**: [2D Blitter Design](2026-03-26-2d-blitter-design.md) (CPU blitter, implemented)

## Goal

Add a GPU-accelerated 2D sprite blitter that batches hundreds of sprites into minimal draw calls via wgpu instanced quad rendering. Follows the same CPU-first/GPU-when-needed pattern as the 3D rasterizer. The GPU path wins at ~100-200+ sprites per frame where batch amortization overwhelms PCIe transfer overhead.

## Architecture Overview

```
Caller code
    │
    ├── gpu_blitter.upload_atlas(&texture)        # one-time per atlas
    ├── gpu_blitter.queue(atlas, src, x, y, mode) # per sprite per frame
    ├── gpu_blitter.queue_normalized(...)          # resolution-independent variant
    │
    ├── gpu_blitter.flush_to_screen(&surface)     # direct presentation (fastest)
    └── gpu_blitter.flush_to_framebuffer(&mut fb)  # readback for CPU compositing
```

## API Surface

New module: `crates/abrash-gpu-render/src/blitter.rs`

### Types

```rust
/// Handle to a GPU-resident atlas texture. Returned by upload_atlas().
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtlasHandle(u32);

/// Blend mode for a queued sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlitMode {
    /// Direct copy, no blending. Fastest.
    Opaque,
    /// Skip pixels matching key color.
    ColorKey(u32),
    /// Per-pixel alpha blend using source alpha channel.
    Alpha,
}

/// Coordinate space for destination position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoordMode {
    /// Pixel coordinates, top-left origin. Matches CPU blitter.
    Pixel,
    /// Normalized 0.0..1.0, top-left origin. Resolution-independent.
    Normalized,
}

/// Internal: one sprite command in the batch.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteInstance {
    // Source rect in texels (atlas UV will be computed in vertex shader)
    src_x: f32,
    src_y: f32,
    src_w: f32,
    src_h: f32,
    // Destination position
    dst_x: f32,
    dst_y: f32,
    // Destination size (same as src_w/src_h for pixel mode)
    dst_w: f32,
    dst_h: f32,
    // Atlas texture dimensions (for UV computation)
    atlas_w: f32,
    atlas_h: f32,
    // Blend mode: 0=opaque, 1=colorkey, 2=alpha
    blend_mode: u32,
    // Color key (only used when blend_mode==1)
    color_key: u32,
}
```

### GpuBlitter

```rust
pub struct GpuBlitter {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Two render pipelines (split on blend state)
    opaque_pipeline: wgpu::RenderPipeline,   // opaque + colorkey (no blend)
    alpha_pipeline: wgpu::RenderPipeline,     // src_alpha blend

    // Unit quad geometry (shared across all sprites)
    quad_vertex_buffer: wgpu::Buffer,   // 4 vertices
    quad_index_buffer: wgpu::Buffer,    // 6 indices

    // Per-frame sprite batch
    sprite_commands: Vec<SpriteInstance>,

    // Uploaded atlas textures
    atlases: Vec<GpuAtlas>,

    // Screen dimensions uniform
    screen_uniform_buffer: wgpu::Buffer,

    // Render target
    render_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
}

impl GpuBlitter {
    /// Create a new GPU blitter. Requires a GpuDevice.
    pub fn new(device: &GpuDevice, width: u32, height: u32) -> Self;

    /// Upload a CPU Texture as a GPU atlas. Returns handle for queue().
    /// Call once per atlas texture, reuse the handle every frame.
    pub fn upload_atlas(&mut self, texture: &Texture) -> AtlasHandle;

    /// Queue a sprite for rendering (pixel coordinates).
    /// Coordinates match the CPU blitter: i32, top-left origin.
    pub fn queue(
        &mut self,
        atlas: AtlasHandle,
        src: SrcRect,
        dst_x: i32,
        dst_y: i32,
        mode: BlitMode,
    );

    /// Queue a sprite (normalized coordinates, 0.0..1.0).
    /// Resolution-independent positioning.
    pub fn queue_normalized(
        &mut self,
        atlas: AtlasHandle,
        src: SrcRect,
        dst_x: f32,
        dst_y: f32,
        mode: BlitMode,
    );

    /// Render all queued sprites directly to a window surface.
    /// Fastest path: no CPU readback.
    pub fn flush_to_screen(&mut self, surface: &GpuSurface);

    /// Render all queued sprites, then readback to a CPU Framebuffer.
    /// Use when mixing GPU and CPU rendering.
    pub fn flush_to_framebuffer(&mut self, fb: &mut Framebuffer);

    /// Discard all queued sprites without rendering.
    pub fn clear(&mut self);
}
```

## Rendering Strategy: Instanced Quads

Each sprite is one instance of a unit quad `(0,0)→(1,1)`. The vertex shader transforms it to screen position using per-instance data from a storage buffer.

### Unit Quad (shared, uploaded once)

```
Vertices: (0,0), (1,0), (1,1), (0,1)
Indices:  0,1,2, 0,2,3
```

### Per-Frame Flow

1. **Queue phase** (CPU): `queue()` appends `SpriteInstance` to a `Vec`
2. **Sort phase** (CPU): Sort by `(atlas_id, blend_mode)` to minimize state changes
3. **Upload phase**: Write `Vec<SpriteInstance>` to a GPU storage buffer
4. **Draw phase**: For each (atlas, blend_mode) group:
   - Bind appropriate pipeline (opaque or alpha)
   - Bind atlas texture
   - Draw instanced: 6 indices x N instances
5. **Output phase**: Present to screen or readback to CPU

### Draw Call Count

Worst case: `num_atlases × 2` draw calls (one per pipeline per atlas).
Typical case: 1-3 draw calls (one atlas, 1-2 blend modes active).

## Shaders (WGSL)

### Vertex Shader (~15 lines)

```wgsl
struct ScreenUniforms {
    width: f32,
    height: f32,
};

struct SpriteInstance {
    src_x: f32, src_y: f32, src_w: f32, src_h: f32,
    dst_x: f32, dst_y: f32, dst_w: f32, dst_h: f32,
    atlas_w: f32, atlas_h: f32,
    blend_mode: u32, color_key: u32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniforms;
@group(0) @binding(1) var<storage, read> sprites: array<SpriteInstance>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance: u32,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VsOut {
    let sprite = sprites[iid];

    // Unit quad corners
    let corners = array<vec2<f32>, 4>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)
    );
    let idx_order = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    let corner = corners[idx_order[vid]];

    // Pixel position on screen
    let px = sprite.dst_x + corner.x * sprite.dst_w;
    let py = sprite.dst_y + corner.y * sprite.dst_h;

    // Pixel → NDC
    let ndc_x = (px / screen.width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.height) * 2.0;

    // Atlas UVs
    let u = (sprite.src_x + corner.x * sprite.src_w) / sprite.atlas_w;
    let v = (sprite.src_y + corner.y * sprite.src_h) / sprite.atlas_h;

    var out: VsOut;
    out.pos = vec4(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = vec2(u, v);
    out.instance = iid;
    return out;
}
```

### Fragment Shader — Opaque/ColorKey Pipeline (~15 lines)

```wgsl
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_opaque(in: VsOut) -> @location(0) vec4<f32> {
    let sprite = sprites[in.instance];
    let color = textureSample(atlas_tex, atlas_sampler, in.uv);

    // ColorKey: discard pixels matching key color
    if (sprite.blend_mode == 1u) {
        let r = u32(color.r * 255.0);
        let g = u32(color.g * 255.0);
        let b = u32(color.b * 255.0);
        let pixel = (0xFFu << 24u) | (r << 16u) | (g << 8u) | b;
        if (pixel == sprite.color_key) {
            discard;
        }
    }

    return vec4(color.rgb, 1.0);
}
```

### Fragment Shader — Alpha Pipeline (~5 lines)

```wgsl
@fragment
fn fs_alpha(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas_tex, atlas_sampler, in.uv);
    return color;  // GPU blend state handles src_alpha compositing
}
```

Alpha blending is handled by wgpu's fixed-function blend state, not the shader.

## Pipeline Configuration

### Opaque + ColorKey Pipeline

```rust
BlendState {
    color: BlendComponent::REPLACE,  // write directly
    alpha: BlendComponent::REPLACE,
}
```

### Alpha Pipeline

```rust
BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent::OVER,
}
```

## Output Paths

### flush_to_screen (Direct Presentation)

1. Acquire swapchain texture from `GpuSurface`
2. Render pass writes directly to swapchain texture
3. Submit + present
4. Zero CPU readback — fastest path

### flush_to_framebuffer (CPU Readback)

1. Render to internal `render_texture` (RGBA8, RENDER_ATTACHMENT | COPY_SRC)
2. Copy `render_texture` → readback buffer (MAP_READ, 256-byte row alignment)
3. Map readback buffer, copy pixels to `Framebuffer`
4. ~2ms overhead at 1080p (same as measured in GPU Hi-Z experiments)

## Coordinate Systems

### Pixel Coordinates (queue)

Identical to CPU blitter: `i32`, top-left origin, framebuffer pixels.
Vertex shader divides by screen dimensions to get NDC.

### Normalized Coordinates (queue_normalized)

`f32` in range `0.0..1.0`, top-left origin. Sprite size is scaled by screen dimensions:

```rust
// In queue_normalized():
let dst_w = src.w as f32 * (screen_w as f32);  // or caller specifies?
```

Design decision: normalized mode scales position only, sprite size remains in pixels.
This matches how most 2D frameworks handle resolution independence.

## Atlas Management

### Upload

```rust
let atlas_id = gpu_blitter.upload_atlas(&texture);
```

- Creates `wgpu::Texture` with `TEXTURE_BINDING | COPY_DST`
- Writes pixel data via `queue.write_texture()`
- Creates texture view + bind group
- Returns `AtlasHandle(index)` for use in `queue()`
- One-time cost per atlas; reuse handle every frame

### No Auto-Packing

Caller owns atlas layout (same as CPU blitter taking `Texture` + `SrcRect`).
Auto-packing (bin-packing, defragmentation) is out of scope.

## Performance Expectations

Based on measured GPU overhead from Hi-Z and binning experiments:

| Scenario | Estimated Time | Notes |
|----------|---------------|-------|
| Setup + 1 draw call | ~0.5ms | Pipeline bind + instance upload |
| Per additional draw call | ~0.1ms | State change + draw |
| 100 sprites (1 atlas, 1 mode) | ~0.7ms | 1 draw call |
| 500 sprites (1 atlas, 2 modes) | ~1.0ms | 2 draw calls |
| 1000 sprites (2 atlases, 2 modes) | ~1.5ms | 4 draw calls |
| Readback to CPU (1080p) | ~2.0ms | PCIe download |
| Readback to CPU (4K) | ~8.0ms | PCIe download |

**CPU comparison** (for 500 sprites, 64x64, mixed alpha):
- CPU alpha blit: 500 × 64×64 / 440Mpix/s ≈ **4.7ms**
- GPU batched: ~1.0ms draw + 0ms (screen) or 2ms (readback)
- **GPU wins by 2-4x at 500+ sprites**

## Module Structure

```
crates/abrash-gpu-render/src/
├── blitter.rs          # GpuBlitter, AtlasHandle, BlitMode, SpriteInstance
├── blitter_shaders.rs  # WGSL shader source strings (or embedded in blitter.rs)
├── lib.rs              # add: pub mod blitter;
├── device.rs           # existing GpuDevice (unchanged)
├── surface.rs          # existing GpuSurface (unchanged)
└── capture.rs          # existing GpuCaptureTarget (reuse readback pattern)

Re-export: abrash-gpu-render::blitter → abrash::gpu_blitter (feature-gated)
```

## Testing Strategy

### Correctness (headless capture)
- Single opaque sprite → verify exact pixel placement
- Single colorkey sprite → verify transparent pixels untouched
- Single alpha sprite → verify blend matches CPU blitter output
- Multiple overlapping sprites → verify draw order (back-to-front)
- Atlas subregion → verify correct UV sampling

### Coordinate Systems
- Pixel coords → same output as CPU blitter (pixel-identical)
- Normalized coords → verify position scales with resolution

### Output Paths
- flush_to_screen → verify no crash, pixels visible (manual or screenshot)
- flush_to_framebuffer → verify readback matches headless capture

### Performance (criterion benchmarks)
- Batch sizes: 10, 50, 100, 500, 1000 sprites
- Compare GPU flush_to_screen vs CPU blitter
- Compare GPU flush_to_framebuffer vs CPU blitter (includes readback)
- Report crossover point

### Edge Cases
- Zero sprites queued → flush is no-op
- Sprite fully off-screen → clipped by GPU (or pre-clipped on CPU side)
- Atlas handle reuse across frames
- Multiple atlases in one batch

## Integration with CPU Blitter

The GPU and CPU blitters share the same `SrcRect` type and pixel-coordinate convention.
Callers can switch between them based on sprite count:

```rust
if sprite_count > GPU_THRESHOLD {
    for sprite in sprites {
        gpu_blitter.queue(sprite.atlas, sprite.src, sprite.x, sprite.y, sprite.mode);
    }
    gpu_blitter.flush_to_screen(&surface);
} else {
    for sprite in sprites {
        blit_alpha(&mut fb, &sprite.texture, sprite.src, sprite.x, sprite.y);
    }
}
```

## Estimated Size

- `blitter.rs`: ~400-500 lines (struct, pipeline setup, queue/flush logic)
- WGSL shaders: ~50 lines (vertex + 2 fragment shaders)
- Tests: ~200 lines
- Benchmarks: ~100 lines
- Total: ~750-850 lines
