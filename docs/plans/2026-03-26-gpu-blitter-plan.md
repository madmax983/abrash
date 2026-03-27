# GPU Batched 2D Blitter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a GPU-accelerated 2D sprite blitter using wgpu instanced quads with immediate-mode batching API.

**Architecture:** `GpuBlitter` lives in `abrash-gpu-render`, queues sprite commands per frame, sorts by (atlas, blend_mode), issues 1-3 instanced draw calls per frame via two wgpu render pipelines (opaque+colorkey, alpha). Two output paths: direct screen presentation and CPU framebuffer readback.

**Tech Stack:** wgpu 29.0.0, bytemuck, WGSL shaders (embedded), existing `GpuDevice`/`GpuSurface`/`GpuCaptureTarget` infrastructure.

---

### Task 1: Types and Data Structures

**Files:**
- Create: `crates/abrash-gpu-render/src/blitter.rs`
- Modify: `crates/abrash-gpu-render/src/lib.rs:28` (add `pub mod blitter;`)

**Step 1: Write failing test — AtlasHandle, BlitMode, SpriteInstance types exist**

In `crates/abrash-gpu-render/src/blitter.rs`:

```rust
//! GPU-accelerated 2D sprite blitter with instanced quad rendering.

use abrash_core::blitter::SrcRect;
use bytemuck::{Pod, Zeroable};

/// Handle to a GPU-resident atlas texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtlasHandle(pub(crate) u32);

/// Blend mode for a queued sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlitMode {
    /// Direct copy, no blending.
    Opaque,
    /// Skip pixels matching key color (0xAARRGGBB).
    ColorKey(u32),
    /// Per-pixel alpha blend using source alpha channel.
    Alpha,
}

/// Coordinate mode for sprite positioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoordMode {
    /// Pixel coordinates, top-left origin. Matches CPU blitter.
    Pixel,
    /// Normalized 0.0..1.0, top-left origin.
    Normalized,
}

/// Per-instance GPU data for one sprite. Uploaded to storage buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
    /// Source rect in texels.
    pub src_x: f32,
    pub src_y: f32,
    pub src_w: f32,
    pub src_h: f32,
    /// Destination position in pixels (or normalized, converted before upload).
    pub dst_x: f32,
    pub dst_y: f32,
    /// Destination size in pixels.
    pub dst_w: f32,
    pub dst_h: f32,
    /// Atlas texture dimensions for UV computation.
    pub atlas_w: f32,
    pub atlas_h: f32,
    /// 0=opaque, 1=colorkey, 2=alpha.
    pub blend_mode: u32,
    /// Color key value (only used when blend_mode==1).
    pub color_key: u32,
}

/// Internal: a queued sprite command before sorting.
#[derive(Debug, Clone)]
pub(crate) struct SpriteCommand {
    pub atlas: AtlasHandle,
    pub instance: SpriteInstance,
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
        let key = BlitMode::ColorKey(0xFF00FF00);
        let alpha = BlitMode::Alpha;
        assert_ne!(opaque, key);
        assert_ne!(key, alpha);
    }

    #[test]
    fn sprite_instance_is_pod() {
        // Verify SpriteInstance is 48 bytes (12 × f32/u32)
        assert_eq!(std::mem::size_of::<SpriteInstance>(), 48);
    }

    #[test]
    fn sprite_instance_zeroed() {
        let inst = SpriteInstance::zeroed();
        assert_eq!(inst.src_x, 0.0);
        assert_eq!(inst.blend_mode, 0);
    }
}
```

**Step 2: Register the module**

In `crates/abrash-gpu-render/src/lib.rs`, add after line 28 (after `pub mod temporal;`):

```rust
pub mod blitter;
```

**Step 3: Run tests to verify they pass**

Run: `cargo test -p abrash-gpu-render --lib blitter`
Expected: 4 tests pass

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs crates/abrash-gpu-render/src/lib.rs
git commit -m "feat(gpu-blitter): add types — AtlasHandle, BlitMode, SpriteInstance"
```

---

### Task 2: WGSL Shaders

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write the WGSL shader source as embedded constants**

Add to `blitter.rs` after the type definitions:

```rust
/// WGSL shader source for the GPU blitter.
///
/// Vertex shader: transforms unit quad per-instance using SpriteInstance data.
/// Fragment shaders: fs_opaque (opaque + colorkey), fs_alpha (alpha blend).
pub(crate) const BLITTER_SHADER_SRC: &str = r"
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
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance_id: u32,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VsOut {
    let sprite = sprites[iid];

    // Unit quad corners: 0=(0,0), 1=(1,0), 2=(1,1), 3=(0,1)
    // Index order for two triangles: 0,1,2, 0,2,3
    let quad_idx = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    let corners = array<vec2<f32>, 4>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)
    );
    let corner = corners[quad_idx[vid]];

    // Pixel position on screen
    let px = sprite.dst_x + corner.x * sprite.dst_w;
    let py = sprite.dst_y + corner.y * sprite.dst_h;

    // Pixel coords -> NDC (top-left origin)
    let ndc_x = (px / screen.width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / screen.height) * 2.0;

    // Atlas UV coordinates
    let u = (sprite.src_x + corner.x * sprite.src_w) / sprite.atlas_w;
    let v = (sprite.src_y + corner.y * sprite.src_h) / sprite.atlas_h;

    var out: VsOut;
    out.pos = vec4(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = vec2(u, v);
    out.instance_id = iid;
    return out;
}

@fragment
fn fs_opaque(in: VsOut) -> @location(0) vec4<f32> {
    let sprite = sprites[in.instance_id];
    let color = textureSample(atlas_tex, atlas_sampler, in.uv);

    // ColorKey: discard pixels matching key color
    if (sprite.blend_mode == 1u) {
        let r = u32(color.r * 255.0 + 0.5);
        let g = u32(color.g * 255.0 + 0.5);
        let b = u32(color.b * 255.0 + 0.5);
        let pixel = (0xFFu << 24u) | (r << 16u) | (g << 8u) | b;
        if (pixel == sprite.color_key) {
            discard;
        }
    }

    return vec4(color.rgb, 1.0);
}

@fragment
fn fs_alpha(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(atlas_tex, atlas_sampler, in.uv);
}
";
```

**Step 2: Write test that shader source is non-empty and contains entry points**

```rust
#[test]
fn shader_source_contains_entry_points() {
    assert!(BLITTER_SHADER_SRC.contains("fn vs_main"));
    assert!(BLITTER_SHADER_SRC.contains("fn fs_opaque"));
    assert!(BLITTER_SHADER_SRC.contains("fn fs_alpha"));
    assert!(BLITTER_SHADER_SRC.contains("struct SpriteInstance"));
}
```

**Step 3: Run tests**

Run: `cargo test -p abrash-gpu-render --lib blitter`
Expected: 5 tests pass

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add WGSL shaders — vertex + opaque/alpha fragment"
```

---

### Task 3: GpuBlitter Core — Construction, Atlas Upload, Queue

This is the main struct and CPU-side logic. No GPU pipeline creation yet (that requires a device).

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write failing tests for queue behavior**

```rust
#[test]
fn queue_builds_sprite_commands() {
    let mut commands: Vec<SpriteCommand> = Vec::new();
    let atlas = AtlasHandle(0);
    let src = SrcRect { x: 0, y: 0, w: 32, h: 32 };

    // Simulate queue() logic
    let inst = SpriteInstance {
        src_x: 0.0, src_y: 0.0, src_w: 32.0, src_h: 32.0,
        dst_x: 100.0, dst_y: 50.0, dst_w: 32.0, dst_h: 32.0,
        atlas_w: 256.0, atlas_h: 256.0,
        blend_mode: 0, color_key: 0,
    };
    commands.push(SpriteCommand { atlas, instance: inst });

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].atlas, atlas);
    assert_eq!(commands[0].instance.dst_x, 100.0);
}

#[test]
fn queue_normalized_converts_position() {
    let screen_w: f32 = 1920.0;
    let screen_h: f32 = 1080.0;
    let norm_x: f32 = 0.5;
    let norm_y: f32 = 0.25;

    let px_x = norm_x * screen_w;
    let px_y = norm_y * screen_h;

    assert_eq!(px_x, 960.0);
    assert_eq!(px_y, 270.0);
}

#[test]
fn sort_commands_by_atlas_then_blend() {
    let mut commands = vec![
        SpriteCommand {
            atlas: AtlasHandle(1),
            instance: SpriteInstance { blend_mode: 2, ..SpriteInstance::zeroed() },
        },
        SpriteCommand {
            atlas: AtlasHandle(0),
            instance: SpriteInstance { blend_mode: 0, ..SpriteInstance::zeroed() },
        },
        SpriteCommand {
            atlas: AtlasHandle(0),
            instance: SpriteInstance { blend_mode: 2, ..SpriteInstance::zeroed() },
        },
        SpriteCommand {
            atlas: AtlasHandle(1),
            instance: SpriteInstance { blend_mode: 0, ..SpriteInstance::zeroed() },
        },
    ];

    commands.sort_by(|a, b| {
        a.atlas.0.cmp(&b.atlas.0)
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
```

**Step 2: Implement BlitMode helper methods**

```rust
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
```

**Step 3: Run tests**

Run: `cargo test -p abrash-gpu-render --lib blitter`
Expected: 10 tests pass

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add BlitMode helpers, queue logic, command sorting"
```

---

### Task 4: GpuBlitter Struct and Pipeline Setup

This task creates the actual `GpuBlitter` struct with wgpu pipeline initialization. Requires a GPU device, so tests use headless `GpuDevice`.

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write failing test — GpuBlitter::new() succeeds on headless device**

```rust
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
    }
}
```

**Step 2: Implement GpuBlitter struct with pipeline creation**

This is the largest implementation step. The `GpuBlitter` struct:

1. Creates the WGSL shader module
2. Creates bind group layouts: group 0 (screen uniform + sprite storage), group 1 (atlas texture + sampler)
3. Creates two render pipelines: opaque (BlendState::REPLACE) and alpha (SrcAlpha blend)
4. Creates the screen uniform buffer (8 bytes: width, height)
5. Creates the render target texture (Rgba8Unorm, RENDER_ATTACHMENT | COPY_SRC)
6. Creates the readback buffer for flush_to_framebuffer

```rust
/// GPU-resident atlas texture.
pub(crate) struct GpuAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

/// GPU-accelerated 2D sprite blitter.
///
/// Batches sprites into minimal draw calls using instanced quad rendering.
/// Two render pipelines: opaque+colorkey (no blend state) and alpha (src-over blend).
pub struct GpuBlitter {
    // wgpu handles (borrowed from GpuDevice at creation, stored as owned)
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Render pipelines
    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,

    // Bind group layouts
    frame_bind_group_layout: wgpu::BindGroupLayout,
    atlas_bind_group_layout: wgpu::BindGroupLayout,

    // Sampler (shared by all atlases — nearest-neighbor for pixel art)
    sampler: wgpu::Sampler,

    // Screen dimensions
    width: u32,
    height: u32,
    screen_uniform_buffer: wgpu::Buffer,

    // Render target (for readback path)
    render_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    readback_buffer: wgpu::Buffer,

    // Per-frame state
    commands: Vec<SpriteCommand>,
    atlases: Vec<GpuAtlas>,
}
```

Key implementation notes for the plan executor:

- Use `wgpu::ShaderModuleDescriptor` with `BLITTER_SHADER_SRC`
- Vertex state: **no vertex buffers** (geometry generated in shader via `vertex_index` and `instance_index`)
- Pipeline layout: two bind groups (group 0: frame data, group 1: atlas texture)
- Storage buffer for sprite instances is created dynamically at flush time (sized to command count)
- Render target format: `Rgba8Unorm` (matches `GpuCaptureTarget`)
- Readback buffer: use `aligned_bytes_per_row()` from `capture.rs`

**Step 3: Run test**

Run: `cargo test -p abrash-gpu-render --lib blitter::gpu_tests`
Expected: 1 test passes (GPU blitter created successfully)

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add GpuBlitter struct with wgpu pipeline setup"
```

---

### Task 5: Atlas Upload

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write failing test — upload_atlas returns handle, stores texture**

```rust
#[test]
fn upload_atlas_returns_handle() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 800, 600);

    let tex = Texture::new(64, 64);
    let handle = blitter.upload_atlas(&tex);
    assert_eq!(handle, AtlasHandle(0));

    let handle2 = blitter.upload_atlas(&tex);
    assert_eq!(handle2, AtlasHandle(1));
}
```

**Step 2: Implement upload_atlas**

```rust
impl GpuBlitter {
    pub fn upload_atlas(&mut self, texture: &abrash_core::texture::Texture) -> AtlasHandle {
        let width = texture.width();
        let height = texture.height();
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

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

        // Convert 0xAARRGGBB pixels to RGBA bytes for wgpu
        let pixels = texture.pixels();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for &px in pixels {
            rgba.push(((px >> 16) & 0xFF) as u8); // R
            rgba.push(((px >> 8) & 0xFF) as u8);  // G
            rgba.push((px & 0xFF) as u8);          // B
            rgba.push(((px >> 24) & 0xFF) as u8);  // A
        }

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
            label: Some("Blitter Atlas BindGroup"),
            layout: &self.atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });

        let index = self.atlases.len() as u32;
        self.atlases.push(GpuAtlas { texture: gpu_texture, view, bind_group, width, height });
        AtlasHandle(index)
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p abrash-gpu-render --lib blitter::gpu_tests`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add atlas upload — Texture to GPU texture + bind group"
```

---

### Task 6: Queue and Clear Methods

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write failing tests — queue, queue_normalized, clear**

```rust
#[test]
fn queue_and_clear() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 800, 600);
    let tex = Texture::new(64, 64);
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 32, h: 32 };

    blitter.queue(atlas, src, 100, 50, BlitMode::Opaque);
    blitter.queue(atlas, src, 200, 50, BlitMode::Alpha);
    assert_eq!(blitter.queued_count(), 2);

    blitter.clear();
    assert_eq!(blitter.queued_count(), 0);
}

#[test]
fn queue_normalized_scales_to_pixels() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 1920, 1080);
    let tex = Texture::new(64, 64);
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 32, h: 32 };

    blitter.queue_normalized(atlas, src, 0.5, 0.25, BlitMode::Opaque);
    assert_eq!(blitter.queued_count(), 1);
    // Internal: dst_x should be 960.0, dst_y should be 270.0
}

#[test]
fn queue_negative_coords() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 800, 600);
    let tex = Texture::new(64, 64);
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 32, h: 32 };

    // GPU handles clipping via rasterizer — negative coords are valid
    blitter.queue(atlas, src, -10, -20, BlitMode::Opaque);
    assert_eq!(blitter.queued_count(), 1);
}
```

**Step 2: Implement queue, queue_normalized, clear, queued_count**

```rust
impl GpuBlitter {
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

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    #[must_use]
    pub fn queued_count(&self) -> usize {
        self.commands.len()
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p abrash-gpu-render --lib blitter::gpu_tests`
Expected: 5 tests pass

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add queue, queue_normalized, clear methods"
```

---

### Task 7: Flush — Render to Internal Texture

The core rendering logic: sort commands, upload instance buffer, issue draw calls.

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write failing test — flush renders a sprite to internal texture**

```rust
#[test]
fn flush_renders_opaque_sprite() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 64, 64);

    // Create a 4x4 solid red texture
    let mut tex = Texture::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            tex.set_pixel(x, y, 0xFFFF0000); // solid red
        }
    }
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

    blitter.queue(atlas, src, 0, 0, BlitMode::Opaque);

    // Flush and readback
    let pixels = blitter.flush_and_readback();

    // Check top-left 4x4 region is red (RGBA)
    let stride = 64; // pixels per row
    for y in 0..4u32 {
        for x in 0..4u32 {
            let idx = ((y * stride + x) * 4) as usize;
            let r = pixels[idx];
            let g = pixels[idx + 1];
            let b = pixels[idx + 2];
            assert!(r > 200, "red channel at ({x},{y}): {r}");
            assert!(g < 50, "green channel at ({x},{y}): {g}");
            assert!(b < 50, "blue channel at ({x},{y}): {b}");
        }
    }
}
```

**Step 2: Implement the internal render method**

The `flush_internal()` method:

1. If no commands queued, return early
2. Sort commands by `(atlas.0, blend_mode)` — minimizes pipeline/bind group switches
3. Create a dynamic storage buffer from sorted `SpriteInstance` data
4. Create a frame bind group (screen uniform + sprite storage buffer)
5. Begin render pass on `render_texture` (clear to transparent black)
6. Iterate sorted commands in groups by `(atlas, pipeline)`:
   - Bind appropriate pipeline (opaque or alpha)
   - Bind atlas bind group
   - Draw `6 * count` vertices (6 per quad), instanced with offset
7. Submit command buffer
8. Clear the command list

The `flush_and_readback()` helper:
1. Call `flush_internal()`
2. Copy render texture → readback buffer
3. Map readback buffer, extract RGBA pixels
4. Return `Vec<u8>`

The `flush_to_framebuffer()` method:
1. Call `flush_and_readback()`
2. Convert RGBA → 0xAARRGGBB, write to `Framebuffer` pixels

**Step 3: Run test**

Run: `cargo test -p abrash-gpu-render --lib blitter::gpu_tests::flush_renders_opaque_sprite`
Expected: PASS — 4x4 red pixels at top-left

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add flush — sort, upload instances, instanced draw"
```

---

### Task 8: flush_to_framebuffer

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write failing test — flush_to_framebuffer writes correct pixels**

```rust
#[test]
fn flush_to_framebuffer_writes_pixels() {
    use abrash_core::framebuffer::Framebuffer;

    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 64, 64);
    let mut fb = Framebuffer::new(64, 64).unwrap();
    fb.clear(0xFF000000); // black

    // 4x4 solid green texture
    let mut tex = Texture::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            tex.set_pixel(x, y, 0xFF00FF00);
        }
    }
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };
    blitter.queue(atlas, src, 10, 10, BlitMode::Opaque);

    blitter.flush_to_framebuffer(&mut fb);

    // Check pixel at (10,10) — should be green
    let px = fb.get_pixel(10, 10).unwrap();
    let r = (px >> 16) & 0xFF;
    let g = (px >> 8) & 0xFF;
    let b = px & 0xFF;
    assert!(g > 200, "green channel: {g}");
    assert!(r < 50, "red channel: {r}");
    assert!(b < 50, "blue channel: {b}");

    // Check pixel at (0,0) — should still be black
    let px0 = fb.get_pixel(0, 0).unwrap();
    assert_eq!(px0 & 0x00FFFFFF, 0x000000);
}
```

**Step 2: Implement flush_to_framebuffer**

```rust
impl GpuBlitter {
    pub fn flush_to_framebuffer(&mut self, fb: &mut abrash_core::framebuffer::Framebuffer) {
        let rgba = self.flush_and_readback();
        if rgba.is_empty() { return; }

        let padded_bpr = crate::capture::aligned_bytes_per_row(self.width) as usize;
        let fb_pixels = fb.pixels_mut();
        let fb_w = self.width as usize;

        for y in 0..self.height as usize {
            let src_row = &rgba[y * padded_bpr..y * padded_bpr + fb_w * 4];
            for x in 0..fb_w {
                let i = x * 4;
                let r = src_row[i] as u32;
                let g = src_row[i + 1] as u32;
                let b = src_row[i + 2] as u32;
                let a = src_row[i + 3] as u32;
                fb_pixels[y * fb_w + x] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }
}
```

**Step 3: Run test**

Run: `cargo test -p abrash-gpu-render --lib blitter::gpu_tests::flush_to_framebuffer`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add flush_to_framebuffer — GPU render + CPU readback"
```

---

### Task 9: flush_to_screen (Surface Presentation)

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Note:** This path can only be tested manually or via integration tests with a window. Write the implementation matching the flush_internal pattern but targeting the surface texture.

**Step 1: Implement flush_to_screen**

```rust
#[cfg(feature = "windowed")]
impl GpuBlitter {
    pub fn flush_to_screen(&mut self, surface: &crate::surface::GpuSurface) {
        if self.commands.is_empty() { return; }

        let frame = surface.surface.get_current_texture()
            .expect("failed to acquire swapchain texture");
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.flush_to_view(&view);

        frame.present();
    }
}
```

Factor out `flush_to_view(&self, target: &wgpu::TextureView)` as the shared render logic used by both `flush_to_screen` and `flush_and_readback`.

**Step 2: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "feat(gpu-blitter): add flush_to_screen — direct surface presentation"
```

---

### Task 10: Re-export and Integration

**Files:**
- Modify: `src/gpu_render.rs` (add blitter re-exports)
- Modify: `crates/abrash-gpu-render/src/lib.rs` (already done in Task 1)

**Step 1: Add re-exports**

In `src/gpu_render.rs`, add:

```rust
pub use abrash_gpu_render::blitter::{GpuBlitter, AtlasHandle, BlitMode};
```

**Step 2: Run full workspace tests**

Run: `cargo test -p abrash-gpu-render`
Expected: All blitter tests pass + all existing GPU render tests pass

Run: `cargo test --features gpu-render --lib gpu_render`
Expected: Re-exports resolve

**Step 3: Commit**

```bash
git add src/gpu_render.rs
git commit -m "feat(gpu-blitter): re-export GpuBlitter, AtlasHandle, BlitMode from abrash::gpu_render"
```

---

### Task 11: Alpha Blend and ColorKey Integration Tests

**Files:**
- Modify: `crates/abrash-gpu-render/src/blitter.rs`

**Step 1: Write integration tests for all three blend modes**

```rust
#[test]
fn flush_colorkey_skips_key_pixels() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 64, 64);
    let mut fb = Framebuffer::new(64, 64).unwrap();
    fb.clear(0xFF0000FF); // blue background

    // 4x4 texture: top-left 2x2 red, bottom-right 2x2 magenta (key)
    let mut tex = Texture::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            if x < 2 && y < 2 {
                tex.set_pixel(x, y, 0xFFFF0000); // red
            } else {
                tex.set_pixel(x, y, 0xFFFF00FF); // magenta = key
            }
        }
    }
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

    blitter.queue(atlas, src, 0, 0, BlitMode::ColorKey(0xFFFF00FF));
    blitter.flush_to_framebuffer(&mut fb);

    // (0,0) should be red
    let px00 = fb.get_pixel(0, 0).unwrap();
    assert!((px00 >> 16) & 0xFF > 200, "expected red at (0,0)");

    // (3,3) should be blue (key pixel skipped, background preserved)
    let px33 = fb.get_pixel(3, 3).unwrap();
    assert!((px33 & 0xFF) > 200, "expected blue at (3,3)");
}

#[test]
fn flush_alpha_blends_semitransparent() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 64, 64);
    let mut fb = Framebuffer::new(64, 64).unwrap();
    fb.clear(0xFF0000FF); // blue background

    // 4x4 50% transparent red
    let mut tex = Texture::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            tex.set_pixel(x, y, 0x80FF0000); // 50% red
        }
    }
    let atlas = blitter.upload_atlas(&tex);
    let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

    blitter.queue(atlas, src, 0, 0, BlitMode::Alpha);
    blitter.flush_to_framebuffer(&mut fb);

    // (0,0) should be a blend of red and blue
    let px = fb.get_pixel(0, 0).unwrap();
    let r = (px >> 16) & 0xFF;
    let b = px & 0xFF;
    // With 50% alpha blend: r ~128, b ~128
    assert!(r > 80 && r < 200, "red channel should be mid-range: {r}");
    assert!(b > 80 && b < 200, "blue channel should be mid-range: {b}");
}

#[test]
fn flush_multiple_sprites_draw_order() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 64, 64);
    let mut fb = Framebuffer::new(64, 64).unwrap();
    fb.clear(0xFF000000);

    // Red sprite, then green sprite on top
    let mut red_tex = Texture::new(4, 4);
    let mut green_tex = Texture::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            red_tex.set_pixel(x, y, 0xFFFF0000);
            green_tex.set_pixel(x, y, 0xFF00FF00);
        }
    }
    let red_atlas = blitter.upload_atlas(&red_tex);
    let green_atlas = blitter.upload_atlas(&green_tex);
    let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

    // Queue red first, green second (green should be on top at overlapping position)
    blitter.queue(red_atlas, src, 0, 0, BlitMode::Opaque);
    blitter.queue(green_atlas, src, 0, 0, BlitMode::Opaque);
    blitter.flush_to_framebuffer(&mut fb);

    // (0,0) should be green (drawn last)
    let px = fb.get_pixel(0, 0).unwrap();
    let g = (px >> 8) & 0xFF;
    assert!(g > 200, "expected green on top: g={g}");
}

#[test]
fn flush_zero_sprites_is_noop() {
    let gpu = headless_device();
    let mut blitter = GpuBlitter::new(&gpu, 64, 64);
    let mut fb = Framebuffer::new(64, 64).unwrap();
    fb.clear(0xFFAABBCC);

    blitter.flush_to_framebuffer(&mut fb);

    // Framebuffer should be unchanged
    let px = fb.get_pixel(0, 0).unwrap();
    assert_eq!(px, 0xFFAABBCC);
}
```

**Step 2: Run tests**

Run: `cargo test -p abrash-gpu-render --lib blitter`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/abrash-gpu-render/src/blitter.rs
git commit -m "test(gpu-blitter): add integration tests — colorkey, alpha blend, draw order, empty flush"
```

---

### Task 12: Criterion Benchmarks

**Files:**
- Create: `crates/abrash-gpu-render/benches/gpu_blitter.rs`
- Modify: `crates/abrash-gpu-render/Cargo.toml` (add `criterion` dev-dependency and `[[bench]]`)

**Step 1: Add criterion dev-dependency**

In `crates/abrash-gpu-render/Cargo.toml`:

```toml
[dev-dependencies]
criterion = "0.5"
abrash-core = { path = "../abrash-core" }

[[bench]]
name = "gpu_blitter"
harness = false
```

**Step 2: Write benchmarks**

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use abrash_core::blitter::SrcRect;
use abrash_core::texture::Texture;
use abrash_core::framebuffer::Framebuffer;
use abrash_gpu_render::blitter::{GpuBlitter, AtlasHandle, BlitMode};
use abrash_gpu_render::device::{GpuDevice, GpuDeviceConfig};

fn headless_device() -> GpuDevice {
    GpuDevice::new_headless(&GpuDeviceConfig::headless())
        .expect("GPU required for benchmarks")
}

fn bench_gpu_blit_flush_to_screen(c: &mut Criterion) {
    let gpu = headless_device();
    let sprite_counts = [10, 50, 100, 500, 1000];

    let mut group = c.benchmark_group("gpu_blit_flush");

    for &count in &sprite_counts {
        let mut blitter = GpuBlitter::new(&gpu, 1920, 1080);
        let mut tex = Texture::new(64, 64);
        for y in 0..64 { for x in 0..64 { tex.set_pixel(x, y, 0xFFFF0000); } }
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect { x: 0, y: 0, w: 64, h: 64 };

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("opaque", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    for i in 0..count {
                        blitter.queue(atlas, src, (i * 10) as i32, 0, BlitMode::Opaque);
                    }
                    let _ = blitter.flush_and_readback();
                });
            },
        );
    }
    group.finish();
}

fn bench_gpu_blit_vs_cpu(c: &mut Criterion) {
    let gpu = headless_device();

    let mut group = c.benchmark_group("gpu_vs_cpu_500_sprites");

    let mut tex = Texture::new(64, 64);
    for y in 0..64 { for x in 0..64 { tex.set_pixel(x, y, 0x80FF0000); } }
    let src = SrcRect { x: 0, y: 0, w: 64, h: 64 };

    // GPU path
    let mut blitter = GpuBlitter::new(&gpu, 1920, 1080);
    let atlas = blitter.upload_atlas(&tex);
    group.bench_function("gpu_alpha_500", |b| {
        b.iter(|| {
            for i in 0..500 {
                blitter.queue(atlas, src, (i * 3) as i32, (i * 2) as i32, BlitMode::Alpha);
            }
            let _ = blitter.flush_and_readback();
        });
    });

    // CPU path
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    group.bench_function("cpu_alpha_500", |b| {
        b.iter(|| {
            for i in 0..500 {
                abrash_core::blitter::blit_alpha(&mut fb, &tex, src, (i * 3) as i32, (i * 2) as i32);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_gpu_blit_flush_to_screen, bench_gpu_blit_vs_cpu);
criterion_main!(benches);
```

**Step 3: Run benchmarks**

Run: `cargo bench -p abrash-gpu-render --bench gpu_blitter`
Expected: Benchmark results showing GPU crossover point

**Step 4: Commit**

```bash
git add crates/abrash-gpu-render/benches/gpu_blitter.rs crates/abrash-gpu-render/Cargo.toml
git commit -m "bench(gpu-blitter): add criterion benchmarks — batch scaling + GPU vs CPU comparison"
```

---

### Task 13: Final Verification and Cleanup

**Step 1: Run full test suite**

```bash
cargo test -p abrash-gpu-render
cargo test --features gpu-render
cargo clippy -p abrash-gpu-render -- -D warnings
cargo fmt --check -p abrash-gpu-render
```

**Step 2: Verify re-exports compile**

```bash
cargo check --features gpu-render
```

**Step 3: Run benchmarks for final numbers**

```bash
cargo bench -p abrash-gpu-render --bench gpu_blitter
```

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore(gpu-blitter): final cleanup and verification"
```
