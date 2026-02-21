//! # Internal Rasterization Toolkit 🧰
//!
//! This module contains low-level primitives used by the various rasterization backends.
//!
//! **⚠️ Internal Use Only:** These types are not intended for public consumption unless you are
//! extending the engine with a custom rasterizer.
//!
//! ## Core Components
//!
//! *   **`EdgeWalker`**: Interpolates values (X, Z, Color) along the edge of a triangle.
//! *   **`prepare_scanline`**: Validates bounds and returns mutable slices for the framebuffer and Z-buffer.
//! *   **`is_backface`**: Performs back-face culling using the 2D cross product.
//! *   **`sort_by_y`**: A specialized sorting network for 3 vertices.

use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3};
use crate::zbuffer::ZBuffer;

/// Fixed point scale factor (16.16)
///
/// This constant defines the shift amount for 16.16 fixed-point arithmetic.
/// Value is $2^{16} = 65536.0$.
///
/// Used to convert floating-point coordinates to fixed-point integers for sub-pixel precision.
pub const FIXED_SCALE: f32 = 65536.0;

/// Helper to ensure buffer dimensions match
#[inline]
pub(crate) fn assert_same_dimensions(fb: &Framebuffer, zb: &ZBuffer) {
    assert_eq!(
        fb.width(),
        zb.width(),
        "Framebuffer and ZBuffer widths must match"
    );
    assert_eq!(
        fb.height(),
        zb.height(),
        "Framebuffer and ZBuffer heights must match"
    );
}

/// Helper to prepare scanline slices.
/// Returns (`fb_slice`, `zb_slice`, `adjusted_z_start`) or `None` if off-screen.
#[inline(always)]
pub(crate) fn prepare_scanline<'a>(
    fb: &'a mut Framebuffer,
    zb: &'a mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    mut z: f32,
    dz_dx: f32,
) -> Option<(&'a mut [u32], &'a mut [f32], f32)> {
    if y < 0 || y >= fb.height() as i32 {
        return None;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    if xs < 0 {
        let diff = -xs as f32;
        z += diff * dz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return None;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY:
    // 1. xs and xe are clamped to [0, width-1].
    // 2. y is assumed to be within bounds by caller (clamped in fill_triangle).
    // 3. start_idx <= end_idx because xs <= xe.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    Some((fb_slice, zb_slice, z))
}

/// Helper to sort 3 vertices by Y coordinate
///
/// Optimization: Uses a manual sorting network to avoid the heap allocation
/// incurred by `slice::sort_by_key` for small arrays.
pub(crate) fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
where
    F: Fn(&T) -> i32,
{
    // Manual 3-step sort to avoid allocation
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
    if get_y(&verts[1]) > get_y(&verts[2]) {
        verts.swap(1, 2);
    }
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
}

/// Checks if a triangle is backfacing (or degenerate)
///
/// Uses the 2D cross product of the screen-space edges.
/// Returns true if the triangle should be culled (ccw winding for front faces).
#[inline(always)]
pub(crate) fn is_backface(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
    let ux = i64::from(p1.x) - i64::from(p0.x);
    let uy = i64::from(p1.y) - i64::from(p0.y);
    let vx = i64::from(p2.x) - i64::from(p0.x);
    let vy = i64::from(p2.y) - i64::from(p0.y);
    // Use i128 for cross product to prevent overflow with extreme coordinates.
    // i64 is NOT sufficient if coordinates are near i32 limits (e.g. 4e9 * 4e9 = 1.6e19 > i64::MAX).
    let nz = i128::from(ux) * i128::from(vy) - i128::from(uy) * i128::from(vx);
    nz >= 0
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
#[must_use]
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to convert pre-scaled (0.0-255.0) Vec3 color to u32 ARGB
#[must_use]
#[inline(always)]
#[allow(clippy::missing_const_for_fn)]
pub fn color_to_u32_scaled(color: Vec3) -> u32 {
    let r = color.x.clamp(0.0, 255.0) as u32;
    let g = color.y.clamp(0.0, 255.0) as u32;
    let b = color.z.clamp(0.0, 255.0) as u32;
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

/// Helper for fast color packing from fixed point (i32 version).
#[inline(always)]
#[must_use]
pub fn pack_color_fixed_i32(c: (i32, i32, i32)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[inline]
#[must_use]
pub unsafe fn blend_swar_simd(
    c0: std::arch::x86_64::__m256i,
    c1: std::arch::x86_64::__m256i,
    w: std::arch::x86_64::__m256i,
    inv_w: std::arch::x86_64::__m256i,
) -> std::arch::x86_64::__m256i {
    use std::arch::x86_64::{_mm256_set1_epi32, _mm256_or_si256, _mm256_slli_epi32, _mm256_and_si256, _mm256_srli_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16};
    let mask = _mm256_set1_epi32(0x00FF00FF);

    let w_16 = _mm256_or_si256(w, _mm256_slli_epi32(w, 16));
    let inv_w_16 = _mm256_or_si256(inv_w, _mm256_slli_epi32(inv_w, 16));

    let rb0 = _mm256_and_si256(c0, mask);
    let rb1 = _mm256_and_si256(c1, mask);

    let ag0 = _mm256_and_si256(_mm256_srli_epi32(c0, 8), mask);
    let ag1 = _mm256_and_si256(_mm256_srli_epi32(c1, 8), mask);

    let rb_sum = _mm256_add_epi16(
        _mm256_mullo_epi16(rb0, inv_w_16),
        _mm256_mullo_epi16(rb1, w_16),
    );

    let ag_sum = _mm256_add_epi16(
        _mm256_mullo_epi16(ag0, inv_w_16),
        _mm256_mullo_epi16(ag1, w_16),
    );

    let rb = _mm256_and_si256(_mm256_srli_epi16(rb_sum, 8), mask);
    let ag = _mm256_and_si256(_mm256_srli_epi16(ag_sum, 8), mask);

    _mm256_or_si256(rb, _mm256_slli_epi32(ag, 8))
}

/// Helper to iterate along the edge of a triangle in screen space.
///
/// This struct manages the state for walking down a triangle edge, interpolating
/// X and Z coordinates. It uses **16.16 fixed-point arithmetic** for the X coordinate
/// to ensure pixel-perfect rasterization consistency and avoid "cracks" between adjacent triangles
/// that can occur with floating-point errors.
///
/// *   **16.16 Format**: The upper 16 bits represent the integer pixel coordinate, and the lower
///     16 bits represent the fractional sub-pixel position.
pub(crate) struct EdgeWalker {
    /// Current X coordinate in 16.16 fixed-point format.
    ///
    /// The upper 16 bits represent the integer pixel coordinate.
    /// The lower 16 bits represent sub-pixel precision.
    pub(crate) x: i64,
    /// Change in X per scanline (dx/dy) in 16.16 fixed-point format.
    dx_dy: i64,
    /// Current Z depth.
    pub(crate) z: f32,
    /// Change in Z per scanline (dz/dy).
    dz_dy: f32,
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy) = if height == 0.0 {
            (0, 0.0)
        } else {
            let inv_h = 1.0 / height;

            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
            )
        };

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * (n as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::ScreenPoint;

    #[test]
    fn edge_walker_z_interpolation() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        assert!((walker.z - 1.0).abs() < 0.0001);
        let expected_dz_dy = (2.0_f32 - 1.0_f32) / 100.0_f32;
        assert!((walker.dz_dy - expected_dz_dy).abs() < 0.0001);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let mut walker = EdgeWalker::new(p0, p1);
        let initial_z = walker.z;
        let dz = walker.dz_dy;

        walker.step_n(50);
        let expected_z = initial_z + dz * 50.0;
        assert!((walker.z - expected_z).abs() < 0.0001);
        assert!(
            walker.z > 1.0 && walker.z < 2.0,
            "z should be interpolated between 1.0 and 2.0, got {}",
            walker.z,
        );
    }

    #[test]
    fn edge_walker_zero_height_no_panic() {
        let p0 = ScreenPoint {
            x: 0,
            y: 100,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);
        assert_eq!(walker.dx_dy, 0);
        assert!((walker.dz_dy - 0.0).abs() < f32::EPSILON);
        assert!((walker.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_is_backface_overflow_safe() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 2_000_000_000,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p2 = ScreenPoint {
            x: 0,
            y: 2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };

        let result = is_backface(p0, p1, p2);
        assert!(result);
    }

    #[test]
    fn test_is_backface_extreme_coordinates_overflow() {
        // Test case that overflows i64
        let p0 = ScreenPoint {
            x: -2_000_000_000,
            y: -2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 2_000_000_000,
            y: -2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };
        let p2 = ScreenPoint {
            x: -2_000_000_000,
            y: 2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };

        // ux = 4e9, uy = 0
        // vx = 0, vy = 4e9
        // nz = 16e18. i64 max is 9e18.
        // With i64, this overflows and wraps to negative (kept).
        // With i128, this stays positive (culled).

        let result = is_backface(p0, p1, p2);
        assert!(result, "Huge CW triangle should be culled (nz > 0)");

        // Reverse winding (CCW)
        let result_ccw = is_backface(p0, p2, p1);
        assert!(!result_ccw, "Huge CCW triangle should be kept (nz < 0)");
    }
}
