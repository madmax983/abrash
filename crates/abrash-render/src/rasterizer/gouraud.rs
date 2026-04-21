//! Gouraud shading rasterizer.
//!
//! Per-vertex lighting calculation and color interpolation across the triangle.

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3, project_triangle_to_screen};
use crate::zbuffer::ZBuffer;

use super::core::{
    FIXED_SCALE, assert_same_dimensions, is_backface, pack_color_fixed_i32, sort_by_y,
};

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn draw_scanline_gouraud_simd_fast(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z_start: f32,
    c_start: (i32, i32, i32),
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    use std::arch::x86_64::{
        __m256i, _CMP_LT_OQ, _mm256_add_epi32, _mm256_add_ps, _mm256_and_si256, _mm256_blendv_epi8,
        _mm256_blendv_ps, _mm256_castps_si256, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_loadu_si256,
        _mm256_movemask_ps, _mm256_mul_ps, _mm256_mullo_epi32, _mm256_or_si256, _mm256_set_epi32,
        _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps, _mm256_slli_epi32, _mm256_srli_epi32,
        _mm256_storeu_ps, _mm256_storeu_si256,
    };

    let len = fb_slice.len();
    let mut i = 0;

    // Load constants
    let dz_dx_vec = _mm256_set1_ps(dz_dx);
    let dr_dx_vec = _mm256_set1_epi32(dc_dx.0);
    let dg_dx_vec = _mm256_set1_epi32(dc_dx.1);
    let db_dx_vec = _mm256_set1_epi32(dc_dx.2);

    let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
    let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));

    // Initialize colors: Start + (dc * i)
    let dr_off = _mm256_mullo_epi32(dr_dx_vec, offsets_i);
    let dg_off = _mm256_mullo_epi32(dg_dx_vec, offsets_i);
    let db_off = _mm256_mullo_epi32(db_dx_vec, offsets_i);

    let mut r_vec = _mm256_add_epi32(_mm256_set1_epi32(c_start.0), dr_off);
    let mut g_vec = _mm256_add_epi32(_mm256_set1_epi32(c_start.1), dg_off);
    let mut b_vec = _mm256_add_epi32(_mm256_set1_epi32(c_start.2), db_off);

    // Steps for 8 pixels
    let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
    // dc * 8
    let dr_step = _mm256_slli_epi32(dr_dx_vec, 3);
    let dg_step = _mm256_slli_epi32(dg_dx_vec, 3);
    let db_step = _mm256_slli_epi32(db_dx_vec, 3);

    // Masks
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
    let mask_r = _mm256_set1_epi32(0x00FF_0000);

    while i + 8 <= len {
        // SAFETY: Loop bounds checked (i + 8 <= len). `depth_ptr` and `fb_ptr` are valid.
        // `_mm256_loadu_ps` and `_mm256_storeu_ps` handle potentially unaligned access.
        unsafe {
            // Load Z
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Compare Z
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_int = _mm256_castps_si256(mask);

            if _mm256_movemask_ps(mask) != 0 {
                // Update Z
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Pack Colors: Hybrid Optimization
                // Goal: Reduce instructions AND register pressure.
                // R: mask_r (1 op, 1 const).
                // G: shift, shift (2 ops, 0 const).
                // B: shift (1 op, 0 const).
                // Total: 4 ops, 1 const register (+ alpha). Same register count as baseline, fewer ops.

                let r_packed = _mm256_and_si256(r_vec, mask_r);
                let g_packed = _mm256_slli_epi32(_mm256_srli_epi32(g_vec, 16), 8);
                let b_packed = _mm256_srli_epi32(b_vec, 16);

                // Pack: alpha | r | g | b
                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(r_packed, _mm256_or_si256(g_packed, b_packed)),
                );

                // Store with mask
                #[allow(clippy::cast_ptr_alignment)]
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }
        }

        // Advance
        z_vec = _mm256_add_ps(z_vec, dz_step);
        r_vec = _mm256_add_epi32(r_vec, dr_step);
        g_vec = _mm256_add_epi32(g_vec, dg_step);
        b_vec = _mm256_add_epi32(b_vec, db_step);

        i += 8;
    }

    // Scalar tail
    let mut z = z_start + (i as f32) * dz_dx;
    let mut r_i = c_start.0.wrapping_add(dc_dx.0.wrapping_mul(i as i32));
    let mut g_i = c_start.1.wrapping_add(dc_dx.1.wrapping_mul(i as i32));
    let mut b_i = c_start.2.wrapping_add(dc_dx.2.wrapping_mul(i as i32));

    let dr = dc_dx.0;
    let dg = dc_dx.1;
    let db = dc_dx.2;

    for (depth_val, pixel) in zb_slice[i..len].iter_mut().zip(fb_slice[i..len].iter_mut()) {
        if z < *depth_val {
            *depth_val = z;
            // Fast path: direct shift, no clamp/mask (assuming valid input range)
            let r = (r_i as u32) >> 16;
            let g = (g_i as u32) >> 16;
            let b = (b_i as u32) >> 16;
            *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
        z += dz_dx;
        r_i = r_i.wrapping_add(dr);
        g_i = g_i.wrapping_add(dg);
        b_i = b_i.wrapping_add(db);
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn draw_scanline_gouraud_simd_clamped(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z_start: f32,
    c_start: (i32, i32, i32),
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    use std::arch::x86_64::{
        __m256i, _CMP_LT_OQ, _mm256_add_epi32, _mm256_add_ps, _mm256_and_si256, _mm256_blendv_epi8,
        _mm256_blendv_ps, _mm256_castps_si256, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_loadu_si256,
        _mm256_max_epi32, _mm256_min_epi32, _mm256_movemask_ps, _mm256_mul_ps, _mm256_mullo_epi32,
        _mm256_or_si256, _mm256_set_epi32, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps,
        _mm256_setzero_si256, _mm256_slli_epi32, _mm256_srli_epi32, _mm256_storeu_ps,
        _mm256_storeu_si256,
    };

    let len = fb_slice.len();
    let mut i = 0;

    // Load constants
    let dz_dx_vec = _mm256_set1_ps(dz_dx);
    let dr_dx_vec = _mm256_set1_epi32(dc_dx.0);
    let dg_dx_vec = _mm256_set1_epi32(dc_dx.1);
    let db_dx_vec = _mm256_set1_epi32(dc_dx.2);

    let offsets_f = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
    let offsets_i = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);

    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets_f));

    // Initialize colors: Start + (dc * i)
    let dr_off = _mm256_mullo_epi32(dr_dx_vec, offsets_i);
    let dg_off = _mm256_mullo_epi32(dg_dx_vec, offsets_i);
    let db_off = _mm256_mullo_epi32(db_dx_vec, offsets_i);

    let mut r_vec = _mm256_add_epi32(_mm256_set1_epi32(c_start.0), dr_off);
    let mut g_vec = _mm256_add_epi32(_mm256_set1_epi32(c_start.1), dg_off);
    let mut b_vec = _mm256_add_epi32(_mm256_set1_epi32(c_start.2), db_off);

    // Steps for 8 pixels
    let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
    // dc * 8
    let dr_step = _mm256_slli_epi32(dr_dx_vec, 3);
    let dg_step = _mm256_slli_epi32(dg_dx_vec, 3);
    let db_step = _mm256_slli_epi32(db_dx_vec, 3);

    // Masks
    // 0x00FF0000 is 255.0 in 16.16 fixed point
    let min_val = _mm256_setzero_si256();
    let max_val = _mm256_set1_epi32(0x00FF_0000);
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

    while i + 8 <= len {
        // SAFETY: Loop bounds checked (i + 8 <= len). `depth_ptr` and `fb_ptr` are valid.
        // `_mm256_loadu_ps` and `_mm256_storeu_ps` handle potentially unaligned access.
        unsafe {
            // Load Z
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Compare Z
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_int = _mm256_castps_si256(mask);

            if _mm256_movemask_ps(mask) != 0 {
                // Update Z
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Clamp: min(max(val, 0), 255.0)
                let r_clamped = _mm256_min_epi32(_mm256_max_epi32(r_vec, min_val), max_val);
                let g_clamped = _mm256_min_epi32(_mm256_max_epi32(g_vec, min_val), max_val);
                let b_clamped = _mm256_min_epi32(_mm256_max_epi32(b_vec, min_val), max_val);

                // Pack Colors: Optimized bitwise operations
                // r_clamped is already in 0x00RRxxxx (max 0x00FF0000).
                // Just mask off lower bits to get 0x00RR0000.
                let r_packed = _mm256_and_si256(r_clamped, max_val);

                // Pack G/B using shifts
                let g_val = _mm256_srli_epi32(g_clamped, 16);
                let b_val = _mm256_srli_epi32(b_clamped, 16);

                // Pack: alpha | r | (g << 8) | b
                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        r_packed,
                        _mm256_or_si256(_mm256_slli_epi32(g_val, 8), b_val),
                    ),
                );

                // Store with mask
                #[allow(clippy::cast_ptr_alignment)]
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }
        }

        // Advance
        z_vec = _mm256_add_ps(z_vec, dz_step);
        r_vec = _mm256_add_epi32(r_vec, dr_step);
        g_vec = _mm256_add_epi32(g_vec, dg_step);
        b_vec = _mm256_add_epi32(b_vec, db_step);

        i += 8;
    }

    // Scalar tail
    let mut z = z_start + (i as f32) * dz_dx;
    let mut r_i = c_start.0.wrapping_add(dc_dx.0.wrapping_mul(i as i32));
    let mut g_i = c_start.1.wrapping_add(dc_dx.1.wrapping_mul(i as i32));
    let mut b_i = c_start.2.wrapping_add(dc_dx.2.wrapping_mul(i as i32));

    let dr = dc_dx.0;
    let dg = dc_dx.1;
    let db = dc_dx.2;

    for (depth_val, pixel) in zb_slice[i..len].iter_mut().zip(fb_slice[i..len].iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Clamped path
            let r = r_i.max(0).min(0x00FF_0000);
            let g = g_i.max(0).min(0x00FF_0000);
            let b = b_i.max(0).min(0x00FF_0000);

            *pixel = 0xFF00_0000
                | ((r as u32) & 0x00FF_0000)
                | (((g as u32) & 0x00FF_0000) >> 8)
                | (((b as u32) & 0x00FF_0000) >> 16);
        }
        z += dz_dx;
        r_i = r_i.wrapping_add(dr);
        g_i = g_i.wrapping_add(dg);
        b_i = b_i.wrapping_add(db);
    }
}

/// Draw a single scanline for Gouraud shading (Wrapper for API compatibility)
///
/// # Arguments
///
/// * `fb` - Target framebuffer.
/// * `zb` - Target Z-buffer.
/// * `y` - The Y coordinate of the scanline.
/// * `x_start` - The starting X coordinate.
/// * `x_end` - The ending X coordinate (exclusive).
/// * `z_start` - The initial Z-depth at `x_start`.
/// * `c_start` - The initial color at `x_start` in 16.16 fixed-point format (R, G, B).
/// * `dz_dx` - The change in Z-depth per pixel.
/// * `dc_dx` - The change in color (R, G, B) per pixel in 16.16 fixed-point format.
///
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    c_start: (i64, i64, i64), // Fixed point color
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    draw_scanline_gouraud_i32(
        fb,
        zb,
        y,
        x_start,
        x_end,
        z_start,
        (c_start.0 as i32, c_start.1 as i32, c_start.2 as i32),
        dz_dx,
        dc_dx,
    );
}

/// Draw a single scanline for Gouraud shading (Optimized i32 version)
///
/// # Panics
///
/// Panics if the internal Framebuffer and `ZBuffer` dimensions or slice sizes do not match.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_gouraud_i32(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    c_start: (i32, i32, i32), // Fixed point color
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = z_start;

    // Use i64 for accumulators to prevent overflow when x_start is far off-screen
    let mut r_i = i64::from(c_start.0);
    let mut g_i = i64::from(c_start.1);
    let mut b_i = i64::from(c_start.2);
    let (dr, dg, db) = (i64::from(dc_dx.0), i64::from(dc_dx.1), i64::from(dc_dx.2));

    // Clamp to screen bounds
    if xs < 0 {
        let diff = -i64::from(xs);
        z += (diff as f32) * dz_dx;
        let diff_i64 = diff;
        r_i += diff_i64 * dr;
        g_i += diff_i64 * dg;
        b_i += diff_i64 * db;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    // Optimization: Demote to i32 for the hot loop to reduce register pressure.
    // We used i64 above to handle large off-screen jumps safely without overflow.
    // Once on-screen, 16.16 fixed point color fits comfortably in i32.
    // (Max value ~255 * 65536 = 1.6e7 << i32::MAX)
    let mut r_i = r_i as i32;
    let mut g_i = g_i as i32;
    let mut b_i = b_i as i32;
    let dr = dr as i32;
    let dg = dg as i32;
    let db = db as i32;

    if xs <= xe {
        // Optimization: Use slice iterators to avoid index recalculation and bounds checks in the loop
        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (xs as usize);
        let end_idx = y_offset + (xe as usize);

        // SAFETY:
        // 1. xs and xe are clamped to [0, width-1] by the logic above.
        // 2. y is clamped to [0, height-1] by the caller (fill_triangle_gouraud).
        // 3. We checked `xs <= xe` immediately above, so `start_idx <= end_idx`.
        // Therefore, the range is valid and within bounds.
        let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];
        let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];

        // Optimization: Check for fast path (no clamping needed)
        // If all color channels are within [0, 255] for the entire span, we can skip clamping.
        // r_i is 16.16 fixed point. Max value is 255.0 = 0x00FF_0000.
        // We calculate end values based on start + delta * count.
        let count = xe - xs;
        let r_end = r_i.wrapping_add(dr.wrapping_mul(count));
        let g_end = g_i.wrapping_add(dg.wrapping_mul(count));
        let b_end = b_i.wrapping_add(db.wrapping_mul(count));

        // Use strict upper bound 0x0100_0000 (256.0) to ensure integer part fits in u8.
        // Cast to u32 handles negative check (becomes large u32).
        let safe_limit: u32 = 0x0100_0000;
        let safe = (r_i as u32) < safe_limit
            && (r_end as u32) < safe_limit
            && (g_i as u32) < safe_limit
            && (g_end as u32) < safe_limit
            && (b_i as u32) < safe_limit
            && (b_end as u32) < safe_limit;

        if safe {
            #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
            if fb_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
                unsafe {
                    draw_scanline_gouraud_simd_fast(
                        fb_slice,
                        zb_slice,
                        z,
                        (r_i, g_i, b_i),
                        dz_dx,
                        (dr, dg, db),
                    );
                }
                return;
            }

            let len = fb_slice.len();
            assert!(
                zb_slice.len() >= len,
                "Depth buffer must be at least as large as the framebuffer slice"
            );
            let mut fb_ptr = fb_slice.as_mut_ptr();
            let mut zb_ptr = zb_slice.as_mut_ptr();
            for _ in 0..len {
                unsafe {
                    if z < *zb_ptr {
                        *zb_ptr = z;
                        let r = (r_i as u32) >> 16;
                        let g = (g_i as u32) >> 16;
                        let b = (b_i as u32) >> 16;
                        *fb_ptr = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                    }
                    fb_ptr = fb_ptr.add(1);
                    zb_ptr = zb_ptr.add(1);
                }
                z += dz_dx;
                r_i += dr;
                g_i += dg;
                b_i += db;
            }
        } else {
            #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
            if fb_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
                unsafe {
                    draw_scanline_gouraud_simd_clamped(
                        fb_slice,
                        zb_slice,
                        z,
                        (r_i, g_i, b_i),
                        dz_dx,
                        (dr, dg, db),
                    );
                }
                return;
            }

            let len = fb_slice.len();
            assert!(
                zb_slice.len() >= len,
                "Depth buffer must be at least as large as the framebuffer slice"
            );
            let mut fb_ptr = fb_slice.as_mut_ptr();
            let mut zb_ptr = zb_slice.as_mut_ptr();
            for _ in 0..len {
                unsafe {
                    if z < *zb_ptr {
                        *zb_ptr = z;
                        let r = r_i.max(0).min(0x00FF_0000);
                        let g = g_i.max(0).min(0x00FF_0000);
                        let b = b_i.max(0).min(0x00FF_0000);

                        *fb_ptr = 0xFF00_0000
                            | ((r as u32) & 0x00FF_0000)
                            | (((g as u32) & 0x00FF_0000) >> 8)
                            | (((b as u32) & 0x00FF_0000) >> 16);
                    }
                    fb_ptr = fb_ptr.add(1);
                    zb_ptr = zb_ptr.add(1);
                }
                z += dz_dx;
                r_i += dr;
                g_i += dg;
                b_i += db;
            }
        }
    }
}

#[derive(Clone, Copy)]
/// Gradients used for interpolating values across a Gouraud shaded triangle.
pub struct GouraudGradients {
    pub(crate) dz_dx: f32,
    pub(crate) dc_dx: (i32, i32, i32),
}

impl GouraudGradients {
    pub(crate) fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        c0: Vec3,
        c1: Vec3,
        c2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vc = c2 - c0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_r = uy * vc.x - uc.x * vy;
        let nx_g = uy * vc.y - uc.y * vy;
        let nx_b = uy * vc.z - uc.z * vy;

        let dr = nx_r * inv_nz;
        let dg = nx_g * inv_nz;
        let db = nx_b * inv_nz;

        let dr_i = (dr * FIXED_SCALE) as i32;
        let dg_i = (dg * FIXED_SCALE) as i32;
        let db_i = (db * FIXED_SCALE) as i32;

        (
            Self {
                dz_dx,
                dc_dx: (dr_i, dg_i, db_i),
            },
            nz > 0.0,
        )
    }
}

pub(crate) struct GouraudEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) c: (i32, i32, i32),
    dx_dy: i64,
    dz_dy: f32,
    dc_dy: (i32, i32, i32),
}

impl GouraudEdgeWalker {
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        c_start: Vec3,
        c_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height == 0.0 {
            (0, 0.0, (0, 0, 0))
        } else {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i32,
                    (dc.y * FIXED_SCALE) as i32,
                    (dc.z * FIXED_SCALE) as i32,
                ),
            )
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i32,
            (c_start.y * FIXED_SCALE) as i32,
            (c_start.z * FIXED_SCALE) as i32,
        );

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            c: c_fixed,
            dx_dy,
            dz_dy,
            dc_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.c.0 += self.dc_dy.0;
        self.c.1 += self.dc_dy.1;
        self.c.2 += self.dc_dy.2;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        let n_i32 = n as i32;
        self.c.0 = self.c.0.wrapping_add(self.dc_dy.0.wrapping_mul(n_i32));
        self.c.1 = self.c.1.wrapping_add(self.dc_dy.1.wrapping_mul(n_i32));
        self.c.2 = self.c.2.wrapping_add(self.dc_dy.2.wrapping_mul(n_i32));
    }
}

/// Fill a 3D triangle with Gouraud (per-vertex) shading.
///
/// This function linearly interpolates colors across the face of the triangle.
/// Lighting calculations are performed at vertices, and the resulting colors
/// are passed to this function.
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Color)`.
///     *   `Position`: Clip Space position.
///     *   `W`: Homogeneous W coordinate.
///     *   `Color`: RGB color (0.0 - 1.0) for the vertex.
///
/// # Examples
///
/// ```
/// use abrash_render::rasterizer::fill_triangle_gouraud;
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::zbuffer::ZBuffer;
/// use abrash_core::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Each vertex is: ((Position, w), Color)
/// // Position is Clip Space Vec3.
/// // Color is linear RGB Vec3 (0.0 - 1.0).
///
/// // Top Vertex (Red)
/// let v0 = (
///     (Vec3::new(0.0, 5.0, 5.0), 5.0),
///     Vec3::new(1.0, 0.0, 0.0)
/// );
/// // Bottom Left (Green)
/// let v1 = (
///     (Vec3::new(-5.0, -5.0, 5.0), 5.0),
///     Vec3::new(0.0, 1.0, 0.0)
/// );
/// // Bottom Right (Blue)
/// let v2 = (
///     (Vec3::new(5.0, -5.0, 5.0), 5.0),
///     Vec3::new(0.0, 0.0, 1.0)
/// );
///
/// fill_triangle_gouraud(&mut fb, &mut zb, v0, v1, v2);
/// ```
/// Fill a 3D triangle with Gouraud Shading.
///
/// Gouraud shading calculates lighting at the vertices and interpolates the resulting
/// colors across the triangle surface. This is faster than Phong shading but can result
/// in less accurate highlights, especially for large polygons.
///
/// # Arguments
///
/// * `fb` - Target framebuffer.
/// * `zb` - Target Z-buffer.
/// * `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Color)`.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::zbuffer::ZBuffer;
/// use abrash_core::math::Vec3;
/// use abrash_render::rasterizer::fill_triangle_gouraud;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Define a triangle with vertex colors (RGB)
/// let v0 = ((Vec3::new(0.0, 5.0, 5.0), 5.0), Vec3::new(1.0, 0.0, 0.0)); // Red
/// let v1 = ((Vec3::new(-5.0, -5.0, 5.0), 5.0), Vec3::new(0.0, 1.0, 0.0)); // Green
/// let v2 = ((Vec3::new(5.0, -5.0, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0)); // Blue
///
/// fill_triangle_gouraud(&mut fb, &mut zb, v0, v1, v2);
/// ```
pub fn fill_triangle_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3), // ((position, w), color)
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
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

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Optimization: Pre-scale colors to 0..255 for faster interpolation and packing
        // allowing us to skip clamp/mul per pixel
        let c0 = v0.1 * 255.0;
        let c1 = v1.1 * 255.0;
        let c2 = v2.1 * 255.0;

        // Sort by y
        let mut verts = [(p0_orig, c0), (p1_orig, c1), (p2_orig, c2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, c0), (p1, c1), (p2, c2)] = verts;

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
        let (gradients, long_edge_is_left) = GouraudGradients::new(p0, p1, p2, c0, c1, c2);

        let mut edge_a = GouraudEdgeWalker::new(p0, p2, c0, c2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = GouraudEdgeWalker::new(p1, p2, c1, c2);
            }

            let (x_start, x_end, z_left, c_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.c,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.c,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 {
                    // SAFETY: Safe due to clamps on x_start and y
                    unsafe {
                        if zb.test_and_set_unchecked(x_start as usize, y as usize, z_left) {
                            fb.set_pixel_unchecked(
                                x_start as usize,
                                y as usize,
                                pack_color_fixed_i32(c_left),
                            );
                        }
                    }
                }
            } else {
                draw_scanline_gouraud_i32(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    z_left,
                    c_left,
                    gradients.dz_dx,
                    gradients.dc_dx,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::decimal_bitwise_operands)]
    fn draw_scanline_gouraud_interpolation() {
        let width = 100;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        let z_start = 5.0;
        let dz_dx = 0.01;
        let c_start = (200i64 << 16, 0, 50i64 << 16);
        let dc_dx = ((-1i32) << 16, 1i32 << 16, 0);

        draw_scanline_gouraud(&mut fb, &mut zb, 0, 0, 99, z_start, c_start, dz_dx, dc_dx);

        let p0 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p0, 0xFF00_0000 | (0xC8 << 16) | 0x32);
        let p50 = fb.get_pixel(50, 0).unwrap();
        assert_eq!(p50, 0xFF00_0000 | (0x96 << 16) | (0x32 << 8) | 0x32);
        let p99 = fb.get_pixel(99, 0).unwrap();
        assert_eq!(p99, 0xFF00_0000 | (0x65 << 16) | (0x63 << 8) | 0x32);

        let zb_slice = zb.as_slice();
        assert!((zb_slice[0] - 5.0).abs() < 0.0001);
        assert!((zb_slice[50] - 5.5).abs() < 0.0001);
    }
}
