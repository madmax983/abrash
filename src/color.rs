//! Color utilities for packing, unpacking, and blending.

use crate::math::Vec3;

/// Packs RGBA channels into a u32 (0xAARRGGBB).
#[inline(always)]
#[must_use]
pub const fn pack_color(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Unpacks a u32 color into (r, g, b, a).
#[inline(always)]
#[must_use]
pub const fn unpack_color(color: u32) -> (u8, u8, u8, u8) {
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;
    let a = ((color >> 24) & 0xFF) as u8;
    (r, g, b, a)
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB (Alpha is 255).
///
/// Clamps values to [0.0, 1.0].
#[must_use]
pub fn from_vec3(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;
    pack_color(r, g, b, 255)
}

/// Helper for bilinear interpolation blending using SWAR (SIMD Within A Register).
///
/// Blends two colors `c0` and `c1` using weights `w` and `inv_w`.
/// Weights are typically in range [0, 256].
#[inline(always)]
#[must_use]
pub const fn blend_swar(c0: u32, c1: u32, w: u32, inv_w: u32) -> u32 {
    let rb0 = c0 & 0x00FF_00FF;
    let ag0 = (c0 >> 8) & 0x00FF_00FF;
    let rb1 = c1 & 0x00FF_00FF;
    let ag1 = (c1 >> 8) & 0x00FF_00FF;

    let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF_00FF;
    let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF_00FF;

    rb | (ag << 8)
}

/// Helper to average 4 colors (simple box filter).
///
/// Averages all 4 channels (RGBA).
#[inline(always)]
#[must_use]
pub const fn average_4_colors(c00: u32, c10: u32, c01: u32, c11: u32) -> u32 {
    let r = (((c00 >> 16) & 0xFF) + ((c10 >> 16) & 0xFF) + ((c01 >> 16) & 0xFF) + ((c11 >> 16) & 0xFF)) / 4;
    let g = (((c00 >> 8) & 0xFF) + ((c10 >> 8) & 0xFF) + ((c01 >> 8) & 0xFF) + ((c11 >> 8) & 0xFF)) / 4;
    let b = ((c00 & 0xFF) + (c10 & 0xFF) + (c01 & 0xFF) + (c11 & 0xFF)) / 4;
    let a = (((c00 >> 24) & 0xFF) + ((c10 >> 24) & 0xFF) + ((c01 >> 24) & 0xFF) + ((c11 >> 24) & 0xFF)) / 4;

    pack_color(r as u8, g as u8, b as u8, a as u8)
}

/// Calculates luminance using standard weights (Rec. 601).
/// Y = 0.299*R + 0.587*G + 0.114*B
///
/// Approximated as: `Y = (77*R + 150*G + 29*B) >> 8`
#[inline(always)]
#[must_use]
pub const fn luminance(color: u32) -> u8 {
    let r = ((color >> 16) & 0xFF) as u32;
    let g = ((color >> 8) & 0xFF) as u32;
    let b = (color & 0xFF) as u32;

    ((77 * r + 150 * g + 29 * b) >> 8) as u8
}
