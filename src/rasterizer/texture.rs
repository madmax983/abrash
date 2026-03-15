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
use crate::math::{
    ScreenPoint, Vec2, Vec3, Vec4, fast_inv_sqrt, project_quad_to_screen,
    project_triangle_to_screen,
};
use crate::texture::{FilterMode, Texture, blend_four_way, blend_swar};
use crate::zbuffer::ZBuffer;

use super::core::{FIXED_SCALE, assert_same_dimensions, color_to_u32, is_backface, sort_by_y};

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
use super::core::blend_swar_simd;

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
        let inv_nz = if nz.abs() > 0.000_1 { -1.0 / nz } else { 0.0 };

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
pub(crate) fn draw_span_nearest(
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

    // Optimization: Check if the entire span is within texture bounds to avoid per-pixel checks.
    let len = fb_slice.len() as i32;
    let can_use_fast_path = if len > 0 {
        // Calculate range of u_fix and v_fix using i64 to prevent wrap-around bypassing bounds checks.
        let u_start_64 = i64::from(u_fix);
        let du_64 = i64::from(du_fix);
        let u_end_64 = u_start_64 + du_64 * i64::from(len - 1);

        let (u_min_64, u_max_64) = if du_64 >= 0 {
            (u_start_64, u_end_64)
        } else {
            (u_end_64, u_start_64)
        };

        let v_start_64 = i64::from(v_fix);
        let dv_64 = i64::from(dv_fix);
        let v_end_64 = v_start_64 + dv_64 * i64::from(len - 1);

        let (v_min_64, v_max_64) = if dv_64 >= 0 {
            (v_start_64, v_end_64)
        } else {
            (v_end_64, v_start_64)
        };

        // Check bounds
        // (val >> 16) is the integer coordinate.
        u_min_64 >= 0
            && (u_max_64 >> 16) < i64::from(tex_w)
            && v_min_64 >= 0
            && (v_max_64 >> 16) < i64::from(tex_h)
    } else {
        false
    };

    let shift = texture.width_shift;

    macro_rules! process_span_nearest {
        ($fetch_block:block) => {
            for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                if z < *depth_val {
                    let color = $fetch_block;

                    let alpha = (color >> 24) & 0xFF;
                    if alpha == 255 {
                        *depth_val = z;
                        *pixel = color;
                    } else if alpha > 0 {
                        let dest = *pixel;
                        // Correct blending: src * alpha + dest * (1 - alpha)
                        // blend_swar(c0, c1, w, inv_w) -> c0 * inv_w + c1 * w
                        // So w = 255 - alpha, inv_w = alpha
                        *pixel = blend_swar(color, dest, 255 - alpha, alpha);
                    }
                }
                z += dz_dx;
                u_fix = u_fix.wrapping_add(du_fix);
                v_fix = v_fix.wrapping_add(dv_fix);
            }
        };
    }

    if can_use_fast_path {
        // FAST PATH: No bounds checks inside loop
        if shift < 32 {
            process_span_nearest!({
                let u = (u_fix >> 16) as usize;
                let v = (v_fix >> 16) as usize;
                // SAFETY: Verified entire span is within bounds.
                unsafe { *tex_pixels.get_unchecked((v << shift) + u) }
            });
        } else {
            process_span_nearest!({
                let u = (u_fix >> 16) as usize;
                let v = (v_fix >> 16) as usize;
                // SAFETY: Verified entire span is within bounds.
                unsafe { *tex_pixels.get_unchecked(v * tex_w_usize + u) }
            });
        }
    } else {
        // SLOW PATH: Per-pixel bounds checks (handling repeat/clamp/overflow)
        if shift < 32 {
            process_span_nearest!({
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                if (u as u32) < tex_w && (v as u32) < tex_h {
                    // SAFETY: Checked bounds
                    unsafe { *tex_pixels.get_unchecked(((v as usize) << shift) + (u as usize)) }
                } else {
                    texture.get_pixel_texel(u, v)
                }
            });
        } else {
            process_span_nearest!({
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                if (u as u32) < tex_w && (v as u32) < tex_h {
                    // SAFETY: Checked bounds
                    unsafe { *tex_pixels.get_unchecked((v as usize) * tex_w_usize + (u as usize)) }
                } else {
                    texture.get_pixel_texel(u, v)
                }
            });
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_span_bilinear(
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

    // Optimization: Check if the entire span is within texture bounds to avoid per-pixel checks.
    let len = fb_slice.len() as i32;
    let can_use_fast_path = if len > 0 {
        // Calculate range of u_fix and v_fix using i64 to prevent wrap-around bypassing bounds checks.
        let u_start_64 = i64::from(u_fix);
        let du_64 = i64::from(du_fix);
        let u_end_64 = u_start_64 + du_64 * i64::from(len - 1);

        let (u_min_64, u_max_64) = if du_64 >= 0 {
            (u_start_64, u_end_64)
        } else {
            (u_end_64, u_start_64)
        };

        let v_start_64 = i64::from(v_fix);
        let dv_64 = i64::from(dv_fix);
        let v_end_64 = v_start_64 + dv_64 * i64::from(len - 1);

        let (v_min_64, v_max_64) = if dv_64 >= 0 {
            (v_start_64, v_end_64)
        } else {
            (v_end_64, v_start_64)
        };

        // Check bounds for Bilinear (width-1)
        // (val >> 16) is the integer coordinate x0.
        // We need x0 < width - 1 (so x0+1 < width)
        u_min_64 >= 0
            && (u_max_64 >> 16) < i64::from(w_i32)
            && v_min_64 >= 0
            && (v_max_64 >> 16) < i64::from(h_i32)
    } else {
        false
    };

    macro_rules! process_span_bilinear {
        ($op:tt, $val:expr, $fast_path:literal) => {
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
                            if $fast_path || ((x0_raw as u32) < (w_i32 as u32) && (y0_raw as u32) < (h_i32 as u32)) {
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

    if can_use_fast_path {
        if shift < 32 {
            process_span_bilinear!(<<, shift, true);
        } else {
            process_span_bilinear!(*, tex_w_usize, true);
        }
    } else {
        if shift < 32 {
            process_span_bilinear!(<<, shift, false);
        } else {
            process_span_bilinear!(*, tex_w_usize, false);
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
#[allow(clippy::wildcard_imports)]
pub(crate) unsafe fn draw_span_bilinear_simd(
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
    use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};

    let len = fb_slice.len();
    let mut i = 0;

    // Align pixels buffer
    let align_mask = 0x1F;
    let addr = fb_slice.as_ptr() as usize;
    let misalign = addr & align_mask;
    let pre_simd_count = if misalign == 0 {
        0
    } else {
        (32 - misalign) / 4
    };
    let pre_simd_count = pre_simd_count.min(len);

    let mut z_curr = z_start;
    let mut u_curr = u_fix_start;
    let mut v_curr = v_fix_start;

    // Scalar pre-loop
    for k in 0..pre_simd_count {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(k);
            if z_curr < *depth_val {
                // Inline scalar bilinear
                let u_img_fixed = u_curr >> 8;
                let v_img_fixed = v_curr >> 8;
                let x0_raw = u_img_fixed >> 8;
                let y0_raw = v_img_fixed >> 8;

                let tex_w = texture.width;
                let tex_h = texture.height;
                let w_i32 = (tex_w as i32).wrapping_sub(1);
                let h_i32 = (tex_h as i32).wrapping_sub(1);
                let tex_w_usize = tex_w as usize;

                let (c00, c10, c01, c11) =
                    if (x0_raw as u32) < (w_i32 as u32) && (y0_raw as u32) < (h_i32 as u32) {
                        let x0 = x0_raw as usize;
                        let y0 = y0_raw as usize;
                        let row0 = if texture.width_shift < 32 {
                            y0 << texture.width_shift
                        } else {
                            y0 * tex_w_usize
                        };
                        let row1 = row0 + tex_w_usize;
                        (
                            *texture.pixels.get_unchecked(row0 + x0),
                            *texture.pixels.get_unchecked(row0 + x0 + 1),
                            *texture.pixels.get_unchecked(row1 + x0),
                            *texture.pixels.get_unchecked(row1 + x0 + 1),
                        )
                    } else {
                        let x0 = x0_raw.clamp(0, w_i32) as usize;
                        let y0 = y0_raw.clamp(0, h_i32) as usize;
                        let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
                        let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

                        let row0 = if texture.width_shift < 32 {
                            y0 << texture.width_shift
                        } else {
                            y0 * tex_w_usize
                        };
                        let row1 = if texture.width_shift < 32 {
                            y1 << texture.width_shift
                        } else {
                            y1 * tex_w_usize
                        };

                        (
                            *texture.pixels.get_unchecked(row0 + x0),
                            *texture.pixels.get_unchecked(row0 + x1),
                            *texture.pixels.get_unchecked(row1 + x0),
                            *texture.pixels.get_unchecked(row1 + x1),
                        )
                    };

                let wx = (u_img_fixed & 0xFF) as u32;
                let wy = (v_img_fixed & 0xFF) as u32;

                let final_color = blend_four_way(c00, c10, c01, c11, wx, wy);
                let alpha = (final_color >> 24) & 0xFF;

                if alpha == 255 {
                    *depth_val = z_curr;
                    *fb_slice.get_unchecked_mut(k) = final_color;
                } else if alpha > 0 {
                    let dest = *fb_slice.get_unchecked(k);
                    *fb_slice.get_unchecked_mut(k) =
                        blend_swar(final_color, dest, 255 - alpha, alpha);
                }
            }
        }
        z_curr += dz_dx;
        u_curr = u_curr.wrapping_add(du_fix);
        v_curr = v_curr.wrapping_add(dv_fix);
    }

    i += pre_simd_count;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_curr), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_curr), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_curr), dv_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let one_i = _mm256_set1_epi32(1);

        let shift_vec = _mm256_set1_epi32(i32::from(texture.width_shift));
        let is_pot = texture.width_shift < 32;

        let mask_ff = _mm256_set1_epi32(0xFF);

        macro_rules! process_bilinear_loop {
            ($is_pot_const:literal) => {
                while i + 8 <= len {
                    // Z-Test
                    let depth_ptr = zb_slice.as_mut_ptr().add(i);
                    let depth_val = _mm256_loadu_ps(depth_ptr);

                    // Optimization: Early Out if fully occluded
                    let ge_mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_GE_OQ);
                    let ge_bits = _mm256_movemask_ps(ge_mask);

                    if ge_bits != 0xFF {
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
                        // Optimization: Use bitwise shifts for Power-of-Two textures
                        let (idx00, idx10, idx01, idx11) = if $is_pot_const {
                            let y0_shifted = _mm256_sllv_epi32(y0, shift_vec);
                            let y1_shifted = _mm256_sllv_epi32(y1, shift_vec);
                            (
                                _mm256_or_si256(y0_shifted, x0),
                                _mm256_or_si256(y0_shifted, x1),
                                _mm256_or_si256(y1_shifted, x0),
                                _mm256_or_si256(y1_shifted, x1),
                            )
                        } else {
                            let y0_w = _mm256_mullo_epi32(y0, w_vec);
                            let y1_w = _mm256_mullo_epi32(y1, w_vec);
                            (
                                _mm256_add_epi32(y0_w, x0),
                                _mm256_add_epi32(y0_w, x1),
                                _mm256_add_epi32(y1_w, x0),
                                _mm256_add_epi32(y1_w, x1),
                            )
                        };

                        // Gather
                        let pixels_ptr = texture.pixels.as_ptr().cast::<i32>();
                        let c00 = _mm256_i32gather_epi32(pixels_ptr, idx00, 4);
                        let c10 = _mm256_i32gather_epi32(pixels_ptr, idx10, 4);
                        let c01 = _mm256_i32gather_epi32(pixels_ptr, idx01, 4);
                        let c11 = _mm256_i32gather_epi32(pixels_ptr, idx11, 4);

                        let top = blend_swar_simd(c00, c10, wx, inv_wx);
                        let bot = blend_swar_simd(c01, c11, wx, inv_wx);
                        let final_color = blend_swar_simd(top, bot, wy, inv_wy);

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

                            let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                            let old_color = _mm256_load_si256(fb_ptr); // Aligned load
                            let new_color =
                                _mm256_blendv_epi8(old_color, final_color, write_opaque_mask);
                            _mm256_store_si256(fb_ptr, new_color); // Aligned store
                        }

                        // Translucent
                        let zero_mask = _mm256_cmpeq_epi32(a, zero_i);
                        let trans_mask = _mm256_andnot_si256(
                            opaque_mask,
                            _mm256_andnot_si256(zero_mask, mask_z_int),
                        );
                        let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(trans_mask));

                        if trans_bits != 0 {
                            // Load current framebuffer (might have been updated by opaque write)
                            // Aligned load/store for pixels
                            let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                            let current_dest = _mm256_load_si256(fb_ptr); // Aligned load

                            // Alpha blending: src * alpha + dest * (1 - alpha)
                            // blend_swar(c0, c1, w, inv_w) -> c0 * inv_w + c1 * w
                            // So w = 255 - alpha, inv_w = alpha
                            // We want src * alpha + dest * (1-alpha)
                            // blend_swar_simd(src, dest, inv_alpha, alpha)

                            let alpha_src = a; // Already extracted: (final_color >> 24) & 0xFF
                            let inv_alpha_src = _mm256_sub_epi32(const_256, alpha_src);

                            let blended = blend_swar_simd(
                                final_color,
                                current_dest,
                                inv_alpha_src,
                                alpha_src,
                            );

                            // Only write where trans_mask is set.
                            // blendv_epi8(a, b, mask): if mask bit is 1, take b.
                            let result = _mm256_blendv_epi8(current_dest, blended, trans_mask);
                            _mm256_store_si256(fb_ptr, result); // Aligned store
                        }
                    }
                    } // End early-out block

                    z_vec = _mm256_add_ps(z_vec, dz_step);
                    u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
                    v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
                    i += 8;
                }
            };
        }

        if is_pot {
            process_bilinear_loop!(true);
        } else {
            process_bilinear_loop!(false);
        }
    }

    // Scalar Tail
    if i < len {
        // Recalculate currents based on i (which advanced)
        let z_tail = z_start + (i as f32) * dz_dx;
        let u_tail = u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32));
        let v_tail = v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32));

        draw_span_bilinear(
            &mut fb_slice[i..],
            &mut zb_slice[i..],
            texture,
            z_tail,
            dz_dx,
            u_tail,
            v_tail,
            du_fix,
            dv_fix,
        );
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_span_trilinear(
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
#[allow(clippy::wildcard_imports)]
pub(crate) unsafe fn draw_span_nearest_simd(
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
    use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};

    let len = fb_slice.len();
    let mut i = 0;

    // Align buffer
    let align_mask = 0x1F;
    let addr = fb_slice.as_ptr() as usize;
    let misalign = addr & align_mask;
    let pre_simd_count = if misalign == 0 {
        0
    } else {
        (32 - misalign) / 4
    };
    let pre_simd_count = pre_simd_count.min(len);

    let mut z_curr = z_start;
    let mut u_curr = u_fix_start;
    let mut v_curr = v_fix_start;

    // Scalar pre-loop
    for k in 0..pre_simd_count {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(k);
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
                    *fb_slice.get_unchecked_mut(k) = color;
                } else if alpha > 0 {
                    let dest = *fb_slice.get_unchecked(k);
                    *fb_slice.get_unchecked_mut(k) = blend_swar(color, dest, 255 - alpha, alpha);
                }
            }
        }
        z_curr += dz_dx;
        u_curr = u_curr.wrapping_add(du_fix);
        v_curr = v_curr.wrapping_add(dv_fix);
    }

    i += pre_simd_count;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_curr), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_curr), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_curr), dv_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let ff_mask_shifted = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let shift_vec = _mm256_set1_epi32(i32::from(texture.width_shift));

        let is_pot = texture.width_shift < 32;

        while i + 8 <= len {
            // Z-Test
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Optimization: Early Out if fully occluded
            let ge_mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_GE_OQ);
            let ge_bits = _mm256_movemask_ps(ge_mask);

            if ge_bits != 0xFF {
                let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
                let mask_z_int = _mm256_castps_si256(mask_z);

                if _mm256_movemask_ps(mask_z) != 0 {
                    // Calculate Indices
                    let u_i = _mm256_srai_epi32(u_fix_vec, 16);
                    let v_i = _mm256_srai_epi32(v_fix_vec, 16);



                        let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    let idx = if is_pot {
                        // Note: We clamp to match the scalar implementation (draw_span_nearest / get_pixel_texel).
                        // Although wrapping is faster and standard for PoT, we must preserve rendering parity.
                        // The existing `draw_scanline_normal_mapped_simd` uses wrapping, but that creates
                        // an inconsistency with its own scalar fallback. We choose to be consistent with scalar here.


                        _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                    } else {


                        _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                    };

                    // Gather
                    let pixel_vals =
                        _mm256_i32gather_epi32(texture.pixels.as_ptr().cast::<i32>(), idx, 4);

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

                        let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                        let old_color = _mm256_load_si256(fb_ptr); // Aligned load
                        let new_color =
                            _mm256_blendv_epi8(old_color, pixel_vals, write_opaque_mask);
                        _mm256_store_si256(fb_ptr, new_color); // Aligned store
                    }

                    // Translucent: (mask_z & !opaque & !zero)
                    let trans_mask = _mm256_andnot_si256(
                        opaque_mask,
                        _mm256_andnot_si256(zero_mask, mask_z_int),
                    );
                    let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(trans_mask));

                    if trans_bits != 0 {
                        let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                        let current_dest = _mm256_load_si256(fb_ptr); // Aligned load

                        let const_256 = _mm256_set1_epi32(256);
                        let alpha_src = _mm256_and_si256(alphas_shifted, ff_mask_shifted);
                        let alpha_src = _mm256_srli_epi32(alpha_src, 24);
                        let inv_alpha_src = _mm256_sub_epi32(const_256, alpha_src);

                        let blended =
                            blend_swar_simd(pixel_vals, current_dest, inv_alpha_src, alpha_src);

                        let result = _mm256_blendv_epi8(current_dest, blended, trans_mask);
                        _mm256_store_si256(fb_ptr, result); // Aligned store
                    }
                } // End inner mask_z check
            } // End early out

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

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
#[allow(clippy::wildcard_imports)]
unsafe fn draw_scanline_textured_perspective_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    gradients: &PerspectiveTextureGradients,
    texture: &Texture,
) {
    unsafe {
        use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256};

        let len = fb_slice.len();
        let mut i = 0;

        // Load gradients
        let dz_dx = _mm256_set1_ps(gradients.dz_dx);
        let dq_dx = _mm256_set1_ps(gradients.dq_dx);
        let du_dx = _mm256_set1_ps(gradients.du_dx);
        let dv_dx = _mm256_set1_ps(gradients.dv_dx);

        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx, offsets));
        let mut q_vec = _mm256_add_ps(_mm256_set1_ps(q), _mm256_mul_ps(dq_dx, offsets));
        let mut u_vec = _mm256_add_ps(_mm256_set1_ps(u), _mm256_mul_ps(du_dx, offsets));
        let mut v_vec = _mm256_add_ps(_mm256_set1_ps(v), _mm256_mul_ps(dv_dx, offsets));

        let step_8 = _mm256_set1_ps(8.0);
        let dz_step = _mm256_mul_ps(dz_dx, step_8);
        let dq_step = _mm256_mul_ps(dq_dx, step_8);
        let du_step = _mm256_mul_ps(du_dx, step_8);
        let dv_step = _mm256_mul_ps(dv_dx, step_8);

        let one = _mm256_set1_ps(1.0);
        let two = _mm256_set1_ps(2.0);
        let scale_256 = _mm256_set1_ps(256.0);
        let offset_neg_0_5 = _mm256_set1_ps(-128.0); // -0.5 * 256

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let one_i = _mm256_set1_epi32(1);
        let const_256_i = _mm256_set1_epi32(256);
        let mask_ff = _mm256_set1_epi32(0xFF);

        while i + 8 <= len {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask_z) != 0 {
                // Perspective recover
                let q_abs = _mm256_andnot_ps(_mm256_set1_ps(-0.0), q_vec);
                let q_valid = _mm256_cmp_ps(q_abs, _mm256_set1_ps(1e-6), _CMP_GT_OQ);
                let safe_q = _mm256_blendv_ps(one, q_vec, q_valid);

                let rcp = _mm256_rcp_ps(safe_q);
                let w_recip = _mm256_mul_ps(rcp, _mm256_sub_ps(two, _mm256_mul_ps(safe_q, rcp)));

                let u_tex_f = _mm256_mul_ps(u_vec, w_recip);
                let v_tex_f = _mm256_mul_ps(v_vec, w_recip);

                // Bilinear Coordinates (24.8 format)
                // val = u * 256 - 128
                let u_val = _mm256_add_ps(_mm256_mul_ps(u_tex_f, scale_256), offset_neg_0_5);
                let v_val = _mm256_add_ps(_mm256_mul_ps(v_tex_f, scale_256), offset_neg_0_5);

                let u_floor = _mm256_floor_ps(u_val);
                let v_floor = _mm256_floor_ps(v_val);

                let u_int = _mm256_cvtps_epi32(u_floor);
                let v_int = _mm256_cvtps_epi32(v_floor);

                let wx = _mm256_and_si256(u_int, mask_ff);
                let wy = _mm256_and_si256(v_int, mask_ff);
                let inv_wx = _mm256_sub_epi32(const_256_i, wx);
                let inv_wy = _mm256_sub_epi32(const_256_i, wy);

                let x0_raw = _mm256_srai_epi32(u_int, 8);
                let y0_raw = _mm256_srai_epi32(v_int, 8);

                // Clamping
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

                // Gather
                let y0_w = _mm256_mullo_epi32(y0, w_vec);
                let y1_w = _mm256_mullo_epi32(y1, w_vec);

                let idx00 = _mm256_add_epi32(y0_w, x0);
                let idx10 = _mm256_add_epi32(y0_w, x1);
                let idx01 = _mm256_add_epi32(y1_w, x0);
                let idx11 = _mm256_add_epi32(y1_w, x1);

                let pixels_ptr = texture.pixels.as_ptr().cast::<i32>();
                let c00 = _mm256_i32gather_epi32(pixels_ptr, idx00, 4);
                let c10 = _mm256_i32gather_epi32(pixels_ptr, idx10, 4);
                let c01 = _mm256_i32gather_epi32(pixels_ptr, idx01, 4);
                let c11 = _mm256_i32gather_epi32(pixels_ptr, idx11, 4);

                let top = blend_swar_simd(c00, c10, wx, inv_wx);
                let bot = blend_swar_simd(c01, c11, wx, inv_wx);
                let final_color = blend_swar_simd(top, bot, wy, inv_wy);

                // Blending / Writing
                let a = _mm256_and_si256(_mm256_srli_epi32(final_color, 24), mask_ff);
                let opaque_mask = _mm256_cmpeq_epi32(a, mask_ff);
                let mask_z_int = _mm256_castps_si256(mask_z);
                let write_opaque_mask = _mm256_and_si256(mask_z_int, opaque_mask);
                let write_opaque_mask_ps = _mm256_castsi256_ps(write_opaque_mask);

                if _mm256_movemask_ps(write_opaque_mask_ps) != 0 {
                    let old_z = _mm256_loadu_ps(depth_ptr);
                    let new_z = _mm256_blendv_ps(old_z, z_vec, write_opaque_mask_ps);
                    _mm256_storeu_ps(depth_ptr, new_z);

                    let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
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
                    let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                    let current_dest = _mm256_loadu_si256(fb_ptr);
                    let alpha_src = a;
                    let inv_alpha_src = _mm256_sub_epi32(const_256_i, alpha_src);
                    let blended =
                        blend_swar_simd(final_color, current_dest, inv_alpha_src, alpha_src);
                    let result = _mm256_blendv_epi8(current_dest, blended, trans_mask);
                    _mm256_storeu_si256(fb_ptr, result);
                }
            }

            z_vec = _mm256_add_ps(z_vec, dz_step);
            q_vec = _mm256_add_ps(q_vec, dq_step);
            u_vec = _mm256_add_ps(u_vec, du_step);
            v_vec = _mm256_add_ps(v_vec, dv_step);
            i += 8;
        }

        // Scalar Tail
        while i < len {
            let i_f = i as f32;
            let z_curr = z + i_f * gradients.dz_dx;
            let q_curr = q + i_f * gradients.dq_dx;
            let u_curr = u + i_f * gradients.du_dx;
            let v_curr = v + i_f * gradients.dv_dx;

            let depth_val = zb_slice.get_unchecked_mut(i);
            if z_curr < *depth_val {
                let w = if q_curr.abs() > 1e-6 {
                    1.0 / q_curr
                } else {
                    1.0
                };
                let u_tex = u_curr * w;
                let v_tex = v_curr * w;

                let color = texture.get_pixel_bilinear_texel(u_tex, v_tex);

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z_curr;
                    *fb_slice.get_unchecked_mut(i) = color;
                } else if alpha > 0 {
                    let dest = *fb_slice.get_unchecked(i);
                    *fb_slice.get_unchecked_mut(i) = blend_swar(color, dest, 255 - alpha, alpha);
                }
            }
            i += 1;
        }
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
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

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

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if texture.filter_mode == FilterMode::Bilinear && is_x86_feature_detected!("avx2") {
        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (xs as usize);
        let end_idx = y_offset + (xe as usize);

        // SAFETY: xs and xe are clamped to [0, width-1] and xs <= xe.
        unsafe {
            let fb_slice = fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx);
            let zb_slice = zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx);
            draw_scanline_textured_perspective_simd(
                fb_slice, zb_slice, z, q, u, v, gradients, texture,
            );
        }
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

                #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_trilinear_simd(
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
                } else {
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

                #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
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
/// let texture = Texture::new(32, 32).unwrap(); // Assume empty texture
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

    let clipped = clip_triangle_to_frustum(
        v0,
        v1,
        v2,
        |v| v.0,
        |a, b, t| {
            (
                (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                a.1.lerp(b.1, t),
            )
        },
    );

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped[base];
        let v1 = clipped[base + 1];
        let v2 = clipped[base + 2];
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

        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let u0 = v0.1.x * texture.width as f32 * inv_w0;
        let v0_val = v0.1.y * texture.height as f32 * inv_w0;

        let u1 = v1.1.x * texture.width as f32 * inv_w1;
        let v1_val = v1.1.y * texture.height as f32 * inv_w1;

        let u2 = v2.1.x * texture.width as f32 * inv_w2;
        let v2_val = v2.1.y * texture.height as f32 * inv_w2;

        fill_projected_triangle_textured(
            fb, zb, p0_orig, p1_orig, p2_orig, u0, v0_val, u1, v1_val, u2, v2_val, texture,
        );
    }
}

/// Fill a textured quad with Gouraud shading.
///
/// Optimized for quads that are fully within the view frustum.
#[allow(clippy::too_many_arguments)]
pub fn fill_quad_textured_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec2),
    v1: ((Vec3, f32), Vec3, Vec2),
    v2: ((Vec3, f32), Vec3, Vec2),
    v3: ((Vec3, f32), Vec3, Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    // Trivial Acceptance Check: All inside frustum
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    let all_inside = unsafe {
        use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32};
        let x_vec = _mm_set_ps(v3.0.0.x, v2.0.0.x, v1.0.0.x, v0.0.0.x);
        let y_vec = _mm_set_ps(v3.0.0.y, v2.0.0.y, v1.0.0.y, v0.0.0.y);
        let z_vec = _mm_set_ps(v3.0.0.z, v2.0.0.z, v1.0.0.z, v0.0.0.z);
        let w_vec = _mm_set_ps(v3.0.1, v2.0.1, v1.0.1, v0.0.1);
        let neg_w = _mm_sub_ps(_mm_setzero_ps(), w_vec);

        let x_ok = _mm_and_ps(
            _mm_cmp_ps(x_vec, neg_w, _CMP_GE_OQ),
            _mm_cmp_ps(x_vec, w_vec, _CMP_LE_OQ),
        );
        let y_ok = _mm_and_ps(
            _mm_cmp_ps(y_vec, neg_w, _CMP_GE_OQ),
            _mm_cmp_ps(y_vec, w_vec, _CMP_LE_OQ),
        );
        let z_ok = _mm_and_ps(
            _mm_cmp_ps(z_vec, neg_w, _CMP_GE_OQ),
            _mm_cmp_ps(z_vec, w_vec, _CMP_LE_OQ),
        );

        let all_ok = _mm_and_ps(x_ok, _mm_and_ps(y_ok, z_ok));
        _mm_movemask_ps(all_ok) == 0xF
    };

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    let all_inside = {
        let is_inside = |v: (Vec3, f32)| -> bool {
            let (p, w) = v;
            p.x >= -w && p.x <= w && p.y >= -w && p.y <= w && p.z >= -w && p.z <= w
        };
        is_inside(v0.0) && is_inside(v1.0) && is_inside(v2.0) && is_inside(v3.0)
    };

    if all_inside {
        // Fast Path
        let width = fb.width();
        let height = fb.height();
        let half_width = width as f32 * 0.5;
        let half_height = height as f32 * 0.5;

        let (p0, p1, p2, p3) = project_quad_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            v3.0.0,
            v3.0.1,
            half_width,
            half_height,
        );

        let tex_w = texture.width as f32;
        let tex_h = texture.height as f32;

        let u0 = v0.2.x * tex_w * p0.inv_w;
        let v0_val = v0.2.y * tex_h * p0.inv_w;

        let u1 = v1.2.x * tex_w * p1.inv_w;
        let v1_val = v1.2.y * tex_h * p1.inv_w;

        let u2 = v2.2.x * tex_w * p2.inv_w;
        let v2_val = v2.2.y * tex_h * p2.inv_w;

        let u3 = v3.2.x * tex_w * p3.inv_w;
        let v3_val = v3.2.y * tex_h * p3.inv_w;

        // Calculate gradients once for the quad plane using the first triangle (0, 1, 2).
        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        // Backface Culling (Check Tri 1, assuming Planar Quad)
        if is_backface(p0, p1, p2) {
            return;
        }

        // Note argument order for gradients: u0, u1, u2, then v0, v1, v2
        let (gradients, _) = TexturedGouraudGradients::new(
            p0, p1, p2, q0, q1, q2, u0, u1, u2, v0_val, v1_val, v2_val, v0.1, v1.1, v2.1,
        );

        // Render two triangles using the shared gradients: (0, 1, 2) and (0, 2, 3)
        fill_projected_triangle_textured_gouraud_with_gradients(
            fb, zb, p0, p1, p2, u0, v0_val, u1, v1_val, u2, v2_val, v0.1, v1.1, v2.1, texture,
            &gradients,
        );

        fill_projected_triangle_textured_gouraud_with_gradients(
            fb, zb, p0, p2, p3, u0, v0_val, u2, v2_val, u3, v3_val, v0.1, v2.1, v3.1, texture,
            &gradients,
        );
    } else {
        // Fallback
        fill_triangle_textured_gouraud(fb, zb, v0, v1, v2, texture);
        fill_triangle_textured_gouraud(fb, zb, v0, v2, v3, texture);
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn fill_projected_triangle_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    p0_orig: ScreenPoint,
    p1_orig: ScreenPoint,
    p2_orig: ScreenPoint,
    u0_in: f32,
    v0_in: f32,
    u1_in: f32,
    v1_in: f32,
    u2_in: f32,
    v2_in: f32,
    texture: &Texture,
) {
    // Backface Culling
    if is_backface(p0_orig, p1_orig, p2_orig) {
        return;
    }

    // Gradients
    // Calculate gradients based on the triangle plane.
    // We compute this on unsorted vertices.
    // Gradients are invariant to vertex order for the same plane.
    let q0 = p0_orig.inv_w;
    let q1 = p1_orig.inv_w;
    let q2 = p2_orig.inv_w;

    let (gradients, _) = PerspectiveTextureGradients::new_with_winding(
        p0_orig, p1_orig, p2_orig, q0, q1, q2, u0_in, u1_in, u2_in, v0_in, v1_in, v2_in,
    );

    fill_projected_triangle_textured_with_gradients(
        fb, zb, p0_orig, p1_orig, p2_orig, u0_in, v0_in, u1_in, v1_in, u2_in, v2_in, texture,
        gradients, true,
    );
}

#[inline(always)]
fn calculate_signed_area_doubled(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> f32 {
    let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
    let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
    let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
    let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    ux * vy - uy * vx
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn fill_projected_triangle_textured_with_gradients(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    p0: ScreenPoint,
    p1: ScreenPoint,
    p2: ScreenPoint,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    u2: f32,
    v2: f32,
    texture: &Texture,
    gradients: PerspectiveTextureGradients,
    check_backface: bool,
) {
    if check_backface && is_backface(p0, p1, p2) {
        return;
    }

    let width = fb.width();
    let height = fb.height();

    let mut verts = [(p0, u0, v0), (p1, u1, v1), (p2, u2, v2)];
    sort_by_y(&mut verts, |(p, _, _)| p.y);
    let [(p0, u0, v0), (p1, u1, v1), (p2, u2, v2)] = verts;

    let q0 = p0.inv_w;
    let q1 = p1.inv_w;
    let q2 = p2.inv_w;

    let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    if total_height == 0.0 {
        return;
    }

    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = p0.y.max(y_min);
    let y_end = p2.y.min(y_max);

    if y_start > y_end {
        return;
    }

    // Determine winding for Edge Walking setup
    // We already have gradients, but we need to know which edge is "long" (left or right).
    // This depends on the signed area of the *sorted* triangle.
    let nz = calculate_signed_area_doubled(p0, p1, p2);
    let long_edge_is_left = nz > 0.0;

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
                            FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                            FilterMode::Trilinear => {
                                let w = 1.0 / q_left;
                                let w_sq = w * w;

                                let du_tex_dx =
                                    (gradients.du_dx * q_left - u_left * gradients.dq_dx) * w_sq;
                                let dv_tex_dx =
                                    (gradients.dv_dx * q_left - v_left * gradients.dq_dx) * w_sq;
                                let du_tex_dy =
                                    (gradients.du_dy * q_left - u_left * gradients.dq_dy) * w_sq;
                                let dv_tex_dy =
                                    (gradients.dv_dy * q_left - v_left * gradients.dq_dy) * w_sq;

                                let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
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

/// Fill a textured quad (e.g., particle) with perspective correction.
///
/// Optimized for quads that are fully within the view frustum.
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2`, `v3` - Vertices defined as `((Position, W), UV)`.
///     The quad is assumed to be composed of two triangles: `(v0, v1, v2)` and `(v0, v2, v3)`.
pub fn fill_quad_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2),
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    v3: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    // Trivial Acceptance Check: All inside frustum
    // -w <= x,y,z <= w
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    let all_inside = unsafe {
        use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};
        // Note: _mm_set_ps arguments are reversed: (e3, e2, e1, e0)
        let x_vec = _mm_set_ps(v3.0.0.x, v2.0.0.x, v1.0.0.x, v0.0.0.x);
        let y_vec = _mm_set_ps(v3.0.0.y, v2.0.0.y, v1.0.0.y, v0.0.0.y);
        let z_vec = _mm_set_ps(v3.0.0.z, v2.0.0.z, v1.0.0.z, v0.0.0.z);
        let w_vec = _mm_set_ps(v3.0.1, v2.0.1, v1.0.1, v0.0.1);
        let neg_w = _mm_sub_ps(_mm_setzero_ps(), w_vec);

        // Check -w <= val <= w
        // equivalent to: val >= -w AND val <= w
        let x_ok = _mm_and_ps(
            _mm_cmp_ps(x_vec, neg_w, _CMP_GE_OQ),
            _mm_cmp_ps(x_vec, w_vec, _CMP_LE_OQ),
        );
        let y_ok = _mm_and_ps(
            _mm_cmp_ps(y_vec, neg_w, _CMP_GE_OQ),
            _mm_cmp_ps(y_vec, w_vec, _CMP_LE_OQ),
        );
        let z_ok = _mm_and_ps(
            _mm_cmp_ps(z_vec, neg_w, _CMP_GE_OQ),
            _mm_cmp_ps(z_vec, w_vec, _CMP_LE_OQ),
        );

        let all_ok = _mm_and_ps(x_ok, _mm_and_ps(y_ok, z_ok));
        _mm_movemask_ps(all_ok) == 0xF
    };

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    let all_inside = {
        let is_inside = |v: (Vec3, f32)| -> bool {
            let (p, w) = v;
            p.x >= -w && p.x <= w && p.y >= -w && p.y <= w && p.z >= -w && p.z <= w
        };
        is_inside(v0.0) && is_inside(v1.0) && is_inside(v2.0) && is_inside(v3.0)
    };

    if all_inside {
        // Fast Path: Project and Rasterize directly
        let width = fb.width();
        let height = fb.height();
        let half_width = width as f32 * 0.5;
        let half_height = height as f32 * 0.5;

        // Project all 4 (SIMD)
        let (p0, p1, p2, p3) = project_quad_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            v3.0.0,
            v3.0.1,
            half_width,
            half_height,
        );

        // Precompute UVs
        let tex_w = texture.width as f32;
        let tex_h = texture.height as f32;

        let u0 = v0.1.x * tex_w * p0.inv_w;
        let v0_val = v0.1.y * tex_h * p0.inv_w;

        let u1 = v1.1.x * tex_w * p1.inv_w;
        let v1_val = v1.1.y * tex_h * p1.inv_w;

        let u2 = v2.1.x * tex_w * p2.inv_w;
        let v2_val = v2.1.y * tex_h * p2.inv_w;

        let u3 = v3.1.x * tex_w * p3.inv_w;
        let v3_val = v3.1.y * tex_h * p3.inv_w;

        // Calculate gradients once for the quad plane using the first triangle (0, 1, 2).
        // Since the quad is coplanar, these gradients apply to the second triangle (0, 2, 3) as well.
        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        // Note argument order for gradients: u0, u1, u2, then v0, v1, v2
        let (gradients, is_front_facing) = PerspectiveTextureGradients::new_with_winding(
            p0, p1, p2, q0, q1, q2, u0, u1, u2, v0_val, v1_val, v2_val,
        );

        if !is_front_facing {
            return;
        }

        // Render two triangles using the shared gradients: (0, 1, 2) and (0, 2, 3)
        // Note argument order for fill: u0, v0, u1, v1, u2, v2
        // We skip backface checking because we already confirmed winding for the plane.
        fill_projected_triangle_textured_with_gradients(
            fb, zb, p0, p1, p2, u0, v0_val, u1, v1_val, u2, v2_val, texture, gradients, false,
        );

        fill_projected_triangle_textured_with_gradients(
            fb, zb, p0, p2, p3, u0, v0_val, u2, v2_val, u3, v3_val, texture, gradients, false,
        );
    } else {
        // Fallback: Split and Clip
        // Tri 1
        fill_triangle_textured(fb, zb, v0, v1, v2, texture);
        // Tri 2
        fill_triangle_textured(fb, zb, v0, v2, v3, texture);
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
        use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};

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
                    let shift_vec = _mm256_set1_epi32(i32::from(texture.width_shift));

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
                let diff_base = texture.pixels.as_ptr().cast::<i32>();
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
                            let shift_vec = _mm256_set1_epi32(i32::from(normal_map.width_shift));
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

                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
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
                let intensity = if len_sq > 0.000_1 {
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

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
#[allow(clippy::wildcard_imports)]
pub(crate) unsafe fn draw_span_trilinear_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    z_start: f32,
    dz_dx: f32,
    u_fix_start: i32,
    v_fix_start: i32,
    du_fix: i32,
    dv_fix: i32,
    lod: f32,
) {
    use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};

    if lod <= 0.0 || texture.mips.is_empty() {
        // Fallback to bilinear if LOD is 0 or no mips
        // u_fix_start is 16.16 (center). Bilinear simd expects 16.16 (offset -0.5).
        let u_fix = u_fix_start.wrapping_sub(32768);
        let v_fix = v_fix_start.wrapping_sub(32768);
        unsafe {
            draw_span_bilinear_simd(
                fb_slice, zb_slice, texture, z_start, dz_dx, u_fix, v_fix, du_fix, dv_fix,
            );
        }
        return;
    }

    let max_level = texture.mips.len() as f32;
    // We can't easily fallback to "just sample max mip" with bilinear SIMD because the
    // texture pointer changes.
    // So we handle all cases here or dispatch carefully.

    // Clamp LOD
    let lod_clamped = lod.max(0.0).min(max_level);
    let level_f = lod_clamped.floor();
    let frac = lod_clamped - level_f;
    let level = level_f as usize;

    // Calculate weight for blending (0..256)
    let weight_int = (frac * 256.0) as i32;
    let inv_weight_int = 256 - weight_int;
    let weight_vec = _mm256_set1_epi32(weight_int);
    let inv_weight_vec = _mm256_set1_epi32(inv_weight_int);

    // Identify the two levels
    // Level 0 is texture.pixels
    // Level K > 0 is texture.mips[K-1]

    let (pixels0, w0, h0, shift0) = if level == 0 {
        (
            texture.pixels.as_slice(),
            texture.width,
            texture.height,
            0, // base shift is 0 relative to u_fix >> 8 (bilinear)
               // But u_fix passed in is 16.16. Bilinear expects 24.8.
               // So effectively shift is 8.
        )
    } else {
        let idx = level - 1;
        let w = (texture.width >> level).max(1);
        let h = (texture.height >> level).max(1);
        (texture.mips[idx].as_slice(), w, h, level)
    };

    let (pixels1, w1, h1, shift1) = if level >= texture.mips.len() {
        // If we are at max level, we blend with itself (or clamped)
        // This happens if lod == max_level.
        let idx = texture.mips.len() - 1;
        let w = (texture.width >> (idx + 1)).max(1);
        let h = (texture.height >> (idx + 1)).max(1);
        (texture.mips[idx].as_slice(), w, h, idx + 1)
    } else {
        // Next level is level + 1 (mips[level])
        let idx = level;
        let w = (texture.width >> (level + 1)).max(1);
        let h = (texture.height >> (level + 1)).max(1);
        (texture.mips[idx].as_slice(), w, h, level + 1)
    };

    // Prepare SIMD constants
    let len = fb_slice.len();
    let mut i = 0;

    // Align buffer
    let align_mask = 0x1F;
    let addr = fb_slice.as_ptr() as usize;
    let misalign = addr & align_mask;
    let pre_simd_count = if misalign == 0 {
        0
    } else {
        (32 - misalign) / 4
    };
    let pre_simd_count = pre_simd_count.min(len);

    let mut z_curr = z_start;
    let mut u_curr = u_fix_start;
    let mut v_curr = v_fix_start;

    // Scalar pre-loop
    for k in 0..pre_simd_count {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(k);
            if z_curr < *depth_val {
                let color = texture.get_pixel_trilinear_fixed(u_curr, v_curr, lod);
                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z_curr;
                    *fb_slice.get_unchecked_mut(k) = color;
                } else if alpha > 0 {
                    let dest = *fb_slice.get_unchecked(k);
                    *fb_slice.get_unchecked_mut(k) = blend_swar(color, dest, 255 - alpha, alpha);
                }
            }
        }
        z_curr += dz_dx;
        u_curr = u_curr.wrapping_add(du_fix);
        v_curr = v_curr.wrapping_add(dv_fix);
    }

    i += pre_simd_count;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_curr), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_curr), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_curr), dv_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let mask_ff = _mm256_set1_epi32(0xFF);
        let zero_i = _mm256_setzero_si256();
        let one_i = _mm256_set1_epi32(1);
        let const_256 = _mm256_set1_epi32(256);

        // Level 0 constants
        let w0_vec = _mm256_set1_epi32(w0 as i32);
        let max_x0 = _mm256_set1_epi32((w0 as i32).wrapping_sub(1));
        let max_y0 = _mm256_set1_epi32((h0 as i32).wrapping_sub(1));

        // Level 1 constants
        let w1_vec = _mm256_set1_epi32(w1 as i32);
        let max_x1 = _mm256_set1_epi32((w1 as i32).wrapping_sub(1));
        let max_y1 = _mm256_set1_epi32((h1 as i32).wrapping_sub(1));

        // Shift amounts. u_fix is 16.16.
        // For Level 0 (base): shift right 8 to get 24.8.
        // For Level L: shift right 8 + L.
        let shift_amt0 = _mm_cvtsi32_si128(8 + shift0 as i32);
        let shift_amt1 = _mm_cvtsi32_si128(8 + shift1 as i32);

        // Generic Bilinear Gather Macro
        // It takes u_img, v_img (24.8 format), width constants, and pixel pointer
        // Returns packed colors
        macro_rules! sample_level {
            ($u_img:expr, $v_img:expr, $w_vec:expr, $max_x:expr, $max_y:expr, $pixels_ptr:expr) => {{
                let wx = _mm256_and_si256($u_img, mask_ff);
                let wy = _mm256_and_si256($v_img, mask_ff);
                let inv_wx = _mm256_sub_epi32(const_256, wx);
                let inv_wy = _mm256_sub_epi32(const_256, wy);

                let x0_raw = _mm256_srai_epi32($u_img, 8);
                let y0_raw = _mm256_srai_epi32($v_img, 8);

                let x0 = _mm256_min_epi32(_mm256_max_epi32(x0_raw, zero_i), $max_x);
                let y0 = _mm256_min_epi32(_mm256_max_epi32(y0_raw, zero_i), $max_y);
                let x1 = _mm256_min_epi32(
                    _mm256_max_epi32(_mm256_add_epi32(x0_raw, one_i), zero_i),
                    $max_x,
                );
                let y1 = _mm256_min_epi32(
                    _mm256_max_epi32(_mm256_add_epi32(y0_raw, one_i), zero_i),
                    $max_y,
                );

                let y0_w = _mm256_mullo_epi32(y0, $w_vec);
                let y1_w = _mm256_mullo_epi32(y1, $w_vec);

                let idx00 = _mm256_add_epi32(y0_w, x0);
                let idx10 = _mm256_add_epi32(y0_w, x1);
                let idx01 = _mm256_add_epi32(y1_w, x0);
                let idx11 = _mm256_add_epi32(y1_w, x1);

                let ptr = $pixels_ptr as *const i32;
                let c00 = _mm256_i32gather_epi32(ptr, idx00, 4);
                let c10 = _mm256_i32gather_epi32(ptr, idx10, 4);
                let c01 = _mm256_i32gather_epi32(ptr, idx01, 4);
                let c11 = _mm256_i32gather_epi32(ptr, idx11, 4);

                let top = blend_swar_simd(c00, c10, wx, inv_wx);
                let bot = blend_swar_simd(c01, c11, wx, inv_wx);
                blend_swar_simd(top, bot, wy, inv_wy)
            }};
        }

        while i + 8 <= len {
            // Z-Test
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Optimization: Early Out if fully occluded
            let ge_mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_GE_OQ);
            let ge_bits = _mm256_movemask_ps(ge_mask);

            if ge_bits != 0xFF {
                let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

                if _mm256_movemask_ps(mask_z) != 0 {
                    // Apply half-pixel offset (-0.5 in 16.16 is 32768)
                    let offset = _mm256_set1_epi32(32768);
                    let u_shifted = _mm256_sub_epi32(u_fix_vec, offset);
                    let v_shifted = _mm256_sub_epi32(v_fix_vec, offset);

                    // Level 0 Sampling
                    let u_img0 = _mm256_sra_epi32(u_shifted, shift_amt0);
                    let v_img0 = _mm256_sra_epi32(v_shifted, shift_amt0);
                    let color0 =
                        sample_level!(u_img0, v_img0, w0_vec, max_x0, max_y0, pixels0.as_ptr());

                    // Level 1 Sampling
                    let u_img1 = _mm256_sra_epi32(u_shifted, shift_amt1);
                    let v_img1 = _mm256_sra_epi32(v_shifted, shift_amt1);
                    let color1 =
                        sample_level!(u_img1, v_img1, w1_vec, max_x1, max_y1, pixels1.as_ptr());

                    // Trilinear Blend
                    let final_color = blend_swar_simd(color0, color1, weight_vec, inv_weight_vec);

                    // Write Opaque/Translucent (Same as bilinear)
                    let a = _mm256_and_si256(_mm256_srli_epi32(final_color, 24), mask_ff);
                    let opaque_mask = _mm256_cmpeq_epi32(a, mask_ff);
                    let mask_z_int = _mm256_castps_si256(mask_z);
                    let write_opaque_mask = _mm256_and_si256(mask_z_int, opaque_mask);
                    let write_opaque_mask_ps = _mm256_castsi256_ps(write_opaque_mask);

                    if _mm256_movemask_ps(write_opaque_mask_ps) != 0 {
                        let old_z = _mm256_loadu_ps(depth_ptr);
                        let new_z = _mm256_blendv_ps(old_z, z_vec, write_opaque_mask_ps);
                        _mm256_storeu_ps(depth_ptr, new_z);

                        let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                        let old_color = _mm256_load_si256(fb_ptr); // Aligned load
                        let new_color =
                            _mm256_blendv_epi8(old_color, final_color, write_opaque_mask);
                        _mm256_store_si256(fb_ptr, new_color); // Aligned store
                    }

                    let zero_mask = _mm256_cmpeq_epi32(a, zero_i);
                    let trans_mask = _mm256_andnot_si256(
                        opaque_mask,
                        _mm256_andnot_si256(zero_mask, mask_z_int),
                    );
                    let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(trans_mask));

                    if trans_bits != 0 {
                        let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                        let current_dest = _mm256_load_si256(fb_ptr); // Aligned load

                        let alpha_src = a;
                        let inv_alpha_src = _mm256_sub_epi32(const_256, alpha_src);

                        let blended =
                            blend_swar_simd(final_color, current_dest, inv_alpha_src, alpha_src);

                        let result = _mm256_blendv_epi8(current_dest, blended, trans_mask);
                        _mm256_store_si256(fb_ptr, result); // Aligned store
                    }
                } // End inner mask_z check
            } // End early out

            z_vec = _mm256_add_ps(z_vec, dz_step);
            u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
            v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
            i += 8;
        }
    }
    // Scalar Tail
    if i < len {
        let z_tail = z_start + (i as f32) * dz_dx;
        let u_tail = u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32));
        let v_tail = v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32));

        draw_span_trilinear(
            &mut fb_slice[i..],
            &mut zb_slice[i..],
            texture,
            z_tail,
            dz_dx,
            u_tail,
            v_tail,
            du_fix,
            dv_fix,
            lod,
        );
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
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

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
            let intensity = if len_sq > 0.000_1 {
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

    let clipped = clip_triangle_to_frustum(
        v0,
        v1,
        v2,
        |v| v.0,
        |a, b, t| {
            (
                (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                a.1.lerp(b.1, t),
                a.2.lerp(b.2, t),
                a.3.lerp(b.3, t),
            )
        },
    );

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped[base];
        let v1 = clipped[base + 1];
        let v2 = clipped[base + 2];

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
        let (gradients, long_edge_is_left) =
            NormalMapGradients::new(p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2, l0, l1, l2);

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
    pub dq_dy: f32,
    pub du_dy: f32,
    pub dv_dy: f32,
    pub dr_dy: f32,
    pub dg_dy: f32,
    pub db_dy: f32,
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
#[allow(clippy::wildcard_imports)]
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
    r_start: i32,
    g_start: i32,
    b_start: i32,
    dr_dx: i32,
    dg_dx: i32,
    db_dx: i32,
) {
    use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};

    let len = fb_slice.len();
    let mut i = 0;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);
        let dr_dx_vec = _mm256_set1_epi32(dr_dx);
        let dg_dx_vec = _mm256_set1_epi32(dg_dx);
        let db_dx_vec = _mm256_set1_epi32(db_dx);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);
        let dr_off = _mm256_mullo_epi32(dr_dx_vec, offsets_i);
        let dg_off = _mm256_mullo_epi32(dg_dx_vec, offsets_i);
        let db_off = _mm256_mullo_epi32(db_dx_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_fix_start), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_fix_start), dv_off);
        let mut r_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(r_start), dr_off);
        let mut g_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(g_start), dg_off);
        let mut b_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(b_start), db_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let dr_step = _mm256_slli_epi32(dr_dx_vec, 3);
        let dg_step = _mm256_slli_epi32(dg_dx_vec, 3);
        let db_step = _mm256_slli_epi32(db_dx_vec, 3);
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let shift_vec = _mm256_set1_epi32(i32::from(texture.width_shift));
        let is_pot = texture.width_shift < 32;

        let ff_mask = _mm256_set1_epi32(0xFF);
        let mask_255 = _mm256_set1_epi32(255);

        while i + 8 <= len {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask_z) != 0 {
                let u_i = _mm256_srai_epi32(u_fix_vec, 16);
                let v_i = _mm256_srai_epi32(v_fix_vec, 16);



                        let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    let idx = if is_pot {


                    _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                } else {


                    _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                };

                let pixel_vals =
                    _mm256_i32gather_epi32(texture.pixels.as_ptr().cast::<i32>(), idx, 4);

                let tex_r_i = _mm256_and_si256(_mm256_srli_epi32(pixel_vals, 16), ff_mask);
                let tex_g_i = _mm256_and_si256(_mm256_srli_epi32(pixel_vals, 8), ff_mask);
                let tex_b_i = _mm256_and_si256(pixel_vals, ff_mask);
                let tex_a_i = _mm256_and_si256(_mm256_srli_epi32(pixel_vals, 24), ff_mask);

                // Use max(0, r) to clamp lower bound of shade.
                let r_clamped = _mm256_max_epi32(r_fix_vec, zero_i);
                let g_clamped = _mm256_max_epi32(g_fix_vec, zero_i);
                let b_clamped = _mm256_max_epi32(b_fix_vec, zero_i);

                // Multiply: (tex * shade)
                let mod_r = _mm256_mullo_epi32(tex_r_i, r_clamped);
                let mod_g = _mm256_mullo_epi32(tex_g_i, g_clamped);
                let mod_b = _mm256_mullo_epi32(tex_b_i, b_clamped);

                // Shift right 16 (divide by 65536) and clamp to 255
                let out_r = _mm256_min_epi32(_mm256_srai_epi32(mod_r, 16), mask_255);
                let out_g = _mm256_min_epi32(_mm256_srai_epi32(mod_g, 16), mask_255);
                let out_b = _mm256_min_epi32(_mm256_srai_epi32(mod_b, 16), mask_255);

                let out_color = _mm256_or_si256(
                    _mm256_slli_epi32(tex_a_i, 24),
                    _mm256_or_si256(
                        _mm256_slli_epi32(out_r, 16),
                        _mm256_or_si256(_mm256_slli_epi32(out_g, 8), out_b),
                    ),
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

                    let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                    let old_color = _mm256_loadu_si256(fb_ptr);
                    let new_color = _mm256_blendv_epi8(old_color, out_color, write_opaque);
                    _mm256_storeu_si256(fb_ptr, new_color);
                }

                // Translucent Write
                let write_trans =
                    _mm256_andnot_si256(opaque_mask, _mm256_andnot_si256(zero_mask, mask_z_int));
                let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(write_trans));

                if trans_bits != 0 {
                    let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                    let current_dest = _mm256_loadu_si256(fb_ptr);

                    // Alpha blending: src * alpha + dest * inv_alpha
                    // blend_swar_simd(c0, c1, w, inv_w) -> c0 * inv_w + c1 * w
                    // c0 = src, inv_w = alpha
                    // c1 = dest, w = inv_alpha

                    let const_256 = _mm256_set1_epi32(256);
                    let alpha_src = tex_a_i;
                    let inv_alpha_src = _mm256_sub_epi32(const_256, alpha_src);

                    let blended =
                        blend_swar_simd(out_color, current_dest, inv_alpha_src, alpha_src);

                    let result = _mm256_blendv_epi8(current_dest, blended, write_trans);
                    _mm256_storeu_si256(fb_ptr, result);
                }
            }

            z_vec = _mm256_add_ps(z_vec, dz_step);
            r_fix_vec = _mm256_add_epi32(r_fix_vec, dr_step);
            g_fix_vec = _mm256_add_epi32(g_fix_vec, dg_step);
            b_fix_vec = _mm256_add_epi32(b_fix_vec, db_step);
            u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
            v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
            i += 8;
        }
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
        r_start.wrapping_add(dr_dx.wrapping_mul(i as i32)),
        g_start.wrapping_add(dg_dx.wrapping_mul(i as i32)),
        b_start.wrapping_add(db_dx.wrapping_mul(i as i32)),
        dr_dx,
        dg_dx,
        db_dx,
    );
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::ptr_as_ptr)]
#[allow(clippy::wildcard_imports)]
unsafe fn draw_span_textured_gouraud_bilinear_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    z_start: f32,
    dz_dx: f32,
    u_fix_start: i32,
    v_fix_start: i32,
    du_fix: i32,
    dv_fix: i32,
    r_start: i32,
    g_start: i32,
    b_start: i32,
    dr_dx: i32,
    dg_dx: i32,
    db_dx: i32,
) {
    use std::arch::x86_64::{_mm256_sub_epi32, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_srai_epi32, _mm256_set_epi32, _mm256_set1_ps, _mm256_set_ps, _mm256_add_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_set1_epi32, _mm256_loadu_ps, _mm256_cmp_ps, _CMP_LT_OQ, _mm256_movemask_ps, _mm256_blendv_ps, _mm256_storeu_ps, _mm256_andnot_ps, _CMP_GT_OQ, _mm256_rcp_ps, _mm256_sub_ps, _mm256_cvttps_epi32, _mm256_and_si256, _mm256_or_si256, _mm256_sllv_epi32, _mm256_setzero_si256, _mm256_min_epi32, _mm256_max_epi32, _mm256_add_epi32, _mm256_mullo_epi32, _mm256_i32gather_epi32, _mm256_srli_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_slli_epi32, __m256i, _mm256_loadu_si256, _mm256_castps_si256, _mm256_blendv_epi8, _mm256_storeu_si256, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_cmpeq_epi32, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_sub_epi32, _mm_set_ps, _mm_sub_ps, _mm_setzero_ps, _mm_cmp_ps, _mm_and_ps, _mm_movemask_ps, _CMP_GE_OQ, _CMP_LE_OQ, _mm256_max_epi32, _mm256_sub_ps, _mm256_floor_ps, _mm256_cvtps_epi32, _mm256_cvtepi32_ps, _mm256_fmsub_ps, _mm256_rsqrt_ps, _mm256_max_ps, _mm256_fmadd_ps, _mm256_min_ps, _mm256_load_si256, _mm256_store_si256, _mm256_castsi256_ps, _mm256_andnot_si256, _mm256_cmpeq_epi32, _mm256_sra_epi32, _mm_cvtsi32_si128, _mm256_srai_epi32, _mm256_add_epi16, _mm256_mullo_epi16, _mm256_srli_epi16, _mm256_blendv_ps};

    let len = fb_slice.len();
    let mut i = 0;

    unsafe {
        let dz_dx_vec = _mm256_set1_ps(dz_dx);
        let du_fix_vec = _mm256_set1_epi32(du_fix);
        let dv_fix_vec = _mm256_set1_epi32(dv_fix);
        let dr_dx_vec = _mm256_set1_epi32(dr_dx);
        let dg_dx_vec = _mm256_set1_epi32(dg_dx);
        let db_dx_vec = _mm256_set1_epi32(db_dx);

        let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));

        let du_off = _mm256_mullo_epi32(du_fix_vec, offsets_i);
        let dv_off = _mm256_mullo_epi32(dv_fix_vec, offsets_i);
        let dr_off = _mm256_mullo_epi32(dr_dx_vec, offsets_i);
        let dg_off = _mm256_mullo_epi32(dg_dx_vec, offsets_i);
        let db_off = _mm256_mullo_epi32(db_dx_vec, offsets_i);

        let mut u_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(u_fix_start), du_off);
        let mut v_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(v_fix_start), dv_off);
        let mut r_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(r_start), dr_off);
        let mut g_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(g_start), dg_off);
        let mut b_fix_vec = _mm256_add_epi32(_mm256_set1_epi32(b_start), db_off);

        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let dr_step = _mm256_slli_epi32(dr_dx_vec, 3);
        let dg_step = _mm256_slli_epi32(dg_dx_vec, 3);
        let db_step = _mm256_slli_epi32(db_dx_vec, 3);
        let du_step = _mm256_slli_epi32(du_fix_vec, 3);
        let dv_step = _mm256_slli_epi32(dv_fix_vec, 3);

        let w_vec = _mm256_set1_epi32(texture.width as i32);
        let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
        let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
        let zero_i = _mm256_setzero_si256();
        let one_i = _mm256_set1_epi32(1);

        let shift_vec = _mm256_set1_epi32(i32::from(texture.width_shift));
        let is_pot = texture.width_shift < 32;

        let mask_ff = _mm256_set1_epi32(0xFF);
        let const_256 = _mm256_set1_epi32(256);
        let mask_255 = _mm256_set1_epi32(255);

        // Closure inside unsafe block needs to be safe or unsafe?
        // It uses intrinsics, so it must be unsafe block inside?
        // Actually, since we are inside `unsafe` block, we can call unsafe fns.
        let blend_swar_avx2 = |c0: __m256i, c1: __m256i, w: __m256i, inv_w: __m256i| -> __m256i {
            let mask = _mm256_set1_epi32(0x00FF_00FF);

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

        macro_rules! process_gouraud_bilinear_loop {
            ($is_pot_const:literal) => {
                while i + 8 <= len {
                    let depth_ptr = zb_slice.as_mut_ptr().add(i);
                    let depth_val = _mm256_loadu_ps(depth_ptr);
                    let mask_z = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

                    if _mm256_movemask_ps(mask_z) != 0 {
                        // Bilinear Texture Lookup
                        let u_img = _mm256_srai_epi32(u_fix_vec, 8);
                        let v_img = _mm256_srai_epi32(v_fix_vec, 8);

                        let wx = _mm256_and_si256(u_img, mask_ff);
                        let wy = _mm256_and_si256(v_img, mask_ff);

                        let inv_wx = _mm256_sub_epi32(const_256, wx);
                        let inv_wy = _mm256_sub_epi32(const_256, wy);

                        let x0_raw = _mm256_srai_epi32(u_img, 8);
                        let y0_raw = _mm256_srai_epi32(v_img, 8);

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

                        // Optimization: Use bitwise shifts for Power-of-Two textures
                        let (idx00, idx10, idx01, idx11) = if $is_pot_const {
                            let y0_shifted = _mm256_sllv_epi32(y0, shift_vec);
                            let y1_shifted = _mm256_sllv_epi32(y1, shift_vec);
                            (
                                _mm256_or_si256(y0_shifted, x0),
                                _mm256_or_si256(y0_shifted, x1),
                                _mm256_or_si256(y1_shifted, x0),
                                _mm256_or_si256(y1_shifted, x1),
                            )
                        } else {
                            let y0_w = _mm256_mullo_epi32(y0, w_vec);
                            let y1_w = _mm256_mullo_epi32(y1, w_vec);
                            (
                                _mm256_add_epi32(y0_w, x0),
                                _mm256_add_epi32(y0_w, x1),
                                _mm256_add_epi32(y1_w, x0),
                                _mm256_add_epi32(y1_w, x1),
                            )
                        };

                        let pixels_ptr = texture.pixels.as_ptr().cast::<i32>();
                        let c00 = _mm256_i32gather_epi32(pixels_ptr, idx00, 4);
                        let c10 = _mm256_i32gather_epi32(pixels_ptr, idx10, 4);
                        let c01 = _mm256_i32gather_epi32(pixels_ptr, idx01, 4);
                        let c11 = _mm256_i32gather_epi32(pixels_ptr, idx11, 4);

                        let top = blend_swar_avx2(c00, c10, wx, inv_wx);
                        let bot = blend_swar_avx2(c01, c11, wx, inv_wx);
                        let tex_color = blend_swar_avx2(top, bot, wy, inv_wy);

                        // Gouraud Modulation
                        let tex_r_i = _mm256_and_si256(_mm256_srli_epi32(tex_color, 16), mask_ff);
                        let tex_g_i = _mm256_and_si256(_mm256_srli_epi32(tex_color, 8), mask_ff);
                        let tex_b_i = _mm256_and_si256(tex_color, mask_ff);
                        let tex_a_i = _mm256_and_si256(_mm256_srli_epi32(tex_color, 24), mask_ff);

                        // Clamp shade to 0
                        let r_clamped = _mm256_max_epi32(r_fix_vec, zero_i);
                        let g_clamped = _mm256_max_epi32(g_fix_vec, zero_i);
                        let b_clamped = _mm256_max_epi32(b_fix_vec, zero_i);

                        // Multiply (tex * shade)
                        let mod_r = _mm256_mullo_epi32(tex_r_i, r_clamped);
                        let mod_g = _mm256_mullo_epi32(tex_g_i, g_clamped);
                        let mod_b = _mm256_mullo_epi32(tex_b_i, b_clamped);

                        // Shift >> 16 and clamp to 255
                        let out_r = _mm256_min_epi32(_mm256_srai_epi32(mod_r, 16), mask_255);
                        let out_g = _mm256_min_epi32(_mm256_srai_epi32(mod_g, 16), mask_255);
                        let out_b = _mm256_min_epi32(_mm256_srai_epi32(mod_b, 16), mask_255);

                        let out_color = _mm256_or_si256(
                            _mm256_slli_epi32(tex_a_i, 24),
                            _mm256_or_si256(
                                _mm256_slli_epi32(out_r, 16),
                                _mm256_or_si256(_mm256_slli_epi32(out_g, 8), out_b),
                            ),
                        );

                        // Write Output
                        let opaque_mask = _mm256_cmpeq_epi32(tex_a_i, mask_ff);
                        let zero_mask = _mm256_cmpeq_epi32(tex_a_i, zero_i);
                        let mask_z_int = _mm256_castps_si256(mask_z);

                        // Opaque
                        let write_opaque = _mm256_and_si256(mask_z_int, opaque_mask);
                        let write_opaque_ps = _mm256_castsi256_ps(write_opaque);

                        if _mm256_movemask_ps(write_opaque_ps) != 0 {
                            let old_z = _mm256_loadu_ps(depth_ptr);
                            let new_z = _mm256_blendv_ps(old_z, z_vec, write_opaque_ps);
                            _mm256_storeu_ps(depth_ptr, new_z);

                            let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                            let old_color = _mm256_loadu_si256(fb_ptr);
                            let new_color = _mm256_blendv_epi8(old_color, out_color, write_opaque);
                            _mm256_storeu_si256(fb_ptr, new_color);
                        }

                        // Translucent
                        let write_trans = _mm256_andnot_si256(
                            opaque_mask,
                            _mm256_andnot_si256(zero_mask, mask_z_int),
                        );
                        let trans_bits = _mm256_movemask_ps(_mm256_castsi256_ps(write_trans));

                        if trans_bits != 0 {
                            let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                            let current_dest = _mm256_loadu_si256(fb_ptr);

                            let alpha_src = tex_a_i;
                            let inv_alpha_src = _mm256_sub_epi32(const_256, alpha_src);

                            let blended =
                                blend_swar_simd(out_color, current_dest, inv_alpha_src, alpha_src);

                            let result = _mm256_blendv_epi8(current_dest, blended, write_trans);
                            _mm256_storeu_si256(fb_ptr, result);
                        }
                    }

                    z_vec = _mm256_add_ps(z_vec, dz_step);
                    r_fix_vec = _mm256_add_epi32(r_fix_vec, dr_step);
                    g_fix_vec = _mm256_add_epi32(g_fix_vec, dg_step);
                    b_fix_vec = _mm256_add_epi32(b_fix_vec, db_step);
                    u_fix_vec = _mm256_add_epi32(u_fix_vec, du_step);
                    v_fix_vec = _mm256_add_epi32(v_fix_vec, dv_step);
                    i += 8;
                }
            };
        }

        if is_pot {
            process_gouraud_bilinear_loop!(true);
        } else {
            process_gouraud_bilinear_loop!(false);
        }
    }

    // Scalar Tail
    let mut z_curr = z_start + (i as f32) * dz_dx;
    let mut u_curr = u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32));
    let mut v_curr = v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32));
    let mut r_curr = r_start.wrapping_add(dr_dx.wrapping_mul(i as i32));
    let mut g_curr = g_start.wrapping_add(dg_dx.wrapping_mul(i as i32));
    let mut b_curr = b_start.wrapping_add(db_dx.wrapping_mul(i as i32));

    while i < len {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(i);
            if z_curr < *depth_val {
                let color = texture.get_pixel_bilinear_fixed(u_curr, v_curr);

                let tex_r = ((color >> 16) & 0xFF) as i32;
                let tex_g = ((color >> 8) & 0xFF) as i32;
                let tex_b = (color & 0xFF) as i32;
                let tex_a = (color >> 24) & 0xFF;

                let r_val = r_curr.max(0);
                let g_val = g_curr.max(0);
                let b_val = b_curr.max(0);

                let final_r = ((tex_r * r_val) >> 16).clamp(0, 255) as u32;
                let final_g = ((tex_g * g_val) >> 16).clamp(0, 255) as u32;
                let final_b = ((tex_b * b_val) >> 16).clamp(0, 255) as u32;

                let final_color =
                    ((tex_a as u32) << 24) | (final_r << 16) | (final_g << 8) | final_b;

                if tex_a == 255 {
                    *depth_val = z_curr;
                    *fb_slice.get_unchecked_mut(i) = final_color;
                } else if tex_a > 0 {
                    let dest = *fb_slice.get_unchecked(i);
                    *fb_slice.get_unchecked_mut(i) = blend_swar(
                        final_color,
                        dest,
                        (255 - (tex_a as u8)).into(),
                        (tex_a as u8).into(),
                    );
                }
            }
        }
        z_curr += dz_dx;
        u_curr = u_curr.wrapping_add(du_fix);
        v_curr = v_curr.wrapping_add(dv_fix);
        r_curr = r_curr.wrapping_add(dr_dx);
        g_curr = g_curr.wrapping_add(dg_dx);
        b_curr = b_curr.wrapping_add(db_dx);
        i += 1;
    }
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
    mut r_fix: i32,
    mut g_fix: i32,
    mut b_fix: i32,
    dr_dx: i32,
    dg_dx: i32,
    db_dx: i32,
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
                    unsafe {
                        *tex_pixels.get_unchecked((v as usize) * (tex_w as usize) + (u as usize))
                    }
                }
            } else {
                texture.get_pixel_texel(u, v)
            };

            let tex_r = ((color >> 16) & 0xFF) as i32;
            let tex_g = ((color >> 8) & 0xFF) as i32;
            let tex_b = (color & 0xFF) as i32;
            let tex_a = (color >> 24) & 0xFF;

            // Use 16.16 shade values.
            // Shade is max 1.0 (65536). Tex is max 255.
            // tex * shade -> max ~1.67e7 (fits in i32).
            // Shift right 16 to get result in 0..255 range.
            let r_clamped = r_fix.max(0);
            let g_clamped = g_fix.max(0);
            let b_clamped = b_fix.max(0);

            let final_r = ((tex_r * r_clamped) >> 16).clamp(0, 255) as u32;
            let final_g = ((tex_g * g_clamped) >> 16).clamp(0, 255) as u32;
            let final_b = ((tex_b * b_clamped) >> 16).clamp(0, 255) as u32;

            let final_color = ((tex_a as u32) << 24) | (final_r << 16) | (final_g << 8) | final_b;

            if tex_a == 255 {
                *depth_val = z;
                *pixel = final_color;
            } else if tex_a > 0 {
                let dest = *pixel;
                *pixel = blend_swar(
                    final_color,
                    dest,
                    (255 - (tex_a as u8)).into(),
                    (tex_a as u8).into(),
                );
            }
        }
        z += dz_dx;
        u_fix = u_fix.wrapping_add(du_fix);
        v_fix = v_fix.wrapping_add(dv_fix);
        r_fix = r_fix.wrapping_add(dr_dx);
        g_fix = g_fix.wrapping_add(dg_dx);
        b_fix = b_fix.wrapping_add(db_dx);
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_textured_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: TexturedGouraudSpanStart,
    gradients: &TexturedGouraudGradients,
    texture: &Texture,
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

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

    // Fixed point 16.16 setup
    let scale = 65536.0;
    let dr_dx_i = (gradients.dr_dx * scale) as i32;
    let dg_dx_i = (gradients.dg_dx * scale) as i32;
    let db_dx_i = (gradients.db_dx * scale) as i32;

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

    // Current color in fixed point
    let mut r_fix = (r * scale) as i32;
    let mut g_fix = (g * scale) as i32;
    let mut b_fix = (b * scale) as i32;

    while x <= xe {
        let remaining = xe - x + 1;
        let count = remaining.min(span_size);

        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        let inv_count = RECIPROCAL_TABLE[count as usize];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (x as usize);
        let end_idx = y_offset + ((x + count - 1) as usize);

        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        match texture.filter_mode {
            FilterMode::Nearest => {
                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_textured_gouraud_simd(
                            fb_slice,
                            zb_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                            r_fix,
                            g_fix,
                            b_fix,
                            dr_dx_i,
                            dg_dx_i,
                            db_dx_i,
                        );
                    }
                } else {
                    draw_span_textured_gouraud_scalar(
                        fb_slice,
                        zb_slice,
                        texture,
                        z,
                        gradients.dz_dx,
                        u_fix,
                        v_fix,
                        du_fix,
                        dv_fix,
                        r_fix,
                        g_fix,
                        b_fix,
                        dr_dx_i,
                        dg_dx_i,
                        db_dx_i,
                    );
                }
                #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
                draw_span_textured_gouraud_scalar(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                    r_fix,
                    g_fix,
                    b_fix,
                    dr_dx_i,
                    dg_dx_i,
                    db_dx_i,
                );
            }
            FilterMode::Bilinear | FilterMode::Trilinear => {
                let u_fix = ((u_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let v_fix = ((v_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
                if is_x86_feature_detected!("avx2") {
                    unsafe {
                        draw_span_textured_gouraud_bilinear_simd(
                            fb_slice,
                            zb_slice,
                            texture,
                            z,
                            gradients.dz_dx,
                            u_fix,
                            v_fix,
                            du_fix,
                            dv_fix,
                            r_fix,
                            g_fix,
                            b_fix,
                            dr_dx_i,
                            dg_dx_i,
                            db_dx_i,
                        );
                    }
                } else {
                    let mut z_curr = z;
                    let mut r_curr = r_fix;
                    let mut g_curr = g_fix;
                    let mut b_curr = b_fix;

                    for i in 0..count {
                        let pixel = unsafe { fb_slice.get_unchecked_mut(i as usize) };
                        let depth_val = unsafe { zb_slice.get_unchecked_mut(i as usize) };

                        if z_curr < *depth_val {
                            let q_curr = q + (i as f32) * gradients.dq_dx;
                            let u_curr_p = u + (i as f32) * gradients.du_dx;
                            let v_curr_p = v + (i as f32) * gradients.dv_dx;

                            let w_recip = if q_curr.abs() > 0.000_001 {
                                1.0 / q_curr
                            } else {
                                1.0
                            };
                            let u_tex = u_curr_p * w_recip;
                            let v_tex = v_curr_p * w_recip;

                            let color = texture.get_pixel_bilinear_texel(u_tex, v_tex);

                            let tex_r = ((color >> 16) & 0xFF) as i32;
                            let tex_g = ((color >> 8) & 0xFF) as i32;
                            let tex_b = (color & 0xFF) as i32;
                            let tex_a = (color >> 24) & 0xFF;

                            // 16.16 fixed modulation
                            let r_val = r_curr.max(0);
                            let g_val = g_curr.max(0);
                            let b_val = b_curr.max(0);

                            let final_r = ((tex_r * r_val) >> 16).clamp(0, 255) as u32;
                            let final_g = ((tex_g * g_val) >> 16).clamp(0, 255) as u32;
                            let final_b = ((tex_b * b_val) >> 16).clamp(0, 255) as u32;

                            let final_color =
                                ((tex_a as u32) << 24) | (final_r << 16) | (final_g << 8) | final_b;

                            if tex_a == 255 {
                                *depth_val = z_curr;
                                *pixel = final_color;
                            } else if tex_a > 0 {
                                let dest = *pixel;
                                *pixel = blend_swar(
                                    final_color,
                                    dest,
                                    (255 - (tex_a as u8)).into(),
                                    (tex_a as u8).into(),
                                );
                            }
                        }
                        z_curr += gradients.dz_dx;
                        r_curr += dr_dx_i;
                        g_curr += dg_dx_i;
                        b_curr += db_dx_i;
                    }
                }
            }
        }

        z += gradients.dz_dx * count as f32;
        q = q_end;
        u = u_end;
        v = v_end;

        // Accumulate fixed point color
        r_fix = r_fix.wrapping_add(dr_dx_i.wrapping_mul(count));
        g_fix = g_fix.wrapping_add(dg_dx_i.wrapping_mul(count));
        b_fix = b_fix.wrapping_add(db_dx_i.wrapping_mul(count));

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
#[inline(always)]
pub fn fill_triangle_textured_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec2),
    v1: ((Vec3, f32), Vec3, Vec2),
    v2: ((Vec3, f32), Vec3, Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(
        v0,
        v1,
        v2,
        |v| v.0,
        |a, b, t| {
            (
                (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                a.1.lerp(b.1, t),
                a.2.lerp(b.2, t),
            )
        },
    );

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped[base];
        let v1 = clipped[base + 1];
        let v2 = clipped[base + 2];

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

        fill_projected_triangle_textured_gouraud(
            fb, zb, p0, p1, p2, u0, v0, u1, v1, u2, v2, c0, c1, c2, texture,
        );
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn fill_projected_triangle_textured_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    p0: ScreenPoint,
    p1: ScreenPoint,
    p2: ScreenPoint,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    u2: f32,
    v2: f32,
    c0: Vec3,
    c1: Vec3,
    c2: Vec3,
    texture: &Texture,
) {
    if is_backface(p0, p1, p2) {
        return;
    }

    let q0 = p0.inv_w;
    let q1 = p1.inv_w;
    let q2 = p2.inv_w;

    let (gradients, _) =
        TexturedGouraudGradients::new(p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2, c0, c1, c2);

    fill_projected_triangle_textured_gouraud_with_gradients(
        fb, zb, p0, p1, p2, u0, v0, u1, v1, u2, v2, c0, c1, c2, texture, &gradients,
    );
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn fill_projected_triangle_textured_gouraud_with_gradients(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    p0: ScreenPoint,
    p1: ScreenPoint,
    p2: ScreenPoint,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    u2: f32,
    v2: f32,
    c0: Vec3,
    c1: Vec3,
    c2: Vec3,
    texture: &Texture,
    gradients: &TexturedGouraudGradients,
) {
    let height = fb.height();

    let mut verts = [(p0, u0, v0, c0), (p1, u1, v1, c1), (p2, u2, v2, c2)];
    sort_by_y(&mut verts, |(p, ..)| p.y);
    let [(p0, u0, v0, c0), (p1, u1, v1, c1), (p2, u2, v2, c2)] = verts;

    let q0 = p0.inv_w;
    let q1 = p1.inv_w;
    let q2 = p2.inv_w;

    let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    if total_height == 0.0 {
        return;
    }

    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = p0.y.max(y_min);
    let y_end = p2.y.min(y_max);

    if y_start > y_end {
        return;
    }

    let nz = calculate_signed_area_doubled(p0, p1, p2);
    let long_edge_is_left = nz > 0.0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::{Vec2, Vec3};
    use crate::texture::Texture;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn test_draw_scanline_nearest() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();
        let mut tex = Texture::new(2, 2).unwrap();
        // (0,0)=Red, (1,0)=Green, (0,1)=Blue, (1,1)=White
        tex.set_pixel(0, 0, 0xFFFF0000);
        tex.set_pixel(1, 0, 0xFF00FF00);
        tex.set_pixel(0, 1, 0xFF0000FF);
        tex.set_pixel(1, 1, 0xFFFFFFFF);

        // Simple gradient: z=1, q=1 (w=1), u=0..1, v=0
        let start = PerspectiveSpanStart {
            z: 1.0,
            q: 1.0,
            u: 0.0,
            v: 0.0,
        };
        // u goes from 0.0 to 1.0 over 2 pixels (width of texture)
        // u is in range [0, 1]. To span the texture width (2 texels), u goes 0->1.
        // We want x=0 -> u=0 (Texel 0), x=1 -> u=0.5 (Texel 1)
        // du/dx = 0.5.
        let gradients = PerspectiveTextureGradients {
            dz_dx: 0.0,
            dq_dx: 0.0,
            du_dx: 0.5,
            dv_dx: 0.0,
            dq_dy: 0.0,
            du_dy: 0.0,
            dv_dy: 0.0,
        };

        draw_scanline_textured_perspective(
            &mut fb, &mut zb, &tex, 5, // y
            0, // x_start
            3, // x_end
            start, &gradients,
        );

        // x=0: u=0.0 -> Texel 0 -> Red
        // x=1: u=0.5 -> Texel 0 (floor(0.5)=0) -> Red
        // x=2: u=1.0 -> Texel 1 (floor(1.0)=1) -> Green

        assert_eq!(
            fb.get_pixel(0, 5).unwrap(),
            0xFFFF0000,
            "Pixel 0 should be Red"
        );
        assert_eq!(
            fb.get_pixel(1, 5).unwrap(),
            0xFFFF0000,
            "Pixel 1 should be Red"
        );
        assert_eq!(
            fb.get_pixel(2, 5).unwrap(),
            0xFF00FF00,
            "Pixel 2 should be Green"
        );
    }

    #[test]
    fn test_draw_scanline_bilinear() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();
        let mut tex = Texture::new(2, 2).unwrap();
        tex.filter_mode = crate::texture::FilterMode::Bilinear;

        // 0,0: Black (0x00000000)
        // 1,0: White (0xFFFFFFFF)
        tex.set_pixel(0, 0, 0xFF000000);
        tex.set_pixel(1, 0, 0xFFFFFFFF);

        let start = PerspectiveSpanStart {
            z: 1.0,
            q: 1.0,
            u: 0.25, // Half-way into the first pixel (center is 0.25 in 0..1 space for 2 pixels)
            // Wait. Texel centers are at index + 0.5.
            // Width 2.
            // Texel 0 center: 0.5 (in 0..2 space). Normalized: 0.25.
            // Texel 1 center: 1.5. Normalized: 0.75.
            // If we sample at u=0.5 (Normalized 0.25), we get exact Pixel 0?
            // Bilinear logic: u_fix - 0.5.
            // u=0.25 -> 0.5 texels.
            // 0.5 - 0.5 = 0.0. Index 0.
            // So u=0.25 should be exactly Black.

            // We want to blend. Sample at u=0.5 (Normalized 0.25) + 0.25 = 0.5 (Texel 1.0)
            // Normalized u = 0.5.
            // Texel coord = 1.0.
            // Offset -0.5 = 0.5.
            // x0 = floor(0.5) = 0.
            // frac = 0.5.
            // Blend 50% Pixel 0, 50% Pixel 1.
            v: 0.0,
        };

        let gradients = PerspectiveTextureGradients {
            dz_dx: 0.0,
            dq_dx: 0.0,
            du_dx: 0.0,
            dv_dx: 0.0,
            dq_dy: 0.0,
            du_dy: 0.0,
            dv_dy: 0.0,
        };

        // Override start.u for the test
        let mut start_blend = start;
        start_blend.u = 1.0; // Normalized 1.0 -> Texel 2.0. Offset -0.5 -> 1.5. x0=1. Blend 1 and 2?
        // Wait. u_fix = u * width * 65536.
        // u=1.0 (texel coords). Width already applied?
        // In this test setup, I manually pass `start`.
        // `draw_scanline` calculates `u_tex_start = u * w_start`.
        // If I pass u=1.0. `u_tex_start` = 1.0.
        // `u_fix` = 65536.
        // Bilinear sub 32768 -> 32768 (0.5).
        // x0_raw = 0. wx = 128 (0.5).
        // Blend Pixel 0 and Pixel 1.
        // Pixel 0: Black. Pixel 1: White.
        // Result: Grey.
        start_blend.u = 1.0;

        draw_scanline_textured_perspective(
            &mut fb,
            &mut zb,
            &tex,
            5,
            0,
            0,
            start_blend,
            &gradients,
        );

        let pixel = fb.get_pixel(0, 5).unwrap();
        let r = (pixel >> 16) & 0xFF;

        // (0 + 255) / 2 = 127.
        assert!((120..=135).contains(&r), "Pixel should be ~127, got {r}");
    }

    #[test]
    fn test_fill_triangle_textured_clipped() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();
        let tex = Texture::new(2, 2).unwrap();

        // Triangle mostly outside
        let v0 = ((Vec3::new(-100.0, 0.0, 5.0), 5.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(100.0, -100.0, 5.0), 5.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(100.0, 100.0, 5.0), 5.0), Vec2::new(0.0, 1.0));

        // Should clip and run without panic
        fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &tex);
    }

    #[test]
    #[ignore]
    fn test_fill_quad_textured_optimization() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();
        let mut tex = Texture::new(2, 2).unwrap();
        // Fill white to avoid sampling issues
        for y in 0..2 {
            for x in 0..2 {
                tex.set_pixel(x, y, 0xFFFFFFFF);
            }
        }

        // Quad fully inside frustum (-w..w)
        let v0 = ((Vec3::new(-0.5, -0.5, 0.0), 1.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(0.5, -0.5, 0.0), 1.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(0.5, 0.5, 0.0), 1.0), Vec2::new(1.0, 1.0));
        let v3 = ((Vec3::new(-0.5, 0.5, 0.0), 1.0), Vec2::new(0.0, 1.0));

        fill_quad_textured(&mut fb, &mut zb, v0, v1, v2, v3, &tex);

        // Should produce pixels.
        // We didn't set view/proj matrices, so it projects directly.
        // -0.5 -> screen coords.
        // width=10. half=5.
        // x = (-0.5 + 1) * 5 = 2.5 -> 2.
        // y = (1 - (-0.5)) * 5 = 7.5 -> 7.
        // It covers roughly 2..7 in x and y.

        // Check pixel (5, 5)
        let p = fb.get_pixel(5, 5).unwrap();
        assert_eq!(p, 0xFFFFFFFF, "Center pixel should be set");
    }
}

#[test]
fn test_draw_scanline_trilinear() {
    // 1. Setup Buffer
    let mut fb = Framebuffer::new(10, 10).unwrap();
    let mut zb = ZBuffer::new(10, 10).unwrap();

    // 2. Setup Texture (2x2 Checkered)
    let mut tex = Texture::new(2, 2).unwrap();
    // Level 0:
    // B W
    // W B
    tex.set_pixel(0, 0, 0xFF000000); // Black
    tex.set_pixel(1, 0, 0xFFFFFFFF); // White
    tex.set_pixel(0, 1, 0xFFFFFFFF); // White
    tex.set_pixel(1, 1, 0xFF000000); // Black

    tex.generate_mipmaps();
    // Level 1 (1x1) should be Grey (approx 127/128)
    // 0x7F7F7F...

    tex.filter_mode = crate::texture::FilterMode::Trilinear;

    // 3. Setup Span
    // We want to sample at (0.5, 0.5) in texel space (Center of Top-Left pixel).
    // Level 0 value at (0.5, 0.5) is Black.
    // Level 1 value at (0.5, 0.5) is Grey.

    // We control LOD via gradients.
    // LOD = 0.5 * log2(rho^2).
    // We want LOD = 0.5 -> rho = sqrt(2) approx 1.41421356.
    // du_dx (texels) = 1.41421356.

    let start = PerspectiveSpanStart {
        z: 1.0,
        q: 1.0, // w=1
        u: 0.5, // u_tex = 0.5
        v: 0.5, // v_tex = 0.5
    };

    let gradients = PerspectiveTextureGradients {
        dz_dx: 0.0,
        dq_dx: 0.0,
        du_dx: std::f32::consts::SQRT_2, // This is du_tex/dx because q=1, u=0.5 (small)
        dv_dx: 0.0,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
    };

    draw_scanline_textured_perspective(
        &mut fb, &mut zb, &tex, 5, // y
        0, // x_start
        0, // x_end (single pixel)
        start, &gradients,
    );

    let pixel = fb.get_pixel(0, 5).unwrap();
    let r = (pixel >> 16) & 0xFF;

    // Level 0 (Black) mixed with Level 1 (Grey ~127).
    // 50/50 blend -> ~63.
    // Allow range 55-75.
    assert!((55..=75).contains(&r), "Expected ~64, got {r}");
}

#[test]
fn test_draw_span_nearest_overflow_vulnerability() {
    let mut fb = vec![0u32; 16];
    let mut zb = vec![100.0f32; 16];
    let mut tex = Texture::new(2, 2).unwrap();
    for y in 0..2 {
        for x in 0..2 {
            tex.set_pixel(x, y, 0xFFFFFFFF);
        }
    }

    let z = 1.0;
    let dz_dx = 0.0;

    let u_fix = 10;
    let v_fix = 0;
    let du_fix = 286331154; // Causes wrap around on 15 iterations: 286331154 * 15 % 2^32 = 14
    let dv_fix = 0;

    // Using `std::panic::catch_unwind` and `AssertUnwindSafe` to ensure intentional panic testing
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        draw_span_nearest(
            &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
        );
    }));

    assert!(
        result.is_ok(),
        "draw_span_nearest panicked due to overflow vulnerability!"
    );
}

#[test]
fn test_reciprocal_table_accuracy() {
    for (i, &table_val) in RECIPROCAL_TABLE.iter().enumerate().skip(1) {
        let actual = 1.0 / (i as f32);
        let diff = (table_val - actual).abs();

        // Precision should be very high (f32 epsilon level)
        assert!(
            diff < 1e-6,
            "Table index {i} mismatch: table={table_val}, actual={actual}"
        );
    }
}
