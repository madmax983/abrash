//! Phong shading rasterizer.
//!
//! Per-pixel lighting interpolation and calculation.

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, ScreenPoint, Vec3, fast_inv_sqrt, project_triangle_to_screen};
use crate::zbuffer::ZBuffer;

use super::core::{
    FIXED_SCALE, assert_same_dimensions, color_to_u32, color_to_u32_scaled, is_backface, sort_by_y,
};

#[derive(Clone, Copy)]
struct ShadowPhongSpanStart {
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2", enable = "fma")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_phong_shadowed_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    start: ShadowPhongSpanStart,
    gradients: &ShadowPhongGradients,
    pre_diffuse_255: Vec3,
    neg_light_dir: Vec3,
    ambient_255: Vec3,
    shadow_map: &ZBuffer,
    light_vp: Mat4,
) {
    use std::arch::x86_64::{
        __m256i, _CMP_GE_OQ, _CMP_GT_OQ, _CMP_LE_OQ, _CMP_LT_OQ, _mm256_add_epi32, _mm256_add_ps,
        _mm256_and_ps, _mm256_and_si256, _mm256_andnot_ps, _mm256_blendv_epi8, _mm256_blendv_ps,
        _mm256_castps_si256, _mm256_castsi256_ps, _mm256_cmp_ps, _mm256_cmpgt_epi32,
        _mm256_cvtss_f32, _mm256_cvttps_epi32, _mm256_div_ps, _mm256_fmadd_ps, _mm256_i32gather_ps,
        _mm256_loadu_ps, _mm256_loadu_si256, _mm256_max_epi32, _mm256_max_ps, _mm256_min_epi32,
        _mm256_min_ps, _mm256_movemask_ps, _mm256_mul_ps, _mm256_mullo_epi32, _mm256_or_si256,
        _mm256_rsqrt_ps, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps, _mm256_setzero_ps,
        _mm256_setzero_si256, _mm256_slli_epi32, _mm256_storeu_ps, _mm256_storeu_si256,
        _mm256_sub_epi32, _mm256_sub_ps,
    };

    let len = fb_slice.len();
    let mut i = 0;

    let dz_dx = _mm256_set1_ps(gradients.dz_dx);
    let dq_dx = _mm256_set1_ps(gradients.dq_dx);
    let dnx_dx = _mm256_set1_ps(gradients.dnx_dx);
    let dny_dx = _mm256_set1_ps(gradients.dny_dx);
    let dnz_dx = _mm256_set1_ps(gradients.dnz_dx);
    let dwx_dx = _mm256_set1_ps(gradients.dwx_dx);
    let dwy_dx = _mm256_set1_ps(gradients.dwy_dx);
    let dwz_dx = _mm256_set1_ps(gradients.dwz_dx);

    let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(start.z), _mm256_mul_ps(dz_dx, offsets));
    let mut q_vec = _mm256_add_ps(_mm256_set1_ps(start.q), _mm256_mul_ps(dq_dx, offsets));
    let mut nx_vec = _mm256_add_ps(_mm256_set1_ps(start.nx), _mm256_mul_ps(dnx_dx, offsets));
    let mut ny_vec = _mm256_add_ps(_mm256_set1_ps(start.ny), _mm256_mul_ps(dny_dx, offsets));
    let mut nz_vec = _mm256_add_ps(_mm256_set1_ps(start.nz), _mm256_mul_ps(dnz_dx, offsets));
    let mut wx_vec = _mm256_add_ps(_mm256_set1_ps(start.wx), _mm256_mul_ps(dwx_dx, offsets));
    let mut wy_vec = _mm256_add_ps(_mm256_set1_ps(start.wy), _mm256_mul_ps(dwy_dx, offsets));
    let mut wz_vec = _mm256_add_ps(_mm256_set1_ps(start.wz), _mm256_mul_ps(dwz_dx, offsets));

    let step_8 = _mm256_set1_ps(8.0);
    let dz_step = _mm256_mul_ps(dz_dx, step_8);
    let dq_step = _mm256_mul_ps(dq_dx, step_8);
    let dnx_step = _mm256_mul_ps(dnx_dx, step_8);
    let dny_step = _mm256_mul_ps(dny_dx, step_8);
    let dnz_step = _mm256_mul_ps(dnz_dx, step_8);
    let dwx_step = _mm256_mul_ps(dwx_dx, step_8);
    let dwy_step = _mm256_mul_ps(dwy_dx, step_8);
    let dwz_step = _mm256_mul_ps(dwz_dx, step_8);

    let m00 = _mm256_set1_ps(light_vp.m[0][0]);
    let m10 = _mm256_set1_ps(light_vp.m[1][0]);
    let m20 = _mm256_set1_ps(light_vp.m[2][0]);
    let m30 = _mm256_set1_ps(light_vp.m[3][0]);
    let m01 = _mm256_set1_ps(light_vp.m[0][1]);
    let m11 = _mm256_set1_ps(light_vp.m[1][1]);
    let m21 = _mm256_set1_ps(light_vp.m[2][1]);
    let m31 = _mm256_set1_ps(light_vp.m[3][1]);
    let m02 = _mm256_set1_ps(light_vp.m[0][2]);
    let m12 = _mm256_set1_ps(light_vp.m[1][2]);
    let m22 = _mm256_set1_ps(light_vp.m[2][2]);
    let m32 = _mm256_set1_ps(light_vp.m[3][2]);
    let m03 = _mm256_set1_ps(light_vp.m[0][3]);
    let m13 = _mm256_set1_ps(light_vp.m[1][3]);
    let m23 = _mm256_set1_ps(light_vp.m[2][3]);
    let m33 = _mm256_set1_ps(light_vp.m[3][3]);

    let one = _mm256_set1_ps(1.0);
    let zero = _mm256_setzero_ps();
    let point_five = _mm256_set1_ps(0.5);
    let bias = _mm256_set1_ps(0.005);
    let sm_w = _mm256_set1_ps(shadow_map.width() as f32);
    let sm_h = _mm256_set1_ps(shadow_map.height() as f32);
    let sm_w_i32 = _mm256_set1_epi32(shadow_map.width() as i32);
    let sm_h_i32 = _mm256_set1_epi32(shadow_map.height() as i32);
    let sm_width_stride = _mm256_set1_epi32(shadow_map.width() as i32);

    let lx = _mm256_set1_ps(neg_light_dir.x);
    let ly = _mm256_set1_ps(neg_light_dir.y);
    let lz = _mm256_set1_ps(neg_light_dir.z);
    let diff_r = _mm256_set1_ps(pre_diffuse_255.x);
    let diff_g = _mm256_set1_ps(pre_diffuse_255.y);
    let diff_b = _mm256_set1_ps(pre_diffuse_255.z);
    let amb_r = _mm256_set1_ps(ambient_255.x);
    let amb_g = _mm256_set1_ps(ambient_255.y);
    let amb_b = _mm256_set1_ps(ambient_255.z);
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
    let scale_255 = _mm256_set1_ps(255.0);

    let sm_ptr = shadow_map.as_slice().as_ptr();

    while i + 8 <= len {
        unsafe {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask) != 0 {
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                let q_valid = _mm256_cmp_ps(
                    _mm256_andnot_ps(_mm256_set1_ps(-0.0), q_vec),
                    _mm256_set1_ps(1e-6),
                    _CMP_GT_OQ,
                );
                let safe_q = _mm256_blendv_ps(one, q_vec, q_valid);
                let w_recip = _mm256_div_ps(one, safe_q);

                let world_x = _mm256_mul_ps(wx_vec, w_recip);
                let world_y = _mm256_mul_ps(wy_vec, w_recip);
                let world_z = _mm256_mul_ps(wz_vec, w_recip);

                let lc_x = _mm256_add_ps(
                    _mm256_mul_ps(world_x, m00),
                    _mm256_add_ps(
                        _mm256_mul_ps(world_y, m10),
                        _mm256_add_ps(_mm256_mul_ps(world_z, m20), m30),
                    ),
                );
                let lc_y = _mm256_add_ps(
                    _mm256_mul_ps(world_x, m01),
                    _mm256_add_ps(
                        _mm256_mul_ps(world_y, m11),
                        _mm256_add_ps(_mm256_mul_ps(world_z, m21), m31),
                    ),
                );
                let lc_z = _mm256_add_ps(
                    _mm256_mul_ps(world_x, m02),
                    _mm256_add_ps(
                        _mm256_mul_ps(world_y, m12),
                        _mm256_add_ps(_mm256_mul_ps(world_z, m22), m32),
                    ),
                );
                let lc_w = _mm256_add_ps(
                    _mm256_mul_ps(world_x, m03),
                    _mm256_add_ps(
                        _mm256_mul_ps(world_y, m13),
                        _mm256_add_ps(_mm256_mul_ps(world_z, m23), m33),
                    ),
                );

                let lc_valid = _mm256_cmp_ps(lc_w, _mm256_set1_ps(1e-6), _CMP_GT_OQ);
                let safe_lc_w = _mm256_blendv_ps(one, lc_w, lc_valid);
                let inv_lc_w = _mm256_div_ps(one, safe_lc_w);

                let ndc_x = _mm256_mul_ps(lc_x, inv_lc_w);
                let ndc_y = _mm256_mul_ps(lc_y, inv_lc_w);
                let ndc_z = _mm256_mul_ps(lc_z, inv_lc_w);

                let in_frustum = _mm256_and_ps(
                    _mm256_and_ps(
                        _mm256_and_ps(
                            _mm256_cmp_ps(ndc_x, one, _CMP_LE_OQ),
                            _mm256_cmp_ps(ndc_x, _mm256_set1_ps(-1.0), _CMP_GE_OQ),
                        ),
                        _mm256_and_ps(
                            _mm256_cmp_ps(ndc_y, one, _CMP_LE_OQ),
                            _mm256_cmp_ps(ndc_y, _mm256_set1_ps(-1.0), _CMP_GE_OQ),
                        ),
                    ),
                    _mm256_and_ps(
                        _mm256_cmp_ps(ndc_z, one, _CMP_LE_OQ),
                        _mm256_cmp_ps(ndc_z, _mm256_set1_ps(-1.0), _CMP_GE_OQ),
                    ),
                );
                let shadow_test_mask = _mm256_and_ps(in_frustum, lc_valid);

                let u = _mm256_mul_ps(_mm256_add_ps(ndc_x, one), point_five);
                let v = _mm256_mul_ps(_mm256_sub_ps(one, ndc_y), point_five);

                let sm_x = _mm256_cvttps_epi32(_mm256_mul_ps(u, sm_w));
                let sm_y = _mm256_cvttps_epi32(_mm256_mul_ps(v, sm_h));

                let mut shadow_val = _mm256_setzero_ps();
                let mut sample_count = _mm256_setzero_ps();

                for y_off in -1..=1 {
                    for x_off in -1..=1 {
                        let off_x = _mm256_set1_epi32(x_off);
                        let off_y = _mm256_set1_epi32(y_off);
                        let coord_x = _mm256_add_epi32(sm_x, off_x);
                        let coord_y = _mm256_add_epi32(sm_y, off_y);

                        let in_bounds = _mm256_and_si256(
                            _mm256_and_si256(
                                _mm256_cmpgt_epi32(coord_x, _mm256_set1_epi32(-1)),
                                _mm256_cmpgt_epi32(sm_w_i32, coord_x),
                            ),
                            _mm256_and_si256(
                                _mm256_cmpgt_epi32(coord_y, _mm256_set1_epi32(-1)),
                                _mm256_cmpgt_epi32(sm_h_i32, coord_y),
                            ),
                        );

                        let safe_x = _mm256_max_epi32(
                            _mm256_setzero_si256(),
                            _mm256_min_epi32(
                                coord_x,
                                _mm256_sub_epi32(sm_w_i32, _mm256_set1_epi32(1)),
                            ),
                        );
                        let safe_y = _mm256_max_epi32(
                            _mm256_setzero_si256(),
                            _mm256_min_epi32(
                                coord_y,
                                _mm256_sub_epi32(sm_h_i32, _mm256_set1_epi32(1)),
                            ),
                        );
                        let idx =
                            _mm256_add_epi32(_mm256_mullo_epi32(safe_y, sm_width_stride), safe_x);
                        let depth_sample = _mm256_i32gather_ps(sm_ptr, idx, 4);

                        let is_shadow =
                            _mm256_cmp_ps(ndc_z, _mm256_add_ps(depth_sample, bias), _CMP_GT_OQ);
                        let valid = _mm256_castsi256_ps(in_bounds);

                        sample_count = _mm256_add_ps(sample_count, _mm256_and_ps(one, valid));
                        let lit = _mm256_andnot_ps(is_shadow, valid);
                        shadow_val = _mm256_add_ps(shadow_val, _mm256_and_ps(one, lit));
                    }
                }

                let samples_valid = _mm256_cmp_ps(sample_count, _mm256_set1_ps(0.001), _CMP_GT_OQ);
                let safe_samples = _mm256_blendv_ps(one, sample_count, samples_valid);
                let shadow_factor = _mm256_div_ps(shadow_val, safe_samples);
                let final_shadow = _mm256_blendv_ps(one, shadow_factor, shadow_test_mask);

                let nx_sq = _mm256_mul_ps(nx_vec, nx_vec);
                let ny_sq = _mm256_mul_ps(ny_vec, ny_vec);
                let nz_sq = _mm256_mul_ps(nz_vec, nz_vec);
                let len_sq = _mm256_add_ps(nx_sq, _mm256_add_ps(ny_sq, nz_sq));
                let len_valid = _mm256_cmp_ps(len_sq, _mm256_set1_ps(1e-4), _CMP_GT_OQ);
                let safe_len_sq = _mm256_blendv_ps(one, len_sq, len_valid);
                let rsqrt = _mm256_rsqrt_ps(safe_len_sq);
                let iter1 = _mm256_mul_ps(safe_len_sq, _mm256_mul_ps(rsqrt, rsqrt));
                let iter2 = _mm256_sub_ps(_mm256_set1_ps(1.5), _mm256_mul_ps(point_five, iter1));
                let inv_len = _mm256_mul_ps(rsqrt, iter2);

                let dot = _mm256_add_ps(
                    _mm256_mul_ps(nx_vec, lx),
                    _mm256_add_ps(_mm256_mul_ps(ny_vec, ly), _mm256_mul_ps(nz_vec, lz)),
                );
                let intensity = _mm256_max_ps(zero, _mm256_mul_ps(dot, inv_len));
                let intensity = _mm256_blendv_ps(zero, intensity, len_valid);
                let effective_intensity = _mm256_mul_ps(intensity, final_shadow);

                let r = _mm256_fmadd_ps(diff_r, effective_intensity, amb_r);
                let g = _mm256_fmadd_ps(diff_g, effective_intensity, amb_g);
                let b = _mm256_fmadd_ps(diff_b, effective_intensity, amb_b);

                let r_clamp = _mm256_min_ps(_mm256_max_ps(r, zero), scale_255);
                let g_clamp = _mm256_min_ps(_mm256_max_ps(g, zero), scale_255);
                let b_clamp = _mm256_min_ps(_mm256_max_ps(b, zero), scale_255);

                let r_i = _mm256_cvttps_epi32(r_clamp);
                let g_i = _mm256_cvttps_epi32(g_clamp);
                let b_i = _mm256_cvttps_epi32(b_clamp);

                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        _mm256_slli_epi32(r_i, 16),
                        _mm256_or_si256(_mm256_slli_epi32(g_i, 8), b_i),
                    ),
                );

                #[allow(clippy::cast_ptr_alignment)]
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, _mm256_castps_si256(mask));
                _mm256_storeu_si256(fb_ptr, new_color);
            }
        }

        z_vec = _mm256_add_ps(z_vec, dz_step);
        q_vec = _mm256_add_ps(q_vec, dq_step);
        nx_vec = _mm256_add_ps(nx_vec, dnx_step);
        ny_vec = _mm256_add_ps(ny_vec, dny_step);
        nz_vec = _mm256_add_ps(nz_vec, dnz_step);
        wx_vec = _mm256_add_ps(wx_vec, dwx_step);
        wy_vec = _mm256_add_ps(wy_vec, dwy_step);
        wz_vec = _mm256_add_ps(wz_vec, dwz_step);

        i += 8;
    }

    // Scalar Tail
    while i < len {
        let i_f = i as f32;
        let z = start.z + i_f * gradients.dz_dx;
        let q = start.q + i_f * gradients.dq_dx;
        let nx = start.nx + i_f * gradients.dnx_dx;
        let ny = start.ny + i_f * gradients.dny_dx;
        let nz = start.nz + i_f * gradients.dnz_dx;
        let wx = start.wx + i_f * gradients.dwx_dx;
        let wy = start.wy + i_f * gradients.dwy_dx;
        let wz = start.wz + i_f * gradients.dwz_dx;

        let depth_val = &mut zb_slice[i];
        if z < *depth_val {
            *depth_val = z;

            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            let (light_clip, light_w) = light_vp.transform_point(world_pos);
            let mut shadow_factor = 1.0;

            if light_w > 0.0 {
                let inv_light_w = 1.0 / light_w;
                let ndc_x = light_clip.x * inv_light_w;
                let ndc_y = light_clip.y * inv_light_w;
                let ndc_z = light_clip.z * inv_light_w;

                if (-1.0..=1.0).contains(&ndc_x)
                    && (-1.0..=1.0).contains(&ndc_y)
                    && (-1.0..=1.0).contains(&ndc_z)
                {
                    let u = (ndc_x + 1.0) * 0.5;
                    let v = (1.0 - ndc_y) * 0.5;
                    let sm_x = (u * _mm256_cvtss_f32(sm_w)) as i32;
                    let sm_y = (v * _mm256_cvtss_f32(sm_h)) as i32;

                    let mut shadow_sum = 0.0;
                    let mut samples = 0.0;
                    let bias_s = _mm256_cvtss_f32(bias);

                    for y_off in -1..=1 {
                        for x_off in -1..=1 {
                            if let Some(closest_depth) =
                                shadow_map.get_depth(sm_x + x_off, sm_y + y_off)
                            {
                                if ndc_z <= closest_depth + bias_s {
                                    shadow_sum += 1.0;
                                }
                                samples += 1.0;
                            }
                        }
                    }
                    if samples > 0.0 {
                        shadow_factor = shadow_sum / samples;
                    }
                }
            }

            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;
            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity * shadow_factor;
            let final_color_vec = ambient_255 + diffuse;
            fb_slice[i] = color_to_u32_scaled(final_color_vec);
        }
        i += 1;
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn draw_scanline_point_lit_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
    gradients: &ShadowPhongGradients,
    base_color_255: Vec3,
    light_pos: Vec3,
    light_color: Vec3,
    attenuation: Vec3,
) {
    use std::arch::x86_64::{
        __m256i, _CMP_GT_OQ, _CMP_LT_OQ, _mm256_add_ps, _mm256_andnot_ps, _mm256_blendv_epi8,
        _mm256_blendv_ps, _mm256_castps_si256, _mm256_cmp_ps, _mm256_cvttps_epi32, _mm256_div_ps,
        _mm256_fmadd_ps, _mm256_loadu_ps, _mm256_loadu_si256, _mm256_max_ps, _mm256_min_ps,
        _mm256_movemask_ps, _mm256_mul_ps, _mm256_or_si256, _mm256_rsqrt_ps, _mm256_set_ps,
        _mm256_set1_epi32, _mm256_set1_ps, _mm256_setzero_ps, _mm256_slli_epi32, _mm256_storeu_ps,
        _mm256_storeu_si256, _mm256_sub_ps,
    };

    let len = fb_slice.len();
    let mut i = 0;

    let dz_dx = _mm256_set1_ps(gradients.dz_dx);
    let dq_dx = _mm256_set1_ps(gradients.dq_dx);
    let dnx_dx = _mm256_set1_ps(gradients.dnx_dx);
    let dny_dx = _mm256_set1_ps(gradients.dny_dx);
    let dnz_dx = _mm256_set1_ps(gradients.dnz_dx);
    let dwx_dx = _mm256_set1_ps(gradients.dwx_dx);
    let dwy_dx = _mm256_set1_ps(gradients.dwy_dx);
    let dwz_dx = _mm256_set1_ps(gradients.dwz_dx);

    let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx, offsets));
    let mut q_vec = _mm256_add_ps(_mm256_set1_ps(q), _mm256_mul_ps(dq_dx, offsets));
    let mut nx_vec = _mm256_add_ps(_mm256_set1_ps(nx), _mm256_mul_ps(dnx_dx, offsets));
    let mut ny_vec = _mm256_add_ps(_mm256_set1_ps(ny), _mm256_mul_ps(dny_dx, offsets));
    let mut nz_vec = _mm256_add_ps(_mm256_set1_ps(nz), _mm256_mul_ps(dnz_dx, offsets));
    let mut wx_vec = _mm256_add_ps(_mm256_set1_ps(wx), _mm256_mul_ps(dwx_dx, offsets));
    let mut wy_vec = _mm256_add_ps(_mm256_set1_ps(wy), _mm256_mul_ps(dwy_dx, offsets));
    let mut wz_vec = _mm256_add_ps(_mm256_set1_ps(wz), _mm256_mul_ps(dwz_dx, offsets));

    let step_8 = _mm256_set1_ps(8.0);
    let dz_step = _mm256_mul_ps(dz_dx, step_8);
    let dq_step = _mm256_mul_ps(dq_dx, step_8);
    let dnx_step = _mm256_mul_ps(dnx_dx, step_8);
    let dny_step = _mm256_mul_ps(dny_dx, step_8);
    let dnz_step = _mm256_mul_ps(dnz_dx, step_8);
    let dwx_step = _mm256_mul_ps(dwx_dx, step_8);
    let dwy_step = _mm256_mul_ps(dwy_dx, step_8);
    let dwz_step = _mm256_mul_ps(dwz_dx, step_8);

    let one = _mm256_set1_ps(1.0);
    let zero = _mm256_setzero_ps();
    let epsilon = _mm256_set1_ps(0.0001);
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
    let scale_255 = _mm256_set1_ps(255.0);

    let lx = _mm256_set1_ps(light_pos.x);
    let ly = _mm256_set1_ps(light_pos.y);
    let lz = _mm256_set1_ps(light_pos.z);

    let att_c = _mm256_set1_ps(attenuation.x);
    let att_l = _mm256_set1_ps(attenuation.y);
    let att_q = _mm256_set1_ps(attenuation.z);

    let base_r = _mm256_set1_ps(base_color_255.x);
    let base_g = _mm256_set1_ps(base_color_255.y);
    let base_b = _mm256_set1_ps(base_color_255.z);

    let light_r = _mm256_set1_ps(light_color.x);
    let light_g = _mm256_set1_ps(light_color.y);
    let light_b = _mm256_set1_ps(light_color.z);

    while i + 8 <= len {
        unsafe {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask) != 0 {
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                let q_abs = _mm256_andnot_ps(_mm256_set1_ps(-0.0), q_vec);
                let q_valid = _mm256_cmp_ps(q_abs, epsilon, _CMP_GT_OQ);
                let safe_q = _mm256_blendv_ps(one, q_vec, q_valid);
                let w_recip = _mm256_div_ps(one, safe_q);

                let wx_real = _mm256_mul_ps(wx_vec, w_recip);
                let wy_real = _mm256_mul_ps(wy_vec, w_recip);
                let wz_real = _mm256_mul_ps(wz_vec, w_recip);

                let lv_x = _mm256_sub_ps(lx, wx_real);
                let lv_y = _mm256_sub_ps(ly, wy_real);
                let lv_z = _mm256_sub_ps(lz, wz_real);

                let dist_sq = _mm256_fmadd_ps(
                    lv_z,
                    lv_z,
                    _mm256_fmadd_ps(lv_y, lv_y, _mm256_mul_ps(lv_x, lv_x)),
                );

                let epsilon_sq = _mm256_mul_ps(epsilon, epsilon);
                let dist_valid = _mm256_cmp_ps(dist_sq, epsilon_sq, _CMP_GT_OQ);
                let safe_dist_sq = _mm256_blendv_ps(one, dist_sq, dist_valid);
                let rsqrt_dist = _mm256_rsqrt_ps(safe_dist_sq);

                let point_five = _mm256_set1_ps(0.5);
                let three_halves = _mm256_set1_ps(1.5);

                let iter1 = _mm256_mul_ps(safe_dist_sq, _mm256_mul_ps(rsqrt_dist, rsqrt_dist));
                let iter2 = _mm256_sub_ps(three_halves, _mm256_mul_ps(point_five, iter1));
                let inv_dist = _mm256_mul_ps(rsqrt_dist, iter2);

                let dist = _mm256_mul_ps(dist_sq, inv_dist);

                let denom = _mm256_add_ps(
                    att_c,
                    _mm256_fmadd_ps(att_l, dist, _mm256_mul_ps(att_q, dist_sq)),
                );
                let safe_denom = _mm256_max_ps(epsilon, denom);
                let att_factor = _mm256_div_ps(one, safe_denom);

                let len_sq = _mm256_fmadd_ps(
                    nz_vec,
                    nz_vec,
                    _mm256_fmadd_ps(ny_vec, ny_vec, _mm256_mul_ps(nx_vec, nx_vec)),
                );
                let inv_len = _mm256_rsqrt_ps(len_sq);

                let dot_unorm = _mm256_fmadd_ps(
                    nz_vec,
                    lv_z,
                    _mm256_fmadd_ps(ny_vec, lv_y, _mm256_mul_ps(nx_vec, lv_x)),
                );

                let intensity_raw = _mm256_mul_ps(dot_unorm, _mm256_mul_ps(inv_len, inv_dist));
                let intensity = _mm256_max_ps(zero, intensity_raw);

                let combined = _mm256_mul_ps(intensity, att_factor);

                let r = _mm256_mul_ps(base_r, _mm256_mul_ps(light_r, combined));
                let g = _mm256_mul_ps(base_g, _mm256_mul_ps(light_g, combined));
                let b = _mm256_mul_ps(base_b, _mm256_mul_ps(light_b, combined));

                let r_clamp = _mm256_min_ps(_mm256_max_ps(r, zero), scale_255);
                let g_clamp = _mm256_min_ps(_mm256_max_ps(g, zero), scale_255);
                let b_clamp = _mm256_min_ps(_mm256_max_ps(b, zero), scale_255);

                let r_i = _mm256_cvttps_epi32(r_clamp);
                let g_i = _mm256_cvttps_epi32(g_clamp);
                let b_i = _mm256_cvttps_epi32(b_clamp);

                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        _mm256_slli_epi32(r_i, 16),
                        _mm256_or_si256(_mm256_slli_epi32(g_i, 8), b_i),
                    ),
                );

                #[allow(clippy::cast_ptr_alignment)]
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                let mask_int = _mm256_castps_si256(mask);
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }
        }

        z_vec = _mm256_add_ps(z_vec, dz_step);
        q_vec = _mm256_add_ps(q_vec, dq_step);
        nx_vec = _mm256_add_ps(nx_vec, dnx_step);
        ny_vec = _mm256_add_ps(ny_vec, dny_step);
        nz_vec = _mm256_add_ps(nz_vec, dnz_step);
        wx_vec = _mm256_add_ps(wx_vec, dwx_step);
        wy_vec = _mm256_add_ps(wy_vec, dwy_step);
        wz_vec = _mm256_add_ps(wz_vec, dwz_step);

        i += 8;
    }

    while i < len {
        let i_f = i as f32;
        let z = z + i_f * gradients.dz_dx;
        let q = q + i_f * gradients.dq_dx;
        let nx = nx + i_f * gradients.dnx_dx;
        let ny = ny + i_f * gradients.dny_dx;
        let nz = nz + i_f * gradients.dnz_dx;
        let wx = wx + i_f * gradients.dwx_dx;
        let wy = wy + i_f * gradients.dwy_dx;
        let wz = wz + i_f * gradients.dwz_dx;

        let pixel = &mut fb_slice[i];
        let depth_val = &mut zb_slice[i];

        if z < *depth_val {
            *depth_val = z;
            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            let lv_x = light_pos.x - world_pos.x;
            let lv_y = light_pos.y - world_pos.y;
            let lv_z = light_pos.z - world_pos.z;

            let dist_sq = lv_x * lv_x + lv_y * lv_y + lv_z * lv_z;

            // ⚡ Bolt: Using `sqrt().recip()` is often faster and strictly more precise than `fast_inv_sqrt`
            // on modern architectures with dedicated floating-point units.
            let inv_dist = dist_sq.sqrt().recip();
            let dist = dist_sq * inv_dist;

            let att_factor = 1.0 / (attenuation.x + attenuation.y * dist + attenuation.z * dist_sq);

            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * lv_x + ny * lv_y + nz * lv_z;

            let intensity = if len_sq > 0.0001 && dist > 0.0001 {
                let inv_len = len_sq.sqrt().recip();

                (dot_unorm * inv_len * inv_dist).max(0.0)
            } else {
                0.0
            };

            let diffuse = base_color_255 * light_color * intensity * att_factor;
            *pixel = color_to_u32_scaled(diffuse);
        }
        i += 1;
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_point_lit(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: ShadowPhongSpanStart,
    gradients: &ShadowPhongGradients,
    base_color_255: Vec3,
    light_pos: Vec3,
    light_color: Vec3,
    attenuation: Vec3,
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut q = start.q;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;
    let mut wx = start.wx;
    let mut wy = start.wy;
    let mut wz = start.wz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        wx += diff_f * gradients.dwx_dx;
        wy += diff_f * gradients.dwy_dx;
        wz += diff_f * gradients.dwz_dx;
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
    let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];
    let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if fb_slice.len() >= 32 && is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
        unsafe {
            draw_scanline_point_lit_simd(
                fb_slice,
                zb_slice,
                z,
                q,
                nx,
                ny,
                nz,
                wx,
                wy,
                wz,
                gradients,
                base_color_255,
                light_pos,
                light_color,
                attenuation,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };

            // Recover world position
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            // Light vector
            let lv_x = light_pos.x - world_pos.x;
            let lv_y = light_pos.y - world_pos.y;
            let lv_z = light_pos.z - world_pos.z;

            let dist_sq = lv_x * lv_x + lv_y * lv_y + lv_z * lv_z;

            // ⚡ Bolt: Using `sqrt().recip()` is often faster and strictly more precise than `fast_inv_sqrt`
            // on modern architectures with dedicated floating-point units.
            let inv_dist = dist_sq.sqrt().recip();
            let dist = dist_sq * inv_dist;

            // Attenuation
            let att_factor = 1.0 / (attenuation.x + attenuation.y * dist + attenuation.z * dist_sq);

            // Lighting
            // Deferred Normalization
            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * lv_x + ny * lv_y + nz * lv_z;

            let intensity = if len_sq > 0.0001 && dist > 0.0001 {
                let inv_len = len_sq.sqrt().recip();

                (dot_unorm * inv_len * inv_dist).max(0.0)
            } else {
                0.0
            };

            let diffuse = base_color_255 * light_color * intensity * att_factor;
            *pixel = color_to_u32_scaled(diffuse);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
        wx += gradients.dwx_dx;
        wy += gradients.dwy_dx;
        wz += gradients.dwz_dx;
    }
}

/// Fill a 3D triangle with Point Lighting.
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_point_lit(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec3), // ((ClipPos, W), Normal, WorldPos)
    v1: ((Vec3, f32), Vec3, Vec3),
    v2: ((Vec3, f32), Vec3, Vec3),
    color: Vec3,
    light_pos: Vec3,
    light_color: Vec3,
    attenuation: Vec3,
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

        // Normal * inv_w
        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        // WorldPos * inv_w
        let w0 = v0.2 * inv_w0;
        let w1 = v1.2 * inv_w1;
        let w2 = v2.2 * inv_w2;

        let mut verts = [(p0_orig, n0, w0), (p1_orig, n1, w1), (p2_orig, n2, w2)];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, n0, w0), (p1, n1, w1), (p2, n2, w2)] = verts;

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
        // Reuse ShadowPhongGradients/Walker as they match the ((Clip,W), N, World) layout
        let (gradients, long_edge_is_left) =
            ShadowPhongGradients::new(p0, p1, p2, q0, q1, q2, n0, n1, n2, w0, w1, w2);

        let mut edge_a = ShadowPhongEdgeWalker::new(p0, p2, q0, q2, n0, n2, w0, w2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = ShadowPhongEdgeWalker::new(p0, p1, q0, q1, n0, n1, w0, w1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = ShadowPhongEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        // Lighting constants (pre-scaled)
        let base_color_255 = color * 255.0;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = ShadowPhongEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            }

            let (
                x_start,
                x_end,
                z_left,
                nx_left,
                ny_left,
                nz_left,
                wx_left,
                wy_left,
                wz_left,
                q_left,
            ) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                    edge_a.wx,
                    edge_a.wy,
                    edge_a.wz,
                    edge_a.q,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                    edge_b.wx,
                    edge_b.wy,
                    edge_b.wz,
                    edge_b.q,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_point_lit(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    ShadowPhongSpanStart {
                        z: z_left,
                        q: q_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                        wx: wx_left,
                        wy: wy_left,
                        wz: wz_left,
                    },
                    &gradients,
                    base_color_255,
                    light_pos,
                    light_color,
                    attenuation,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_phong_shadowed(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: ShadowPhongSpanStart,
    gradients: &ShadowPhongGradients,
    pre_diffuse_255: Vec3,
    neg_light_dir: Vec3,
    ambient_255: Vec3,
    shadow_map: &ZBuffer,
    light_vp: Mat4,
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut q = start.q;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;
    let mut wx = start.wx;
    let mut wy = start.wy;
    let mut wz = start.wz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        wx += diff_f * gradients.dwx_dx;
        wy += diff_f * gradients.dwy_dx;
        wz += diff_f * gradients.dwz_dx;
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
    let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];
    let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if fb_slice.len() >= 32 && is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
        unsafe {
            draw_scanline_phong_shadowed_simd(
                fb_slice,
                zb_slice,
                ShadowPhongSpanStart {
                    z,
                    q,
                    nx,
                    ny,
                    nz,
                    wx,
                    wy,
                    wz,
                },
                gradients,
                pre_diffuse_255,
                neg_light_dir,
                ambient_255,
                shadow_map,
                light_vp,
            );
        }
        return;
    }

    // Shadow Map dimensions
    let sm_w = shadow_map.width() as f32;
    let sm_h = shadow_map.height() as f32;
    let bias = 0.005; // Bias to prevent shadow acne

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };

            // Recover world position
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            // Shadow Test
            let (light_clip, light_w) = light_vp.transform_point(world_pos);
            let mut shadow_factor = 1.0;

            if light_w > 0.0 {
                let inv_light_w = 1.0 / light_w;
                let ndc_x = light_clip.x * inv_light_w;
                let ndc_y = light_clip.y * inv_light_w;
                let ndc_z = light_clip.z * inv_light_w;

                // Check if inside light frustum
                if (-1.0..=1.0).contains(&ndc_x)
                    && (-1.0..=1.0).contains(&ndc_y)
                    && (-1.0..=1.0).contains(&ndc_z)
                {
                    // Map to texture coordinates [0, 1]
                    let u = (ndc_x + 1.0) * 0.5;
                    let v = (1.0 - ndc_y) * 0.5; // Flip Y for texture lookup

                    let sm_x = (u * sm_w) as i32;
                    let sm_y = (v * sm_h) as i32;

                    // PCF (Percentage Closer Filtering) 3x3
                    let mut shadow_sum = 0.0;
                    let mut samples = 0.0;

                    for y_off in -1..=1 {
                        for x_off in -1..=1 {
                            if let Some(closest_depth) =
                                shadow_map.get_depth(sm_x + x_off, sm_y + y_off)
                            {
                                if ndc_z > closest_depth + bias {
                                    // In shadow
                                } else {
                                    // Lit
                                    shadow_sum += 1.0;
                                }
                                samples += 1.0;
                            }
                        }
                    }

                    if samples > 0.0 {
                        shadow_factor = shadow_sum / samples;
                    }
                }
            }

            // Lighting
            // Deferred Normalization
            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity * shadow_factor;
            let final_color_vec = ambient_255 + diffuse;
            *pixel = color_to_u32_scaled(final_color_vec);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
        wx += gradients.dwx_dx;
        wy += gradients.dwy_dx;
        wz += gradients.dwz_dx;
    }
}

/// Fill a 3D triangle with Phong Shading and Shadow Mapping.
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_phong_shadowed(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec3), // ((ClipPos, W), Normal, WorldPos)
    v1: ((Vec3, f32), Vec3, Vec3),
    v2: ((Vec3, f32), Vec3, Vec3),
    color: Vec3,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
    shadow_map: &ZBuffer,
    light_vp: Mat4,
) {
    assert_same_dimensions(fb, zb);

    // Note: We use clip_triangle_to_frustum which uses Lerp.
    // Ensure ((Vec3, f32), Vec3, Vec3) implements Lerp in clipping.rs
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

        // Normal * inv_w
        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        // WorldPos * inv_w
        let w0 = v0.2 * inv_w0;
        let w1 = v1.2 * inv_w1;
        let w2 = v2.2 * inv_w2;

        let mut verts = [(p0_orig, n0, w0), (p1_orig, n1, w1), (p2_orig, n2, w2)];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, n0, w0), (p1, n1, w1), (p2, n2, w2)] = verts;

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
            ShadowPhongGradients::new(p0, p1, p2, q0, q1, q2, n0, n1, n2, w0, w1, w2);

        let mut edge_a = ShadowPhongEdgeWalker::new(p0, p2, q0, q2, n0, n2, w0, w2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = ShadowPhongEdgeWalker::new(p0, p1, q0, q1, n0, n1, w0, w1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = ShadowPhongEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        // Lighting constants
        let pre_diffuse_255 = color * light_color * 255.0;
        let neg_light_dir = light_dir * -1.0;
        let ambient_255 = ambient * 255.0;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = ShadowPhongEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            }

            let (
                x_start,
                x_end,
                z_left,
                nx_left,
                ny_left,
                nz_left,
                wx_left,
                wy_left,
                wz_left,
                q_left,
            ) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                    edge_a.wx,
                    edge_a.wy,
                    edge_a.wz,
                    edge_a.q,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                    edge_b.wx,
                    edge_b.wy,
                    edge_b.wz,
                    edge_b.q,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_phong_shadowed(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    ShadowPhongSpanStart {
                        z: z_left,
                        q: q_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                        wx: wx_left,
                        wy: wy_left,
                        wz: wz_left,
                    },
                    &gradients,
                    pre_diffuse_255,
                    neg_light_dir,
                    ambient_255,
                    shadow_map,
                    light_vp,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

struct ShadowPhongGradients {
    dz_dx: f32,
    dq_dx: f32,
    dnx_dx: f32,
    dny_dx: f32,
    dnz_dx: f32,
    dwx_dx: f32,
    dwy_dx: f32,
    dwz_dx: f32,
}

impl ShadowPhongGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
        w0: Vec3,
        w1: Vec3,
        w2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;
        let uwx = w1.x - w0.x;
        let uwy = w1.y - w0.y;
        let uwz = w1.z - w0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;
        let vwx = w2.x - w0.x;
        let vwy = w2.y - w0.y;
        let vwz = w2.z - w0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        let nx_wx = uy * vwx - uwx * vy;
        let dwx_dx = nx_wx * inv_nz;

        let nx_wy = uy * vwy - uwy * vy;
        let dwy_dx = nx_wy * inv_nz;

        let nx_wz = uy * vwz - uwz * vy;
        let dwz_dx = nx_wz * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                dnx_dx,
                dny_dx,
                dnz_dx,
                dwx_dx,
                dwy_dx,
                dwz_dx,
            },
            nz > 0.0,
        )
    }
}

struct ShadowPhongEdgeWalker {
    x: i64,
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
    dwx_dy: f32,
    dwy_dy: f32,
    dwz_dy: f32,
}

impl ShadowPhongEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        n_start: Vec3,
        n_end: Vec3,
        w_start: Vec3,
        w_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;
        let dwx_dy = (w_end.x - w_start.x) * inv_h;
        let dwy_dy = (w_end.y - w_start.y) * inv_h;
        let dwz_dy = (w_end.z - w_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            wx: w_start.x,
            wy: w_start.y,
            wz: w_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
            dwx_dy,
            dwy_dy,
            dwz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
        self.wx += self.dwx_dy;
        self.wy += self.dwy_dy;
        self.wz += self.dwz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
        self.wx += self.dwx_dy * n_f;
        self.wy += self.dwy_dy * n_f;
        self.wz += self.dwz_dy * n_f;
    }
}

#[derive(Clone, Copy)]
struct PhongSpanStart {
    z: f32,
    // q unused in optimization
    nx: f32,
    ny: f32,
    nz: f32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_phong_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled by 255.0
    neg_light_dir: Vec3,
    ambient_255: Vec3, // Pre-scaled by 255.0
) {
    unsafe {
        use std::arch::x86_64::{
            __m256i, _CMP_GT_OQ, _CMP_LT_OQ, _mm256_add_ps, _mm256_blendv_epi8, _mm256_blendv_ps,
            _mm256_castps_si256, _mm256_cmp_ps, _mm256_cvttps_epi32, _mm256_loadu_ps,
            _mm256_loadu_si256, _mm256_max_ps, _mm256_min_ps, _mm256_movemask_ps, _mm256_mul_ps,
            _mm256_or_si256, _mm256_rsqrt_ps, _mm256_set_ps, _mm256_set1_epi32, _mm256_set1_ps,
            _mm256_setzero_ps, _mm256_slli_epi32, _mm256_storeu_ps, _mm256_storeu_si256,
            _mm256_sub_ps,
        };

        let len = fb_slice.len();
        let mut i = 0;

        // Load constants
        let dz_dx_vec = _mm256_set1_ps(gradients.dz_dx);
        let dnx_dx_vec = _mm256_set1_ps(gradients.dnx_dx);
        let dny_dx_vec = _mm256_set1_ps(gradients.dny_dx);
        let dnz_dx_vec = _mm256_set1_ps(gradients.dnz_dx);

        let lx = _mm256_set1_ps(neg_light_dir.x);
        let ly = _mm256_set1_ps(neg_light_dir.y);
        let lz = _mm256_set1_ps(neg_light_dir.z);

        let diff_r = _mm256_set1_ps(pre_diffuse_255.x);
        let diff_g = _mm256_set1_ps(pre_diffuse_255.y);
        let diff_b = _mm256_set1_ps(pre_diffuse_255.z);

        let amb_r = _mm256_set1_ps(ambient_255.x);
        let amb_g = _mm256_set1_ps(ambient_255.y);
        let amb_b = _mm256_set1_ps(ambient_255.z);

        let epsilon = _mm256_set1_ps(0.0001);
        let zero = _mm256_setzero_ps();
        let one = _mm256_set1_ps(1.0);
        let one_point_five = _mm256_set1_ps(1.5);
        let zero_point_five = _mm256_set1_ps(0.5);
        let scale_255 = _mm256_set1_ps(255.0);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        // Initial offsets for 8 pixels
        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx_vec, offsets));
        let mut nx_vec = _mm256_add_ps(_mm256_set1_ps(nx), _mm256_mul_ps(dnx_dx_vec, offsets));
        let mut ny_vec = _mm256_add_ps(_mm256_set1_ps(ny), _mm256_mul_ps(dny_dx_vec, offsets));
        let mut nz_vec = _mm256_add_ps(_mm256_set1_ps(nz), _mm256_mul_ps(dnz_dx_vec, offsets));

        // Steps for 8 pixels
        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let dnx_step = _mm256_mul_ps(dnx_dx_vec, _mm256_set1_ps(8.0));
        let dny_step = _mm256_mul_ps(dny_dx_vec, _mm256_set1_ps(8.0));
        let dnz_step = _mm256_mul_ps(dnz_dx_vec, _mm256_set1_ps(8.0));

        while i + 8 <= len {
            // Load depth buffer
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Z-test
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_int = _mm256_castps_si256(mask);

            // If any pixel passes Z-test
            if _mm256_movemask_ps(mask) != 0 {
                // Update Z-buffer
                // Optimization: Use load-blend-store instead of maskstore which can be slow
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Shading
                let nx_sq = _mm256_mul_ps(nx_vec, nx_vec);
                let ny_sq = _mm256_mul_ps(ny_vec, ny_vec);
                let nz_sq = _mm256_mul_ps(nz_vec, nz_vec);
                let len_sq = _mm256_add_ps(nx_sq, _mm256_add_ps(ny_sq, nz_sq));

                // Check if len_sq > epsilon
                let len_valid = _mm256_cmp_ps(len_sq, epsilon, _CMP_GT_OQ);

                // Calculate rsqrt. Avoid rsqrt(0) by blending with 1.0 (doesn't matter what value, masked out later)
                let safe_len_sq = _mm256_blendv_ps(one, len_sq, len_valid);
                let rsqrt = _mm256_rsqrt_ps(safe_len_sq);

                // Newton-Raphson iteration: y = y * (1.5 - 0.5 * x * y * y)
                let iter1 = _mm256_mul_ps(safe_len_sq, _mm256_mul_ps(rsqrt, rsqrt));
                let iter2 = _mm256_sub_ps(one_point_five, _mm256_mul_ps(zero_point_five, iter1));
                let inv_len = _mm256_mul_ps(rsqrt, iter2);

                // Dot product (unnormalized)
                let dot_x = _mm256_mul_ps(nx_vec, lx);
                let dot_y = _mm256_mul_ps(ny_vec, ly);
                let dot_z = _mm256_mul_ps(nz_vec, lz);
                let dot_unorm = _mm256_add_ps(dot_x, _mm256_add_ps(dot_y, dot_z));

                // Intensity
                let intensity_raw = _mm256_mul_ps(dot_unorm, inv_len);
                let intensity = _mm256_max_ps(zero, intensity_raw);

                // Apply mask for valid length
                let intensity = _mm256_blendv_ps(zero, intensity, len_valid);

                // Calculate Color (Pre-scaled)
                let r = _mm256_add_ps(amb_r, _mm256_mul_ps(diff_r, intensity));
                let g = _mm256_add_ps(amb_g, _mm256_mul_ps(diff_g, intensity));
                let b = _mm256_add_ps(amb_b, _mm256_mul_ps(diff_b, intensity));

                // Clamp and convert to u32
                // Clamp 0.0-255.0
                let r_clamp = _mm256_min_ps(_mm256_max_ps(r, zero), scale_255);
                let g_clamp = _mm256_min_ps(_mm256_max_ps(g, zero), scale_255);
                let b_clamp = _mm256_min_ps(_mm256_max_ps(b, zero), scale_255);

                // Already scaled
                let r_255 = r_clamp;
                let g_255 = g_clamp;
                let b_255 = b_clamp;

                // Convert to i32 (truncation match scalar?) Scalar uses `as u32` which is truncation.
                // cvttps truncates.
                let r_i = _mm256_cvttps_epi32(r_255);
                let g_i = _mm256_cvttps_epi32(g_255);
                let b_i = _mm256_cvttps_epi32(b_255);

                // Pack: 0xFF000000 | (r << 16) | (g << 8) | b
                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        _mm256_slli_epi32(r_i, 16),
                        _mm256_or_si256(_mm256_slli_epi32(g_i, 8), b_i),
                    ),
                );

                // Store pixels
                #[allow(clippy::cast_ptr_alignment)]
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                // blendv_epi8 blends based on the high bit of each byte.
                // Our mask is 32-bit 0xFFFFFFFF or 0x00000000, so it works for bytes too.
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }

            // Advance
            z_vec = _mm256_add_ps(z_vec, dz_step);
            nx_vec = _mm256_add_ps(nx_vec, dnx_step);
            ny_vec = _mm256_add_ps(ny_vec, dny_step);
            nz_vec = _mm256_add_ps(nz_vec, dnz_step);

            i += 8;
        }

        // Scalar tail loop
        while i < len {
            let i_f = i as f32;
            let z = z + i_f * gradients.dz_dx;
            let nx = nx + i_f * gradients.dnx_dx;
            let ny = ny + i_f * gradients.dny_dx;
            let nz = nz + i_f * gradients.dnz_dx;

            let pixel = &mut fb_slice[i];
            let depth_val = &mut zb_slice[i];

            if z < *depth_val {
                *depth_val = z;

                let len_sq = nx * nx + ny * ny + nz * nz;
                let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

                let intensity = if len_sq > 0.0001 {
                    let inv_len = fast_inv_sqrt(len_sq);
                    (dot_unorm * inv_len).max(0.0)
                } else {
                    0.0
                };

                let diffuse = pre_diffuse_255 * intensity;
                let final_color_vec = ambient_255 + diffuse;
                *pixel = color_to_u32_scaled(final_color_vec);
            }

            i += 1;
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PhongSpanStart,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled
    neg_light_dir: Vec3,
    ambient_255: Vec3, // Pre-scaled
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
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
    let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];
    let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if fb_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_phong_simd(
                fb_slice,
                zb_slice,
                z,
                nx,
                ny,
                nz,
                gradients,
                pre_diffuse_255,
                neg_light_dir,
                ambient_255,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity;
            let final_color_vec = ambient_255 + diffuse;
            *pixel = color_to_u32_scaled(final_color_vec);
        }

        z += gradients.dz_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
    }
}

/// Fill a 3D triangle with Phong Shading.
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3),
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
    color: Vec3,
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

        // Prepare attributes: q=1/w, n/w
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        let mut verts = [(p0_orig, n0), (p1_orig, n1), (p2_orig, n2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, n0), (p1, n1), (p2, n2)] = verts;

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
        let (gradients, long_edge_is_left) = PhongGradients::new(p0, p1, p2, n0, n1, n2);

        let mut edge_a = PhongEdgeWalker::new(p0, p2, n0, n2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PhongEdgeWalker::new(p0, p1, n0, n1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PhongEdgeWalker::new(p1, p2, n1, n2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        // Precalculate lighting constants
        let pre_diffuse_255 = color * light_color * 255.0;
        let neg_light_dir = light_dir * -1.0;
        let ambient_255 = ambient * 255.0;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PhongEdgeWalker::new(p1, p2, n1, n2);
            }

            let (x_start, x_end, z_left, nx_left, ny_left, nz_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_phong(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    PhongSpanStart {
                        z: z_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                    },
                    &gradients,
                    pre_diffuse_255,
                    neg_light_dir,
                    ambient_255,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Fill a 3D triangle with custom lighting
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_lit(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    normal: Vec3,
    base_color: Vec3,
    ambient_color: Vec3,
    light_dir: Vec3,
    light_color: Vec3,
) {
    let ambient = base_color * ambient_color;
    let intensity = normal.dot(light_dir * -1.0).max(0.0);
    let diffuse = base_color * light_color * intensity;
    let final_color = ambient + diffuse;
    let color_u32 = color_to_u32(final_color);
    crate::rasterizer::flat::fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

#[derive(Clone, Copy)]
struct PhongGradients {
    dz_dx: f32,
    dnx_dx: f32,
    dny_dx: f32,
    dnz_dx: f32,
}

impl PhongGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        (
            Self {
                dz_dx,
                dnx_dx,
                dny_dx,
                dnz_dx,
            },
            nz > 0.0,
        )
    }
}

struct PhongEdgeWalker {
    x: i64,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
}

impl PhongEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(p_start: ScreenPoint, p_end: ScreenPoint, n_start: Vec3, n_end: Vec3) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            dx_dy,
            dz_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
    }
}
