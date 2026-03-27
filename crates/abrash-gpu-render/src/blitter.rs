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
}
