//! GPU-accelerated 2D sprite blitting.
//!
//! Types for batched sprite rendering on the GPU, matching the CPU blitter's
//! three-tier model (opaque, color-key, alpha) but expressed as instance data
//! suitable for a single instanced draw call.

#[allow(unused_imports)]
use abrash_core::blitter::SrcRect;
use bytemuck::{Pod, Zeroable};

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
