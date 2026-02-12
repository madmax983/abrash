//! Color manipulation utilities.
//!
//! This module provides functions for packing, unpacking, and blending colors.
//! Colors are typically represented as `u32` in `0xAARRGGBB` format.

use crate::math::Vec3;

/// Helper for bilinear interpolation blending using SWAR (SIMD Within A Register).
///
/// **SWAR** is a technique to perform parallel operations on data packed into general-purpose registers,
/// avoiding the need for specialized SIMD instructions (like AVX or NEON).
///
/// This function treats a 32-bit integer as two 16-bit slots (or four 8-bit slots with gaps)
/// to blend two color channels simultaneously.
///
/// *   `c0`, `c1`: Packed color channels (e.g., Red/Blue or Green/Alpha).
/// *   `w`, `inv_w`: Weight and inverse weight (sum must be 256 for 8-bit precision).
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

/// Helper to average 4 colors (simple box filter)
#[must_use]
pub const fn average_4_colors(c00: u32, c10: u32, c01: u32, c11: u32) -> u32 {
    let r =
        (((c00 >> 16) & 0xFF) + ((c10 >> 16) & 0xFF) + ((c01 >> 16) & 0xFF) + ((c11 >> 16) & 0xFF))
            / 4;
    let g =
        (((c00 >> 8) & 0xFF) + ((c10 >> 8) & 0xFF) + ((c01 >> 8) & 0xFF) + ((c11 >> 8) & 0xFF)) / 4;
    let b = ((c00 & 0xFF) + (c10 & 0xFF) + (c01 & 0xFF) + (c11 & 0xFF)) / 4;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
#[must_use]
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
#[must_use]
pub const fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
#[must_use]
pub fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
}
