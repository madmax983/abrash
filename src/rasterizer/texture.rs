//! Perspective-correct texture mapping rasterizer.
//!
//! This module implements scanline rasterization for textured triangles with perspective correction.
//!
//! # The Problem with Linear Interpolation
//!
//! In 3D graphics, simply interpolating texture coordinates $(u, v)$ linearly across the screen
//! results in "affine texture mapping," which looks distorted because it doesn't account for depth.
//! As a polygon recedes into the distance, the texture should appear compressed.
//!
//! # The Solution: Perspective Correction
//!
//! To achieve correct perspective, we must interpolate attributes in a way that respects the
//! projective divide. The standard technique is to interpolate:
//!
//! *   $1/w$: The reciprocal of the homogeneous W coordinate.
//! *   $u/w$: The texture U coordinate divided by W.
//! *   $v/w$: The texture V coordinate divided by W.
//!
//! For each pixel, we recover the true texture coordinates by dividing by the interpolated $1/w$:
//!
//! $$ u_{pixel} = \frac{(u/w)_{interpolated}}{(1/w)_{interpolated}} $$
//! $$ v_{pixel} = \frac{(v/w)_{interpolated}}{(1/w)_{interpolated}} $$
//!
//! # Features
//!
//! *   **Perspective Correction**: Accurate texture mapping at any angle.
//! *   **Sub-pixel Precision**: Uses 16.16 fixed-point arithmetic for edge walking.
//! *   **Multiple Filtering Modes**: Nearest Neighbor, Bilinear, and Trilinear (Mipmapping).
//! *   **Simd Optimization**: AVX2 accelerated rasterization for high performance.

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{fast_inv_sqrt, project_triangle_to_screen, ScreenPoint, Vec2, Vec3, Vec4};
use crate::texture::{blend_four_way, blend_swar, FilterMode, Texture};
use crate::zbuffer::ZBuffer;

use super::core::{
    assert_same_dimensions, color_to_u32, is_backface, sort_by_y, FIXED_SCALE,
};

/// Gradients for perspective-correct texture mapping.
///
/// This struct holds the per-pixel (dX) and per-scanline (dY) changes for:
/// *   `z`: Depth (linear in screen space).
/// *   `q`: Inverse W ($1/w$).
/// *   `u`: Texture U over W ($u/w$).
/// *   `v`: Texture V over W ($v/w$).
///
/// These gradients are calculated once per triangle and used to step the
/// edge walkers and scanline interpolators.
#[derive(Clone, Copy)]
pub struct PerspectiveTextureGradients {
    /// Change in depth (Z) per X pixel.
    pub dz_dx: f32,
    /// Change in $1/w$ per X pixel.
    pub dq_dx: f32,
    /// Change in $u/w$ per X pixel.
    pub du_dx: f32,
    /// Change in $v/w$ per X pixel.
    pub dv_dx: f32,
    /// Change in $1/w$ per Y scanline.
    pub dq_dy: f32,
    /// Change in $u/w$ per Y scanline.
    pub du_dy: f32,
    /// Change in $v/w$ per Y scanline.
    pub dv_dy: f32,
}

impl PerspectiveTextureGradients {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
    ) -> Self {
        Self::new_with_winding(p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2).0
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new_with_winding(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        // Calculate Y gradients
        let ny_q = uq * vx - ux * vq;
        let dq_dy = ny_q * inv_nz;

        let ny_u = uu * vx - ux * vu;
        let du_dy = ny_u * inv_nz;

        let ny_v = uv * vx - ux * vv;
        let dv_dy = ny_v * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                du_dx,
                dv_dx,
                dq_dy,
                du_dy,
                dv_dy,
            },
            nz > 0.0,
        )
    }
}

pub(crate) struct PerspectiveTextureEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
}

impl PerspectiveTextureEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub struct PerspectiveSpanStart {
    pub z: f32,
    pub q: f32,
    pub u: f32,
    pub v: f32,
}

pub(crate) const RECIPROCAL_TABLE: [f32; 17] = [
    0.0,
    1.0,
    0.5,
    0.333_333_34,
    0.25,
    0.2,
    0.166_666_67,
    0.142_857_15,
    0.125,
    0.111_111_11,
    0.1,
    0.090_909_09,
    0.083_333_336,
    0.076_923_08,
    0.071_428_575,
    0.066_666_67,
    0.062_5,
];

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_nearest(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let tex_w_usize = tex_w as usize;

    assert!(tex_pixels.len() >= (tex_w as usize) * (tex_h as usize));

    let shift = texture.width_shift;
    if shift < 32 {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    tex_pixels[((v as usize) << shift) + (u as usize)]
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z;
                    *pixel = color;
                } else if alpha > 0 {
                    let dest = *pixel;
                    *pixel = blend_swar(color, dest, 255 - alpha, alpha);
                }
            }
            z += dz_dx;
            u_fix = u_fix.wrapping_add(du_fix);
            v_fix = v_fix.wrapping_add(dv_fix);
        }
    } else {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    tex_pixels[(v as usize) * tex_w_usize + (u as usize)]
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z;
                    *pixel = color;
                } else if alpha > 0 {
                    let dest = *pixel;
                    *pixel = blend_swar(color, dest, alpha, 255 - alpha);
                }
            }
            z += dz_dx;
            u_fix = u_fix.wrapping_add(du_fix);
            v_fix = v_fix.wrapping_add(dv_fix);
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_bilinear(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let shift = texture.width_shift;

    let w_i32 = (tex_w as i32).wrapping_sub(1);
    let h_i32 = (tex_h as i32).wrapping_sub(1);
    let tex_w_usize = tex_w as usize;

    macro_rules! process_span_bilinear {
        ($op:tt, $val:expr) => {
            let mut cached_x0 = i32::MIN;
            let mut cached_y0 = i32::MIN;
            let mut c00 = 0;
            let mut c10 = 0;
            let mut c01 = 0;
            let mut c11 = 0;

            for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                if z < *depth_val {
                    let u_img_fixed = u_fix >> 8;
                    let v_img_fixed = v_fix >> 8;

                    let x0_raw = u_img_fixed >> 8;
                    let y0_raw = v_img_fixed >> 8;

                    if x0_raw != cached_x0 || y0_raw != cached_y0 {
                        cached_x0 = x0_raw;
                        cached_y0 = y0_raw;

                        let (t00, t10, t01, t11) =
                            if (x0_raw as u32) < (w_i32 as u32) && (y0_raw as u32) < (h_i32 as u32) {
                                let x0 = x0_raw as usize;
                                let y0 = y0_raw as usize;

                                let row0 = y0 $op $val;
                                let row1 = row0 + tex_w_usize;

                                unsafe {
                                    #[cfg(target_endian = "little")]
                                    {
                                        let ptr = tex_pixels.as_ptr();
                                        let row0_pair = ptr.add(row0 + x0).cast::<u64>().read_unaligned();
                                        let row1_pair = ptr.add(row1 + x0).cast::<u64>().read_unaligned();

                                        (
                                            row0_pair as u32,
                                            (row0_pair >> 32) as u32,
                                            row1_pair as u32,
                                            (row1_pair >> 32) as u32,
                                        )
                                    }
                                    #[cfg(not(target_endian = "little"))]
                                    {
                                        (
                                            *tex_pixels.get_unchecked(row0 + x0),
                                            *tex_pixels.get_unchecked(row0 + x0 + 1),
                                            *tex_pixels.get_unchecked(row1 + x0),
                                            *tex_pixels.get_unchecked(row1 + x0 + 1),
                                        )
                                    }
                                }
                            } else {
                                let x0 = x0_raw.clamp(0, w_i32) as usize;
                                let y0 = y0_raw.clamp(0, h_i32) as usize;
                                let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
                                let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

                                let row0 = y0 $op $val;
                                let row1 = y1 $op $val;

                                unsafe {
                                    (
                                        *tex_pixels.get_unchecked(row0 + x0),
                                        *tex_pixels.get_unchecked(row0 + x1),
                                        *tex_pixels.get_unchecked(row1 + x0),
                                        *tex_pixels.get_unchecked(row1 + x1),
                                    )
                                }
                            };
                        c00 = t00;
                        c10 = t10;
                        c01 = t01;
                        c11 = t11;
                    }

                    let wx = (u_img_fixed & 0xFF) as u32;
                    let wy = (v_img_fixed & 0xFF) as u32;

                    let final_color = blend_four_way(c00, c10, c01, c11, wx, wy);

                    let alpha = (final_color >> 24) & 0xFF;
                    if alpha == 255 {
                        *depth_val = z;
                        *pixel = final_color;
                    } else if alpha > 0 {
                        let dest = *pixel;
                        *pixel = blend_swar(final_color, dest, 255 - alpha, alpha);
                    }
                }
                z += dz_dx;
                u_fix = u_fix.wrapping_add(du_fix);
                v_fix = v_fix.wrapping_add(dv_fix);
            }
        };
    }

    if shift < 32 {
        process_span_bilinear!(<<, shift);
    } else {
        process_span_bilinear!(*, tex_w_usize);
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
unsafe fn draw_span_bilinear_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    z_start: f32,
    dz_dx: f32,
    u_fix_start: i32,
    v_fix_start: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    use std::arch::x86_64::*;

    let len = fb_slice.len();
    let mut i = 0;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec =
            _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_fix_start), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_fix_start), dv_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let one_i = _mm256_set1_epi32(1);

        let mask_ff = _mm256_set1_epi32(0xFF);

        while i + 8 <= len {
            // Z-Test
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask_z) != 0 {
                // Bilinear Logic
                // u_fix is 16.16 offset by -0.5 (32768). Shift right 8 to get 24.8
                let u_img = _mm256_srai_epi32(u_fix_vec, 8);
                let v_img = _mm256_srai_epi32(v_fix_vec, 8);

                // Weights (fractional part 0..255)
                let wx = _mm256_and_si256(u_img, mask_ff);
                let wy = _mm256_and_si256(v_img, mask_ff);

                let const_256 = _mm256_set1_epi32(256);
                let inv_wx = _mm256_sub_epi32(const_256, wx);
                let inv_wy = _mm256_sub_epi32(const_256, wy);

                // Integer coords (floor)
                let x0_raw = _mm256_srai_epi32(u_img, 8);
                let y0_raw = _mm256_srai_epi32(v_img, 8);

                // Clamp
                let x0 = _mm256_min_epi32(_mm256_max_epi32(x0_raw, zero_i), max_x);
                let y0 = _mm256_min_epi32(_mm256_max_epi32(y0_raw, zero_i), max_y);

                let x1 = _mm256_min_epi32(
                    _mm256_max_epi32(_mm256_add_epi32(x0_raw, one_i), zero_i),
                    max_x,
                );
                let y1 = _mm256_min_epi32(
                    _mm256_max_epi32(_mm256_add_epi32(y0_raw, one_i), zero_i),
                    max_y,
                );

                // Indices: idx = y * w + x
                let y0_w = _mm256_mullo_epi32(y0, w_vec);
                let y1_w = _mm256_mullo_epi32(y1, w_vec);

                let idx00 = _mm256_add_epi32(y0_w, x0);
                let idx10 = _mm256_add_epi32(y0_w, x1);
                let idx01 = _mm256_add_epi32(y1_w, x0);
                let idx11 = _mm256_add_epi32(y1_w, x1);

                // Gather
                let pixels_ptr = texture.pixels.as_ptr() as *const i32;
                let c00 = _mm256_i32gather_epi32(pixels_ptr, idx00, 4);
                let c10 = _mm256_i32gather_epi32(pixels_ptr, idx10, 4);
                let c01 = _mm256_i32gather_epi32(pixels_ptr, idx01, 4);
                let c11 = _mm256_i32gather_epi32(pixels_ptr, idx11, 4);

                let blend_swar_avx2 = |c0: __m256i, c1: __m256i, w: __m256i, inv_w: __m256i| -> __m256i {
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
                };

                let top = blend_swar_avx2(c00, c10, wx, inv_wx);
                let bot = blend_swar_avx2(c01, c11, wx, inv_wx);
                let final_color = blend_swar_avx2(top, bot, wy, inv_wy);

                // Write Opaque: (mask_z & opaque)
                // Need to extract alpha from final_color
                let a = _mm256_and_si256(_mm256_srli_epi32(final_color, 24), mask_ff);
                let opaque_mask = _mm256_cmpeq_epi32(a, mask_ff); // Alpha == 255
                let mask_z_int = _mm256_castps_si256(mask_z);
                let write_opaque_mask = _mm256_and_si256(mask_z_int, opaque_mask);
                let write_opaque_mask_ps = _mm256_castsi256_ps(write_opaque_mask);

                if _mm256_movemask_ps(write_opaque_mask_ps) != 0 {
                    let old_z = _mm256_loadu_ps(depth_ptr);
                    let new_z = _mm256_blendv_ps(old_z, z_vec, write_opaque_mask_ps);
                    _mm256_storeu_ps(depth_ptr, new_z);

                    let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                    let old_color = _mm256_loadu_si256(fb_ptr);
                    let new_color = _mm256_blendv_epi8(old_color, final_color, write_opaque_mask);
                    _mm256_storeu_si256(fb_ptr, new_color);
                }

                // Translucent
                let zero_mask = _mm256_cmpeq_epi32(a, zero_i);
                let trans_mask =
                    _mm256_andnot_si256(opaque_mask, _mm256_andnot_si256(zero_mask, mask_z_int));
                let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(trans_mask));

                if trans_bits != 0 {
                    let mut temp_pixels = [0u32; 8];
                    _mm256_storeu_si256(temp_pixels.as_mut_ptr() as *mut __m256i, final_color);

                    let mut bit = 1;
                    for k in 0..8 {
                        if (trans_bits & bit) != 0 {
                            let idx_scalar = i + k;
                            let color = temp_pixels[k];
                            let alpha = (color >> 24) & 0xFF;
                            let dest = *fb_slice.get_unchecked(idx_scalar);
                            *fb_slice.get_unchecked_mut(idx_scalar) =
                                blend_swar(color, dest, 255 - alpha, alpha);
                        }
                        bit <<= 1;
                    }
                }
            }

            z_vec = _mm256_add_ps(z_vec, dz_step);
            u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
            v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
            i += 8;
        }
    }

    // Scalar Tail
    if i < len {
        let z_curr = z_start + (i as f32) * dz_dx;
        let u_curr = u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32));
        let v_curr = v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32));

        draw_span_bilinear(
            &mut fb_slice[i..],
            &mut zb_slice[i..],
            texture,
            z_curr,
            dz_dx,
            u_curr,
            v_curr,
            du_fix,
            dv_fix,
        );
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_trilinear(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
    lod: f32,
) {
    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            let color = texture.get_pixel_trilinear_fixed(u_fix, v_fix, lod);
            let alpha = (color >> 24) & 0xFF;

            if alpha == 255 {
                *depth_val = z;
                *pixel = color;
            } else if alpha > 0 {
                let dest = *pixel;
                *pixel = blend_swar(color, dest, 255 - alpha, alpha);
            }
        }
        z += dz_dx;
        u_fix = u_fix.wrapping_add(du_fix);
        v_fix = v_fix.wrapping_add(dv_fix);
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
unsafe fn draw_span_nearest_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    z_start: f32,
    dz_dx: f32,
    u_fix_start: i32,
    v_fix_start: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    use std::arch::x86_64::*;

    let len = fb_slice.len();
    let mut i = 0;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec =
            _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_fix_start), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_fix_start), dv_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let ff_mask_shifted = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let shift_vec = _mm256_set1_epi32(texture.width_shift as i32);

        let is_pot = texture.width_shift < 32;

        while i + 8 <= len {
            // Z-Test
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_z_int = _mm256_castps_si256(mask_z);

            if _mm256_movemask_ps(mask_z) != 0 {
                // Calculate Indices
                let u_i = _mm256_srai_epi32(u_fix_vec, 16);
                let v_i = _mm256_srai_epi32(v_fix_vec, 16);

                let idx = if is_pot {
                // Note: We clamp to match the scalar implementation (draw_span_nearest / get_pixel_texel).
                // Although wrapping is faster and standard for PoT, we must preserve rendering parity.
                // The existing `draw_scanline_normal_mapped_simd` uses wrapping, but that creates
                // an inconsistency with its own scalar fallback. We choose to be consistent with scalar here.
                    let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                } else {
                    let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                };

                // Gather
                let pixel_vals =
                    _mm256_i32gather_epi32(texture.pixels.as_ptr() as *const i32, idx, 4);

                // Check Alpha
                let alphas_shifted = _mm256_and_si256(pixel_vals, ff_mask_shifted);
                let opaque_mask = _mm256_cmpeq_epi32(alphas_shifted, ff_mask_shifted);
                let zero_mask = _mm256_cmpeq_epi32(alphas_shifted, zero_i);

                // Write Opaque: (mask_z & opaque)
                let write_opaque_mask = _mm256_and_si256(mask_z_int, opaque_mask);
                let write_opaque_mask_ps = _mm256_castsi256_ps(write_opaque_mask);

                if _mm256_movemask_ps(write_opaque_mask_ps) != 0 {
                    let old_z = _mm256_loadu_ps(depth_ptr);
                    let new_z = _mm256_blendv_ps(old_z, z_vec, write_opaque_mask_ps);
                    _mm256_storeu_ps(depth_ptr, new_z);

                    let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                    let old_color = _mm256_loadu_si256(fb_ptr);
                    let new_color = _mm256_blendv_epi8(old_color, pixel_vals, write_opaque_mask);
                    _mm256_storeu_si256(fb_ptr, new_color);
                }

                // Translucent: (mask_z & !opaque & !zero)
                let trans_mask =
                    _mm256_andnot_si256(opaque_mask, _mm256_andnot_si256(zero_mask, mask_z_int));
                let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(trans_mask));

                if trans_bits != 0 {
                    let mut temp_pixels = [0u32; 8];
                    _mm256_storeu_si256(temp_pixels.as_mut_ptr() as *mut __m256i, pixel_vals);

                    let mut bit = 1;
                    for k in 0..8 {
                        if (trans_bits & bit) != 0 {
                            let idx_scalar = i + k;
                            let color = temp_pixels[k];
                            let alpha = (color >> 24) & 0xFF;
                            let dest = *fb_slice.get_unchecked(idx_scalar);
                            *fb_slice.get_unchecked_mut(idx_scalar) =
                                blend_swar(color, dest, 255 - alpha, alpha);
                        }
                        bit <<= 1;
                    }
                }
            }

            z_vec = _mm256_add_ps(z_vec, dz_step);
            u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
            v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
            i += 8;
        }
    }

    // Scalar Tail
    let mut z_curr = z_start + (i as f32) * dz_dx;
    let mut u_curr = u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32));
    let mut v_curr = v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32));

    while i < len {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(i);
            if z_curr < *depth_val {
                let u = u_curr >> 16;
                let v = v_curr >> 16;

                let tex_w = texture.width;
                let tex_h = texture.height;

                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    let shift = texture.width_shift;
                    let idx = if shift < 32 {
                        ((v as usize) << shift) + (u as usize)
                    } else {
                        (v as usize) * (tex_w as usize) + (u as usize)
                    };
                    *texture.pixels.get_unchecked(idx)
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z_curr;
                    *fb_slice.get_unchecked_mut(i) = color;
                } else if alpha > 0 {
                    let dest = *fb_slice.get_unchecked(i);
                    *fb_slice.get_unchecked_mut(i) = blend_swar(color, dest, 255 - alpha, alpha);
                }
            }
        }
        z_curr += dz_dx;
        u_curr = u_curr.wrapping_add(du_fix);
        v_curr = v_curr.wrapping_add(dv_fix);
        i += 1;
    }
}

/// Draw a single scanline with perspective-correct texture mapping
/// Optimized using span-based interpolation (every 16 pixels)
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_textured_perspective(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    texture: &Texture,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PerspectiveSpanStart,
    gradients: &PerspectiveTextureGradients,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let span_size = 16;
    let mut x = xs;

    // Calculate initial start values
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
    let mut u_tex_start = u * w_start;
    let mut v_tex_start = v * w_start;

    while x <= xe {
        let remaining = xe - x + 1;
        let count = remaining.min(span_size);

        // End values at 'x + count'
        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        // Perform perspective divide at span endpoints
        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        // Interpolate texel coordinates linearly over the span
        let inv_count = RECIPROCAL_TABLE[count as usize];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (x as usize);
        let end_idx = y_offset + ((x + count - 1) as usize);

        // SAFETY: Bounds checked by xs, xe clamping and loop logic
        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        match texture.filter_mode {
            FilterMode::Nearest => {
                // Fixed point optimization for Nearest Neighbor
                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_nearest_simd(
                            fb_slice,
                            zb_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                        );
                    }
                } else {
                    draw_span_nearest(
                        fb_slice,
                        zb_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                    );
                }

                #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
                draw_span_nearest(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Bilinear => {
                let u_fix = ((u_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let v_fix = ((v_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_bilinear_simd(
                            fb_slice,
                            zb_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                        );
                    }
                } else {
                    draw_span_bilinear(
                        fb_slice,
                        zb_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                    );
                }

                #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
                draw_span_bilinear(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Trilinear => {
                // For Trilinear, we need LOD.
                let w = w_start; // 1/q
                let w_sq = w * w;

                let du_tex_dx = (gradients.du_dx * q - u * gradients.dq_dx) * w_sq;
                let dv_tex_dx = (gradients.dv_dx * q - v * gradients.dq_dx) * w_sq;
                let du_tex_dy = (gradients.du_dy * q - u * gradients.dq_dy) * w_sq;
                let dv_tex_dy = (gradients.dv_dy * q - v * gradients.dq_dy) * w_sq;

                let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
                    .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);
                let lod = 0.5 * max_rho_sq.log2();

                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                draw_span_trilinear(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                    lod,
                );
            }
        }

        // Advance state
        z += gradients.dz_dx * count as f32;
        q = q_end;
        u = u_end;
        v = v_end;

        // Reuse end values for next start
        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;

        x += count;
    }
}

/// Fill a textured 3D triangle with perspective correction.
///
/// This function renders a triangle using the specified texture and perspective-correct interpolation.
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), UV)`.
///     *   `Position`: Clip Space position (before perspective divide).
///     *   `W`: Homogeneous W coordinate (usually from projection matrix).
///     *   `UV`: Texture coordinates (0.0 - 1.0).
/// *   `texture` - Source texture to sample from.
///
/// # Implementation Details
///
/// 1.  **Clipping**: The triangle is clipped against the view frustum.
/// 2.  **Projection**: Vertices are projected to screen space.
/// 3.  **Backface Culling**: Back-facing triangles are discarded.
/// 4.  **Gradient Calculation**: Screen-space derivatives ($1/w$, $u/w$, $v/w$) are computed.
/// 5.  **Rasterization**: The triangle is filled scanline by scanline, interpolating attributes.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_textured;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::{Vec2, Vec3};
/// use abrash::texture::Texture;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
/// let texture = Texture::new(32, 32); // Assume empty texture
///
/// // Vertices: ((Pos, W), UV)
/// let v0 = ((Vec3::new(0.0, 5.0, 5.0), 5.0), Vec2::new(0.5, 0.0));
/// let v1 = ((Vec3::new(-5.0, -5.0, 5.0), 5.0), Vec2::new(0.0, 1.0));
/// let v2 = ((Vec3::new(5.0, -5.0, 5.0), 5.0), Vec2::new(1.0, 1.0));
///
/// fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
/// ```
pub fn fill_triangle_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2),
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            half_width,
            half_height,
        );

        // Backface Culling
        let ux_orig = (i64::from(p1_orig.x) - i64::from(p0_orig.x)) as f32;
        let uy_orig = (i64::from(p1_orig.y) - i64::from(p0_orig.y)) as f32;
        let vx_orig = (i64::from(p2_orig.x) - i64::from(p0_orig.x)) as f32;
        let vy_orig = (i64::from(p2_orig.y) - i64::from(p0_orig.y)) as f32;
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0.0 {
            continue;
        }

        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let u0 = v0.1.x * texture.width as f32 * inv_w0;
        let v0_val = v0.1.y * texture.height as f32 * inv_w0;

        let u1 = v1.1.x * texture.width as f32 * inv_w1;
        let v1_val = v1.1.y * texture.height as f32 * inv_w1;

        let u2 = v2.1.x * texture.width as f32 * inv_w2;
        let v2_val = v2.1.y * texture.height as f32 * inv_w2;

        let mut verts = [
            (p0_orig, u0, v0_val),
            (p1_orig, u1, v1_val),
            (p2_orig, u2, v2_val),
        ];
        sort_by_y(&mut verts, |(p, _, _)| p.y);
        let [(p0, u0, v0), (p1, u1, v1), (p2, u2, v2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        if total_height == 0.0 {
            continue;
        }

        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
        }

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = PerspectiveTextureGradients::new_with_winding(
            p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2,
        );

        let mut edge_a = PerspectiveTextureEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0, v2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PerspectiveTextureEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0, v1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            }

            let (x_start, x_end, z_left, q_left, u_left, v_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.q,
                    edge_a.u,
                    edge_a.v,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.q,
                    edge_b.u,
                    edge_b.v,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 && q_left.abs() > 0.000_001 {
                    // SAFETY: Safe due to clamps on x_start and y
                    unsafe {
                        let z_current = zb.get_depth_unchecked(x_start as usize, y as usize);
                        if z_left < z_current {
                            let w = 1.0 / q_left;
                            let u_tex = u_left * w;
                            let v_tex = v_left * w;
                            let color = match texture.filter_mode {
                                FilterMode::Nearest => {
                                    texture.get_pixel_texel(u_tex as i32, v_tex as i32)
                                }
                                FilterMode::Bilinear => {
                                    texture.get_pixel_bilinear_texel(u_tex, v_tex)
                                }
                                FilterMode::Trilinear => {
                                    let w = 1.0 / q_left;
                                    let w_sq = w * w;

                                    let du_tex_dx = (gradients.du_dx * q_left
                                        - u_left * gradients.dq_dx)
                                        * w_sq;
                                    let dv_tex_dx = (gradients.dv_dx * q_left
                                        - v_left * gradients.dq_dx)
                                        * w_sq;
                                    let du_tex_dy = (gradients.du_dy * q_left
                                        - u_left * gradients.dq_dy)
                                        * w_sq;
                                    let dv_tex_dy = (gradients.dv_dy * q_left
                                        - v_left * gradients.dq_dy)
                                        * w_sq;

                                    let max_rho_sq = (du_tex_dx * du_tex_dx
                                        + dv_tex_dx * dv_tex_dx)
                                        .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                                    let lod = 0.5 * max_rho_sq.log2();
                                    texture.get_pixel_trilinear(u_tex, v_tex, lod)
                                }
                            };

                            let alpha = (color >> 24) & 0xFF;
                            if alpha == 255 {
                                let width_usize = fb.width() as usize;
                                let idx = (y as usize) * width_usize + (x_start as usize);
                                *zb.as_mut_slice().get_unchecked_mut(idx) = z_left;
                                fb.set_pixel_unchecked(x_start as usize, y as usize, color);
                            } else if alpha > 0 {
                                let dest = fb.get_pixel_unchecked(x_start as usize, y as usize);
                                let blended = blend_swar(color, dest, 255 - alpha, alpha);
                                fb.set_pixel_unchecked(x_start as usize, y as usize, blended);
                            }
                        }
                    }
                }
            } else {
                draw_scanline_textured_perspective(
                    fb,
                    zb,
                    texture,
                    y,
                    x_start,
                    x_end,
                    PerspectiveSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                    },
                    &gradients,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[derive(Clone, Copy)]
struct NormalMapGradients {
    dz_dx: f32,
    dq_dx: f32,  // 1/w
    du_dx: f32,  // u/w
    dv_dx: f32,  // v/w
    dlx_dx: f32, // lx/w (Tangent Space Light X)
    dly_dx: f32,
    dlz_dx: f32,
}

impl NormalMapGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
        l0: Vec3, // Tangent Space Light Vectors (pre-scaled by q)
        l1: Vec3,
        l2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let ulx = l1.x - l0.x;
        let uly = l1.y - l0.y;
        let ulz = l1.z - l0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vlx = l2.x - l0.x;
        let vly = l2.y - l0.y;
        let vlz = l2.z - l0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        let nx_lx = uy * vlx - ulx * vy;
        let dlx_dx = nx_lx * inv_nz;

        let nx_ly = uy * vly - uly * vy;
        let dly_dx = nx_ly * inv_nz;

        let nx_lz = uy * vlz - ulz * vy;
        let dlz_dx = nx_lz * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                du_dx,
                dv_dx,
                dlx_dx,
                dly_dx,
                dlz_dx,
            },
            nz > 0.0,
        )
    }
}

struct NormalMapEdgeWalker {
    x: i64,
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    lx: f32,
    ly: f32,
    lz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dlx_dy: f32,
    dly_dy: f32,
    dlz_dy: f32,
}

impl NormalMapEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
        l_start: Vec3,
        l_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dlx_dy = (l_end.x - l_start.x) * inv_h;
        let dly_dy = (l_end.y - l_start.y) * inv_h;
        let dlz_dy = (l_end.z - l_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            lx: l_start.x,
            ly: l_start.y,
            lz: l_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dlx_dy,
            dly_dy,
            dlz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.lx += self.dlx_dy;
        self.ly += self.dly_dy;
        self.lz += self.dlz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.lx += self.dlx_dy * n_f;
        self.ly += self.dly_dy * n_f;
        self.lz += self.dlz_dy * n_f;
    }
}

#[derive(Clone, Copy)]
struct NormalMapSpanStart {
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    lx: f32,
    ly: f32,
    lz: f32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
unsafe fn draw_scanline_normal_mapped_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    lx: f32,
    ly: f32,
    lz: f32,
    gradients: &NormalMapGradients,
    texture: &Texture,
    normal_map: &Texture,
    pre_diffuse_color: Vec3,
    ambient: Vec3,
) {
    unsafe {
        use std::arch::x86_64::*;

        let len = fb_slice.len();
        let mut i = 0;

        // Load gradients
        let dz_dx = _mm256_set1_ps(gradients.dz_dx);
        let dq_dx = _mm256_set1_ps(gradients.dq_dx);
        let du_dx = _mm256_set1_ps(gradients.du_dx);
        let dv_dx = _mm256_set1_ps(gradients.dv_dx);
        let dlx_dx = _mm256_set1_ps(gradients.dlx_dx);
        let dly_dx = _mm256_set1_ps(gradients.dly_dx);
        let dlz_dx = _mm256_set1_ps(gradients.dlz_dx);

        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx, offsets));
        let mut q_vec = _mm256_add_ps(_mm256_set1_ps(q), _mm256_mul_ps(dq_dx, offsets));
        let mut u_vec = _mm256_add_ps(_mm256_set1_ps(u), _mm256_mul_ps(du_dx, offsets));
        let mut v_vec = _mm256_add_ps(_mm256_set1_ps(v), _mm256_mul_ps(dv_dx, offsets));
        let mut lx_vec = _mm256_add_ps(_mm256_set1_ps(lx), _mm256_mul_ps(dlx_dx, offsets));
        let mut ly_vec = _mm256_add_ps(_mm256_set1_ps(ly), _mm256_mul_ps(dly_dx, offsets));
        let mut lz_vec = _mm256_add_ps(_mm256_set1_ps(lz), _mm256_mul_ps(dlz_dx, offsets));

        let step_8 = _mm256_set1_ps(8.0);
        let dz_step = _mm256_mul_ps(dz_dx, step_8);
        let dq_step = _mm256_mul_ps(dq_dx, step_8);
        let du_step = _mm256_mul_ps(du_dx, step_8);
        let dv_step = _mm256_mul_ps(dv_dx, step_8);
        let dlx_step = _mm256_mul_ps(dlx_dx, step_8);
        let dly_step = _mm256_mul_ps(dly_dx, step_8);
        let dlz_step = _mm256_mul_ps(dlz_dx, step_8);

        let one = _mm256_set1_ps(1.0);
        let epsilon = _mm256_set1_ps(0.0001);
        let scale_255 = _mm256_set1_ps(255.0);
        let inv_255 = _mm256_set1_ps(1.0 / 255.0);
        let two = _mm256_set1_ps(2.0);
        let zero = _mm256_setzero_ps();
        let one_point_five = _mm256_set1_ps(1.5);
        let zero_point_five = _mm256_set1_ps(0.5);
        let _neg_one = _mm256_set1_ps(-1.0);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        let diff_r_const = _mm256_set1_ps(pre_diffuse_color.x);
        let diff_g_const = _mm256_set1_ps(pre_diffuse_color.y);
        let diff_b_const = _mm256_set1_ps(pre_diffuse_color.z);

        let amb_r = _mm256_set1_ps(ambient.x);
        let amb_g = _mm256_set1_ps(ambient.y);
        let amb_b = _mm256_set1_ps(ambient.z);

        let scale_nm = _mm256_mul_ps(two, inv_255);

        while i + 8 <= len {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask) != 0 {
                // Update Z
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Perspective recover
                let q_abs = _mm256_andnot_ps(_mm256_set1_ps(-0.0), q_vec); // abs
                let q_valid = _mm256_cmp_ps(q_abs, _mm256_set1_ps(1e-6), _CMP_GT_OQ);
                let safe_q = _mm256_blendv_ps(one, q_vec, q_valid);

                // Fast reciprocal with Newton-Raphson iteration
                let rcp = _mm256_rcp_ps(safe_q);
                let w_recip = _mm256_mul_ps(rcp, _mm256_sub_ps(two, _mm256_mul_ps(safe_q, rcp)));

                let u_tex_f = _mm256_mul_ps(u_vec, w_recip);
                let v_tex_f = _mm256_mul_ps(v_vec, w_recip);

                // Convert to i32 for gathering
                let u_i = _mm256_cvttps_epi32(u_tex_f);
                let v_i = _mm256_cvttps_epi32(v_tex_f);

                // Calculate texture indices (Vectorized)
                // Optimization: Specialized path for Power-of-Two textures using bitwise masking
                let idx = if texture.width_shift < 32 {
                    let mask_x = _mm256_set1_epi32((texture.width - 1) as i32);
                    let mask_y = _mm256_set1_epi32((texture.height - 1) as i32);
                    let shift_vec = _mm256_set1_epi32(texture.width_shift as i32);

                    // wrap: u & (w-1)
                    let u_masked = _mm256_and_si256(u_i, mask_x);
                    let v_masked = _mm256_and_si256(v_i, mask_y);

                    // idx = (v << shift) | u
                    _mm256_or_si256(_mm256_sllv_epi32(v_masked, shift_vec), u_masked)
                } else {
                    // Generic path with clamping
                    let w_vec = _mm256_set1_epi32(texture.width as i32);
                    let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
                    let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
                    let zero_i = _mm256_setzero_si256();

                    // clamp(val, 0, max)
                    let u_clamped = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_clamped = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);

                    // idx = v * w + u
                    _mm256_add_epi32(_mm256_mullo_epi32(v_clamped, w_vec), u_clamped)
                };

                // Gather Diffuse
                let diff_base = texture.pixels.as_ptr() as *const i32;
                let diff_packed = _mm256_i32gather_epi32(diff_base, idx, 4);

                // Gather Normal Map
                let nm_packed =
                    if normal_map.width == texture.width && normal_map.height == texture.height {
                        let nm_base = normal_map.pixels.as_ptr() as *const i32;
                        _mm256_i32gather_epi32(nm_base, idx, 4)
                    } else {
                        let idx_nm = if normal_map.width_shift < 32 {
                            let mask_x = _mm256_set1_epi32((normal_map.width - 1) as i32);
                            let mask_y = _mm256_set1_epi32((normal_map.height - 1) as i32);
                            let shift_vec = _mm256_set1_epi32(normal_map.width_shift as i32);
                            let u_masked = _mm256_and_si256(u_i, mask_x);
                            let v_masked = _mm256_and_si256(v_i, mask_y);
                            _mm256_or_si256(_mm256_sllv_epi32(v_masked, shift_vec), u_masked)
                        } else {
                            let w_vec = _mm256_set1_epi32(normal_map.width as i32);
                            let max_x = _mm256_set1_epi32((normal_map.width - 1) as i32);
                            let max_y = _mm256_set1_epi32((normal_map.height - 1) as i32);
                            let zero_i = _mm256_setzero_si256();
                            let u_clamped = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                            let v_clamped = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                            _mm256_add_epi32(_mm256_mullo_epi32(v_clamped, w_vec), u_clamped)
                        };
                        let nm_base = normal_map.pixels.as_ptr() as *const i32;
                        _mm256_i32gather_epi32(nm_base, idx_nm, 4)
                    };

                // Unpack Diffuse (0..255 -> 0.0..1.0)
                let r_mask_i32 = _mm256_set1_epi32(0xFF);

                let diff_r_i = _mm256_and_si256(_mm256_srli_epi32(diff_packed, 16), r_mask_i32);
                let diff_g_i = _mm256_and_si256(_mm256_srli_epi32(diff_packed, 8), r_mask_i32);
                let diff_b_i = _mm256_and_si256(diff_packed, r_mask_i32);

                let diff_r_tex = _mm256_mul_ps(_mm256_cvtepi32_ps(diff_r_i), inv_255);
                let diff_g_tex = _mm256_mul_ps(_mm256_cvtepi32_ps(diff_g_i), inv_255);
                let diff_b_tex = _mm256_mul_ps(_mm256_cvtepi32_ps(diff_b_i), inv_255);

                // Unpack Normal Map (0..255 -> -1.0..1.0)
                let nm_r_i = _mm256_and_si256(_mm256_srli_epi32(nm_packed, 16), r_mask_i32);
                let nm_g_i = _mm256_and_si256(_mm256_srli_epi32(nm_packed, 8), r_mask_i32);
                let nm_b_i = _mm256_and_si256(nm_packed, r_mask_i32);

                let nm_x = _mm256_fmsub_ps(_mm256_cvtepi32_ps(nm_r_i), scale_nm, one);
                let nm_y = _mm256_fmsub_ps(_mm256_cvtepi32_ps(nm_g_i), scale_nm, one);
                let nm_z = _mm256_fmsub_ps(_mm256_cvtepi32_ps(nm_b_i), scale_nm, one);

                let len_sq = _mm256_add_ps(
                    _mm256_mul_ps(lx_vec, lx_vec),
                    _mm256_add_ps(_mm256_mul_ps(ly_vec, ly_vec), _mm256_mul_ps(lz_vec, lz_vec)),
                );

                let len_valid = _mm256_cmp_ps(len_sq, epsilon, _CMP_GT_OQ);
                let safe_len_sq = _mm256_blendv_ps(one, len_sq, len_valid);
                let rsqrt = _mm256_rsqrt_ps(safe_len_sq);
                let iter1 = _mm256_mul_ps(safe_len_sq, _mm256_mul_ps(rsqrt, rsqrt));
                let iter2 = _mm256_sub_ps(one_point_five, _mm256_mul_ps(zero_point_five, iter1));
                let inv_len = _mm256_mul_ps(rsqrt, iter2);

                let dot = _mm256_add_ps(
                    _mm256_mul_ps(nm_x, lx_vec),
                    _mm256_add_ps(_mm256_mul_ps(nm_y, ly_vec), _mm256_mul_ps(nm_z, lz_vec)),
                );

                let intensity = _mm256_max_ps(zero, _mm256_mul_ps(dot, inv_len));
                let intensity = _mm256_blendv_ps(zero, intensity, len_valid);

                let diffuse_term_r = _mm256_mul_ps(diff_r_const, diff_r_tex);
                let diffuse_term_g = _mm256_mul_ps(diff_g_const, diff_g_tex);
                let diffuse_term_b = _mm256_mul_ps(diff_b_const, diff_b_tex);

                let r_final = _mm256_fmadd_ps(diffuse_term_r, intensity, amb_r);
                let g_final = _mm256_fmadd_ps(diffuse_term_g, intensity, amb_g);
                let b_final = _mm256_fmadd_ps(diffuse_term_b, intensity, amb_b);

                let r_clamp = _mm256_min_ps(
                    _mm256_max_ps(_mm256_mul_ps(r_final, scale_255), zero),
                    scale_255,
                );
                let g_clamp = _mm256_min_ps(
                    _mm256_max_ps(_mm256_mul_ps(g_final, scale_255), zero),
                    scale_255,
                );
                let b_clamp = _mm256_min_ps(
                    _mm256_max_ps(_mm256_mul_ps(b_final, scale_255), zero),
                    scale_255,
                );

                let r_out = _mm256_cvttps_epi32(r_clamp);
                let g_out = _mm256_cvttps_epi32(g_clamp);
                let b_out = _mm256_cvttps_epi32(b_clamp);

                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        _mm256_slli_epi32(r_out, 16),
                        _mm256_or_si256(_mm256_slli_epi32(g_out, 8), b_out),
                    ),
                );

                let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                let old_color = _mm256_loadu_si256(fb_ptr);
                let mask_int = _mm256_castps_si256(mask);
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }

            z_vec = _mm256_add_ps(z_vec, dz_step);
            q_vec = _mm256_add_ps(q_vec, dq_step);
            u_vec = _mm256_add_ps(u_vec, du_step);
            v_vec = _mm256_add_ps(v_vec, dv_step);
            lx_vec = _mm256_add_ps(lx_vec, dlx_step);
            ly_vec = _mm256_add_ps(ly_vec, dly_step);
            lz_vec = _mm256_add_ps(lz_vec, dlz_step);

            i += 8;
        }

        // Scalar tail
        while i < len {
            let i_f = i as f32;
            let z = z + i_f * gradients.dz_dx;
            let q = q + i_f * gradients.dq_dx;
            let u = u + i_f * gradients.du_dx;
            let v = v + i_f * gradients.dv_dx;
            let lx = lx + i_f * gradients.dlx_dx;
            let ly = ly + i_f * gradients.dly_dx;
            let lz = lz + i_f * gradients.dlz_dx;

            let pixel = &mut fb_slice[i];
            let depth_val = &mut zb_slice[i];

            if z < *depth_val {
                *depth_val = z;
                let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
                let u_tex = u * w_recip;
                let v_tex = v * w_recip;

                let diffuse_color_u32 = texture.get_pixel_texel(u_tex as i32, v_tex as i32);
                let diff_r = ((diffuse_color_u32 >> 16) & 0xFF) as f32 / 255.0;
                let diff_g = ((diffuse_color_u32 >> 8) & 0xFF) as f32 / 255.0;
                let diff_b = (diffuse_color_u32 & 0xFF) as f32 / 255.0;
                let diffuse_sample = Vec3::new(diff_r, diff_g, diff_b);

                let nm_color_u32 = normal_map.get_pixel_texel(u_tex as i32, v_tex as i32);
                let nm_r = (((nm_color_u32 >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
                let nm_g = (((nm_color_u32 >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
                let nm_b = ((nm_color_u32 & 0xFF) as f32 / 255.0) * 2.0 - 1.0;

                let len_sq = lx * lx + ly * ly + lz * lz;
                let intensity = if len_sq > 0.0001 {
                    let inv_len = fast_inv_sqrt(len_sq);
                    (nm_r * lx + nm_g * ly + nm_b * lz) * inv_len
                } else {
                    0.0
                }
                .max(0.0);

                let diffuse_total = pre_diffuse_color * diffuse_sample * intensity;
                let final_color_vec = ambient + diffuse_total;
                *pixel = color_to_u32(final_color_vec);
            }
            i += 1;
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: NormalMapSpanStart,
    gradients: &NormalMapGradients,
    texture: &Texture,
    normal_map: &Texture,
    pre_diffuse_color: Vec3, // base_color * light_color
    ambient: Vec3,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    // Local accumulators
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;
    let mut lx = start.lx;
    let mut ly = start.ly;
    let mut lz = start.lz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        lx += diff_f * gradients.dlx_dx;
        ly += diff_f * gradients.dly_dx;
        lz += diff_f * gradients.dlz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY: Clamped above.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_normal_mapped_simd(
                fb_slice,
                zb_slice,
                z,
                q,
                u,
                v,
                lx,
                ly,
                lz,
                gradients,
                texture,
                normal_map,
                pre_diffuse_color,
                ambient,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Perspective recover
            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let u_tex = u * w_recip;
            let v_tex = v * w_recip;

            // Sample diffuse
            let diffuse_color_u32 = texture.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack diffuse to Vec3 (0-1)
            let diff_r = ((diffuse_color_u32 >> 16) & 0xFF) as f32 / 255.0;
            let diff_g = ((diffuse_color_u32 >> 8) & 0xFF) as f32 / 255.0;
            let diff_b = (diffuse_color_u32 & 0xFF) as f32 / 255.0;
            let diffuse_sample = Vec3::new(diff_r, diff_g, diff_b);

            // Sample normal map (Tangent Space Normal)
            let nm_color_u32 = normal_map.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack to [-1, 1]
            let nm_r = (((nm_color_u32 >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_g = (((nm_color_u32 >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_b = ((nm_color_u32 & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            // let tangent_normal = Vec3::new(nm_r, nm_g, nm_b);

            // Light Vector in Tangent Space
            let len_sq = lx * lx + ly * ly + lz * lz;
            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                // Dot product: normal . light
                (nm_r * lx + nm_g * ly + nm_b * lz) * inv_len
            } else {
                0.0
            }
            .max(0.0);

            // Combine
            let diffuse_total = pre_diffuse_color * diffuse_sample * intensity;
            let final_color_vec = ambient + diffuse_total;
            *pixel = color_to_u32(final_color_vec);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        u += gradients.du_dx;
        v += gradients.dv_dx;
        lx += gradients.dlx_dx;
        ly += gradients.dly_dx;
        lz += gradients.dlz_dx;
    }
}

#[allow(clippy::too_many_arguments)]
/// Fill a triangle with Normal Mapping (Bump Mapping).
///
/// Renders a triangle using a diffuse texture and a normal map for detailed surface lighting.
/// The lighting calculation is performed in Tangent Space.
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), UV, Normal, Tangent)`.
///     *   `Position`: Clip Space position.
///     *   `W`: Homogeneous W.
///     *   `UV`: Texture coordinates.
///     *   `Normal`: Model space normal vector.
///     *   `Tangent`: Model space tangent vector (w component stores bitangent handedness).
/// *   `texture`: Diffuse color map (Albedo).
/// *   `normal_map`: Tangent-space normal map (RGB encoded as XYZ).
/// *   `light_dir`: Direction of the light rays (e.g., from light source to scene).
/// *   `light_color`: Color and intensity of the directional light.
/// *   `ambient`: Ambient light color added to the result.
///
/// # Lighting Model
///
/// Uses the Lambertian diffuse model:
/// $$ I = Ambient + (Diffuse \cdot \max(N \cdot L, 0)) $$
/// where $N$ is sampled from the normal map and $L$ is the light vector transformed into Tangent Space.
pub fn fill_triangle_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2, Vec3, Vec4),
    v1: ((Vec3, f32), Vec2, Vec3, Vec4),
    v2: ((Vec3, f32), Vec2, Vec3, Vec4),
    texture: &Texture,
    normal_map: &Texture,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            half_width,
            half_height,
        );

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        // Scale UV by texture size (assuming both maps match size or using one size for ratio)
        // Usually, UVs are 0..1, we multiply by size to get texel coords
        let w = texture.width as f32;
        let h = texture.height as f32;

        let u0 = v0.1.x * w * inv_w0;
        let v0_val = v0.1.y * h * inv_w0;
        let u1 = v1.1.x * w * inv_w1;
        let v1_val = v1.1.y * h * inv_w1;
        let u2 = v2.1.x * w * inv_w2;
        let v2_val = v2.1.y * h * inv_w2;

        // Compute Tangent Space Light Vectors
        let calculate_ts_light = |n: Vec3, t: Vec4| -> Vec3 {
            let n_norm = n.normalize();
            let t_norm = Vec3::new(t.x, t.y, t.z).normalize();
            // Re-orthogonalize T with respect to N (Gram-Schmidt)
            let t_ortho = (t_norm - n_norm * n_norm.dot(t_norm)).normalize();
            let b_ortho = n_norm.cross(t_ortho) * t.w;

            // Transform LightDir to Tangent Space.
            // LightDir passed in is direction of light (sun).
            // We want vector TO light, so -light_dir.
            let l_world = light_dir * -1.0;

            // TS_L = TBN^T * L_world
            Vec3::new(
                t_ortho.dot(l_world),
                b_ortho.dot(l_world),
                n_norm.dot(l_world),
            )
        };

        // Use true normals/tangents (v0.2, v0.3) not scaled by inv_w
        let l0_ts = calculate_ts_light(v0.2, v0.3);
        let l1_ts = calculate_ts_light(v1.2, v1.3);
        let l2_ts = calculate_ts_light(v2.2, v2.3);

        // Prepare for interpolation
        let l0 = l0_ts * inv_w0;
        let l1 = l1_ts * inv_w1;
        let l2 = l2_ts * inv_w2;

        let mut verts = [
            (p0_orig, u0, v0_val, l0),
            (p1_orig, u1, v1_val, l1),
            (p2_orig, u2, v2_val, l2),
        ];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, u0, v0, l0), (p1, u1, v1, l1), (p2, u2, v2, l2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        if total_height == 0.0 {
            continue;
        }

        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
        }

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = NormalMapGradients::new(
            p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2, l0, l1, l2,
        );

        let mut edge_a = NormalMapEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0, v2, l0, l2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = NormalMapEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0, v1, l0, l1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2, l1, l2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let pre_diffuse_color = light_color; // Base color comes from texture

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2, l1, l2);
            }

            // Unpack walker state
            let (x_start, x_end, z_left, q_left, u_left, v_left, lx_left, ly_left, lz_left) =
                if long_edge_is_left {
                    (
                        (edge_a.x >> 16) as i32,
                        (edge_b.x >> 16) as i32,
                        edge_a.z,
                        edge_a.q,
                        edge_a.u,
                        edge_a.v,
                        edge_a.lx,
                        edge_a.ly,
                        edge_a.lz,
                    )
                } else {
                    (
                        (edge_b.x >> 16) as i32,
                        (edge_a.x >> 16) as i32,
                        edge_b.z,
                        edge_b.q,
                        edge_b.u,
                        edge_b.v,
                        edge_b.lx,
                        edge_b.ly,
                        edge_b.lz,
                    )
                };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_normal_mapped(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    NormalMapSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                        lx: lx_left,
                        ly: ly_left,
                        lz: lz_left,
                    },
                    &gradients,
                    texture,
                    normal_map,
                    pre_diffuse_color,
                    ambient,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[derive(Clone, Copy)]
pub struct TexturedGouraudGradients {
    pub dz_dx: f32,
    pub dq_dx: f32,
    pub du_dx: f32,
    pub dv_dx: f32,
    pub dr_dx: f32,
    pub dg_dx: f32,
    pub db_dx: f32,
    // Y gradients for steps
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dr_dy: f32,
    dg_dy: f32,
    db_dy: f32,
}

impl TexturedGouraudGradients {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
        c0: Vec3,
        c1: Vec3,
        c2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let ur = c1.x - c0.x;
        let ug = c1.y - c0.y;
        let ub = c1.z - c0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vr = c2.x - c0.x;
        let vg = c2.y - c0.y;
        let vb = c2.z - c0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        let nx_r = uy * vr - ur * vy;
        let dr_dx = nx_r * inv_nz;

        let nx_g = uy * vg - ug * vy;
        let dg_dx = nx_g * inv_nz;

        let nx_b = uy * vb - ub * vy;
        let db_dx = nx_b * inv_nz;

        // Y gradients
        let ny_q = uq * vx - ux * vq;
        let dq_dy = ny_q * inv_nz;

        let ny_u = uu * vx - ux * vu;
        let du_dy = ny_u * inv_nz;

        let ny_v = uv * vx - ux * vv;
        let dv_dy = ny_v * inv_nz;

        let ny_r = ur * vx - ux * vr;
        let dr_dy = ny_r * inv_nz;

        let ny_g = ug * vx - ux * vg;
        let dg_dy = ny_g * inv_nz;

        let ny_b = ub * vx - ux * vb;
        let db_dy = ny_b * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                du_dx,
                dv_dx,
                dr_dx,
                dg_dx,
                db_dx,
                dq_dy,
                du_dy,
                dv_dy,
                dr_dy,
                dg_dy,
                db_dy,
            },
            nz > 0.0,
        )
    }
}

pub(crate) struct TexturedGouraudEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    pub(crate) r: f32, // r
    pub(crate) g: f32, // g
    pub(crate) b: f32, // b
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dr_dy: f32,
    dg_dy: f32,
    db_dy: f32,
}

impl TexturedGouraudEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
        c_start: Vec3,
        c_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dr_dy = (c_end.x - c_start.x) * inv_h;
        let dg_dy = (c_end.y - c_start.y) * inv_h;
        let db_dy = (c_end.z - c_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            r: c_start.x,
            g: c_start.y,
            b: c_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dr_dy,
            dg_dy,
            db_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.r += self.dr_dy;
        self.g += self.dg_dy;
        self.b += self.db_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.r += self.dr_dy * n_f;
        self.g += self.dg_dy * n_f;
        self.b += self.db_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub struct TexturedGouraudSpanStart {
    pub z: f32,
    pub q: f32,
    pub u: f32,
    pub v: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_span_textured_gouraud_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    z_start: f32,
    dz_dx: f32,
    u_fix_start: i32,
    v_fix_start: i32,
    du_fix: i32,
    dv_fix: i32,
    r_start: f32,
    g_start: f32,
    b_start: f32,
    dr_dx: f32,
    dg_dx: f32,
    db_dx: f32,
) {
    use std::arch::x86_64::*;

    let len = fb_slice.len();
    let mut i = 0;

    let dz_dx_vec = _mm256_set1_ps(dz_dx);
    let du_fix_vec = _mm256_set1_epi32(du_fix);
    let dv_fix_vec = _mm256_set1_epi32(dv_fix);
    let dr_dx_vec = _mm256_set1_ps(dr_dx);
    let dg_dx_vec = _mm256_set1_ps(dg_dx);
    let db_dx_vec = _mm256_set1_ps(db_dx);

    let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
    let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));
    let mut r_vec = _mm256_add_ps(_mm256_set1_ps(r_start), _mm256_mul_ps(dr_dx_vec, offsets_f));
    let mut g_vec = _mm256_add_ps(_mm256_set1_ps(g_start), _mm256_mul_ps(dg_dx_vec, offsets_f));
    let mut b_vec = _mm256_add_ps(_mm256_set1_ps(b_start), _mm256_mul_ps(db_dx_vec, offsets_f));

    let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
    let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);

    let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_fix_start), du_off);
    let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_fix_start), dv_off);

    let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
    let dr_step = _mm256_mul_ps(dr_dx_vec, _mm256_set1_ps(8.0));
    let dg_step = _mm256_mul_ps(dg_dx_vec, _mm256_set1_ps(8.0));
    let db_step = _mm256_mul_ps(db_dx_vec, _mm256_set1_ps(8.0));
    let du_step = _mm256_slli_epi32(du_fix_vec, 3);
    let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

    let w_vec = _mm256_set1_epi32(texture.width as i32);
    let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
    let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
    let zero_i = _mm256_setzero_si256();
    let shift_vec = _mm256_set1_epi32(texture.width_shift as i32);
    let is_pot = texture.width_shift < 32;

    let ff_mask = _mm256_set1_epi32(0xFF);
    let scale_255 = _mm256_set1_ps(255.0);
    let zero_ps = _mm256_setzero_ps();

    while i + 8 <= len {
        let depth_ptr = zb_slice.as_mut_ptr().add(i);
        let depth_val = _mm256_loadu_ps(depth_ptr);
        let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

        if _mm256_movemask_ps(mask_z) != 0 {
            let u_i = _mm256_srai_epi32(u_fix_vec, 16);
            let v_i = _mm256_srai_epi32(v_fix_vec, 16);

            let idx = if is_pot {
                let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
            } else {
                let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
            };

            let pixel_vals = _mm256_i32gather_epi32(texture.pixels.as_ptr() as *const i32, idx, 4);

            let tex_r_i = _mm256_and_si256(_mm256_srli_epi32(pixel_vals, 16), ff_mask);
            let tex_g_i = _mm256_and_si256(_mm256_srli_epi32(pixel_vals, 8), ff_mask);
            let tex_b_i = _mm256_and_si256(pixel_vals, ff_mask);
            let tex_a_i = _mm256_and_si256(_mm256_srli_epi32(pixel_vals, 24), ff_mask);

            let tex_r = _mm256_cvtepi32_ps(tex_r_i);
            let tex_g = _mm256_cvtepi32_ps(tex_g_i);
            let tex_b = _mm256_cvtepi32_ps(tex_b_i);

            let mod_r = _mm256_mul_ps(tex_r, r_vec);
            let mod_g = _mm256_mul_ps(tex_g, g_vec);
            let mod_b = _mm256_mul_ps(tex_b, b_vec);

            let out_r = _mm256_cvttps_epi32(_mm256_min_ps(_mm256_max_ps(mod_r, zero_ps), scale_255));
            let out_g = _mm256_cvttps_epi32(_mm256_min_ps(_mm256_max_ps(mod_g, zero_ps), scale_255));
            let out_b = _mm256_cvttps_epi32(_mm256_min_ps(_mm256_max_ps(mod_b, zero_ps), scale_255));

            let out_color = _mm256_or_si256(
                _mm256_slli_epi32(tex_a_i, 24),
                _mm256_or_si256(
                    _mm256_slli_epi32(out_r, 16),
                    _mm256_or_si256(_mm256_slli_epi32(out_g, 8), out_b)
                )
            );

            // Alpha Masks
            let opaque_mask = _mm256_cmpeq_epi32(tex_a_i, ff_mask);
            let zero_mask = _mm256_cmpeq_epi32(tex_a_i, zero_i);
            let mask_z_int = _mm256_castps_si256(mask_z);

            // Opaque Write
            let write_opaque = _mm256_and_si256(mask_z_int, opaque_mask);
            let write_opaque_ps = _mm256_castsi256_ps(write_opaque);

            if _mm256_movemask_ps(write_opaque_ps) != 0 {
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, write_opaque_ps);
                _mm256_storeu_ps(depth_ptr, new_z);

                let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                let old_color = _mm256_loadu_si256(fb_ptr);
                let new_color = _mm256_blendv_epi8(old_color, out_color, write_opaque);
                _mm256_storeu_si256(fb_ptr, new_color);
            }

            // Translucent Write
            let write_trans = _mm256_andnot_si256(opaque_mask, _mm256_andnot_si256(zero_mask, mask_z_int));
            let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(write_trans));

            if trans_bits != 0 {
                let mut temp_colors = [0u32; 8];
                _mm256_storeu_si256(temp_colors.as_mut_ptr() as *mut __m256i, out_color);

                let mut bit = 1;
                for k in 0..8 {
                    if (trans_bits & bit) != 0 {
                        let idx = i + k;
                        let src = temp_colors[k];
                        let alpha = (src >> 24) as u8;
                        let dest = *fb_slice.get_unchecked(idx);
                        *fb_slice.get_unchecked_mut(idx) = blend_swar(src, dest, (255 - alpha).into(), alpha.into());
                    }
                    bit <<= 1;
                }
            }
        }

        z_vec = _mm256_add_ps(z_vec, dz_step);
        r_vec = _mm256_add_ps(r_vec, dr_step);
        g_vec = _mm256_add_ps(g_vec, dg_step);
        b_vec = _mm256_add_ps(b_vec, db_step);
        u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
        v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
        i += 8;
    }

    draw_span_textured_gouraud_scalar(
        &mut fb_slice[i..],
        &mut zb_slice[i..],
        texture,
        z_start + (i as f32) * dz_dx,
        dz_dx,
        u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32)),
        v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32)),
        du_fix,
        dv_fix,
        r_start + (i as f32) * dr_dx,
        g_start + (i as f32) * dg_dx,
        b_start + (i as f32) * db_dx,
        dr_dx,
        dg_dx,
        db_dx,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_span_textured_gouraud_scalar(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
    mut r: f32,
    mut g: f32,
    mut b: f32,
    dr_dx: f32,
    dg_dx: f32,
    db_dx: f32,
) {
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let shift = texture.width_shift;
    let is_pot = shift < 32;

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            let u = u_fix >> 16;
            let v = v_fix >> 16;

            let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                 if is_pot {
                    unsafe { *tex_pixels.get_unchecked(((v as usize) << shift) + (u as usize)) }
                 } else {
                    unsafe { *tex_pixels.get_unchecked((v as usize) * (tex_w as usize) + (u as usize)) }
                 }
            } else {
                texture.get_pixel_texel(u, v)
            };

            let tex_r = ((color >> 16) & 0xFF) as f32;
            let tex_g = ((color >> 8) & 0xFF) as f32;
            let tex_b = (color & 0xFF) as f32;
            let tex_a = (color >> 24) & 0xFF;

            let final_r = (tex_r * r).clamp(0.0, 255.0) as u32;
            let final_g = (tex_g * g).clamp(0.0, 255.0) as u32;
            let final_b = (tex_b * b).clamp(0.0, 255.0) as u32;

            let final_color = ((tex_a as u32) << 24) | (final_r << 16) | (final_g << 8) | final_b;

            if tex_a == 255 {
                *depth_val = z;
                *pixel = final_color;
            } else if tex_a > 0 {
                let dest = *pixel;
                *pixel = blend_swar(final_color, dest, (255 - (tex_a as u8)).into(), (tex_a as u8).into());
            }
        }
        z += dz_dx;
        u_fix = u_fix.wrapping_add(du_fix);
        v_fix = v_fix.wrapping_add(dv_fix);
        r += dr_dx;
        g += dg_dx;
        b += db_dx;
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_textured_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: TexturedGouraudSpanStart,
    gradients: &TexturedGouraudGradients,
    texture: &Texture,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;
    let mut r = start.r;
    let mut g = start.g;
    let mut b = start.b;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        r += diff_f * gradients.dr_dx;
        g += diff_f * gradients.dg_dx;
        b += diff_f * gradients.db_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    // Span-based optimization
    let span_size = 16;
    let mut x = xs;

    // Calculate initial start values for perspective correction
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
    let mut u_tex_start = u * w_start;
    let mut v_tex_start = v * w_start;

    while x <= xe {
        let remaining = xe - x + 1;
        let count = remaining.min(span_size);

        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        let w_end = if q_end.abs() > 0.000_001 { 1.0 / q_end } else { 1.0 };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        let inv_count = RECIPROCAL_TABLE[count as usize];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        // Color interpolation (Linear)
        let r_end = r + gradients.dr_dx * count as f32;
        let g_end = g + gradients.dg_dx * count as f32;
        let b_end = b + gradients.db_dx * count as f32;

        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (x as usize);
        let end_idx = y_offset + ((x + count - 1) as usize);

        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        if matches!(texture.filter_mode, FilterMode::Nearest) {
             let u_fix = (u_tex_start * 65536.0) as i32;
             let v_fix = (v_tex_start * 65536.0) as i32;
             let du_fix = (du_tex_step * 65536.0) as i32;
             let dv_fix = (dv_tex_step * 65536.0) as i32;

             #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
             if is_x86_feature_detected!("avx2") {
                 unsafe {
                     draw_span_textured_gouraud_simd(
                         fb_slice, zb_slice, texture,
                         z, gradients.dz_dx,
                         u_fix, v_fix, du_fix, dv_fix,
                         r, g, b, gradients.dr_dx, gradients.dg_dx, gradients.db_dx
                     );
                 }
             } else {
                 draw_span_textured_gouraud_scalar(
                     fb_slice, zb_slice, texture,
                     z, gradients.dz_dx,
                     u_fix, v_fix, du_fix, dv_fix,
                     r, g, b, gradients.dr_dx, gradients.dg_dx, gradients.db_dx
                 );
             }
             #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
             draw_span_textured_gouraud_scalar(
                 fb_slice, zb_slice, texture,
                 z, gradients.dz_dx,
                 u_fix, v_fix, du_fix, dv_fix,
                 r, g, b, gradients.dr_dx, gradients.dg_dx, gradients.db_dx
             );
        } else {
             let mut z_curr = z;
             let mut r_curr = r;
             let mut g_curr = g;
             let mut b_curr = b;

             for i in 0..count {
                 let pixel = unsafe { fb_slice.get_unchecked_mut(i as usize) };
                 let depth_val = unsafe { zb_slice.get_unchecked_mut(i as usize) };

                 if z_curr < *depth_val {
                    let q_curr = q + (i as f32) * gradients.dq_dx;
                    let u_curr_p = u + (i as f32) * gradients.du_dx;
                    let v_curr_p = v + (i as f32) * gradients.dv_dx;

                    let w_recip = if q_curr.abs() > 0.000_001 { 1.0 / q_curr } else { 1.0 };
                    let u_tex = u_curr_p * w_recip;
                    let v_tex = v_curr_p * w_recip;

                    let color = match texture.filter_mode {
                        FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                        FilterMode::Trilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                        _ => texture.get_pixel_texel(u_tex as i32, v_tex as i32),
                    };

                    let tex_r = ((color >> 16) & 0xFF) as f32;
                    let tex_g = ((color >> 8) & 0xFF) as f32;
                    let tex_b = (color & 0xFF) as f32;
                    let tex_a = (color >> 24) & 0xFF;

                    let final_r = (tex_r * r_curr).clamp(0.0, 255.0) as u32;
                    let final_g = (tex_g * g_curr).clamp(0.0, 255.0) as u32;
                    let final_b = (tex_b * b_curr).clamp(0.0, 255.0) as u32;

                    let final_color = ((tex_a as u32) << 24) | (final_r << 16) | (final_g << 8) | final_b;

                    if tex_a == 255 {
                        *depth_val = z_curr;
                        *pixel = final_color;
                    } else if tex_a > 0 {
                        let dest = *pixel;
                        *pixel = blend_swar(final_color, dest, (255 - (tex_a as u8)).into(), (tex_a as u8).into());
                    }
                 }
                 z_curr += gradients.dz_dx;
                 r_curr += gradients.dr_dx;
                 g_curr += gradients.dg_dx;
                 b_curr += gradients.db_dx;
             }
        }

        z += gradients.dz_dx * count as f32;
        q = q_end;
        u = u_end;
        v = v_end;
        r = r_end;
        g = g_end;
        b = b_end;
        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;
        x += count;
    }
}

/// Fill a textured triangle with Gouraud shading (Vertex Color Modulation).
///
/// This function combines a texture lookup with linearly interpolated vertex colors.
/// The texture color is multiplied by the interpolated vertex color.
///
/// $$ Pixel = Texture(u, v) \cdot Interpolated(VertexColor) $$
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Color, UV)`.
///     *   `Position`: Clip Space position.
///     *   `W`: Homogeneous W.
///     *   `Color`: RGB color (0.0 - 1.0).
///     *   `UV`: Texture coordinates.
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_textured_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec2),
    v1: ((Vec3, f32), Vec3, Vec2),
    v2: ((Vec3, f32), Vec3, Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            half_width,
            half_height,
        );

        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let w = texture.width as f32;
        let h = texture.height as f32;

        let u0 = v0.2.x * w * inv_w0;
        let v0_val = v0.2.y * h * inv_w0;
        let u1 = v1.2.x * w * inv_w1;
        let v1_val = v1.2.y * h * inv_w1;
        let u2 = v2.2.x * w * inv_w2;
        let v2_val = v2.2.y * h * inv_w2;

        let c0 = v0.1;
        let c1 = v1.1;
        let c2 = v2.1;

        let mut verts = [
            (p0_orig, u0, v0_val, c0),
            (p1_orig, u1, v1_val, c1),
            (p2_orig, u2, v2_val, c2),
        ];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, u0, v0, c0), (p1, u1, v1, c1), (p2, u2, v2, c2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        if total_height == 0.0 {
            continue;
        }

        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
        }

        let (gradients, long_edge_is_left) = TexturedGouraudGradients::new(
            p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2, c0, c1, c2,
        );

        let mut edge_a = TexturedGouraudEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0, v2, c0, c2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = TexturedGouraudEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0, v1, c0, c1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = TexturedGouraudEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2, c1, c2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = TexturedGouraudEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2, c1, c2);
            }

            let (x_start, x_end, z_left, q_left, u_left, v_left, r_left, g_left, b_left) =
                if long_edge_is_left {
                    (
                        (edge_a.x >> 16) as i32,
                        (edge_b.x >> 16) as i32,
                        edge_a.z,
                        edge_a.q,
                        edge_a.u,
                        edge_a.v,
                        edge_a.r,
                        edge_a.g,
                        edge_a.b,
                    )
                } else {
                    (
                        (edge_b.x >> 16) as i32,
                        (edge_a.x >> 16) as i32,
                        edge_b.z,
                        edge_b.q,
                        edge_b.u,
                        edge_b.v,
                        edge_b.r,
                        edge_b.g,
                        edge_b.b,
                    )
                };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_textured_gouraud(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    TexturedGouraudSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                        r: r_left,
                        g: g_left,
                        b: b_left,
                    },
                    &gradients,
                    texture,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}
