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
    let one_i = _mm256_set1_epi32(1);

    let mask_ff = _mm256_set1_epi32(0xFF);
    let const_256 = _mm256_set1_epi32(256);
    let scale_255 = _mm256_set1_ps(255.0);
    let zero_ps = _mm256_setzero_ps();

    let blend_swar_avx2 =
        |c0: __m256i, c1: __m256i, w: __m256i, inv_w: __m256i| -> __m256i {
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

    while i + 8 <= len {
        unsafe {
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

            let y0_w = _mm256_mullo_epi32(y0, w_vec);
            let y1_w = _mm256_mullo_epi32(y1, w_vec);

            let idx00 = _mm256_add_epi32(y0_w, x0);
            let idx10 = _mm256_add_epi32(y0_w, x1);
            let idx01 = _mm256_add_epi32(y1_w, x0);
            let idx11 = _mm256_add_epi32(y1_w, x1);

            let pixels_ptr = texture.pixels.as_ptr() as *const i32;
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

            let tex_r = _mm256_cvtepi32_ps(tex_r_i);
            let tex_g = _mm256_cvtepi32_ps(tex_g_i);
            let tex_b = _mm256_cvtepi32_ps(tex_b_i);

            let mod_r = _mm256_mul_ps(tex_r, r_vec);
            let mod_g = _mm256_mul_ps(tex_g, g_vec);
            let mod_b = _mm256_mul_ps(tex_b, b_vec);

            let out_r =
                _mm256_cvttps_epi32(_mm256_min_ps(_mm256_max_ps(mod_r, zero_ps), scale_255));
            let out_g =
                _mm256_cvttps_epi32(_mm256_min_ps(_mm256_max_ps(mod_g, zero_ps), scale_255));
            let out_b =
                _mm256_cvttps_epi32(_mm256_min_ps(_mm256_max_ps(mod_b, zero_ps), scale_255));

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

                let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                let old_color = _mm256_loadu_si256(fb_ptr);
                let new_color = _mm256_blendv_epi8(old_color, out_color, write_opaque);
                _mm256_storeu_si256(fb_ptr, new_color);
            }

            // Translucent
            let write_trans =
                _mm256_andnot_si256(opaque_mask, _mm256_andnot_si256(zero_mask, mask_z_int));
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
                        *fb_slice.get_unchecked_mut(idx) =
                            blend_swar(src, dest, (255 - alpha).into(), alpha.into());
                    }
                    bit <<= 1;
                }
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

    // Scalar Tail
    let mut z_curr = z_start + (i as f32) * dz_dx;
    let mut u_curr = u_fix_start.wrapping_add(du_fix.wrapping_mul(i as i32));
    let mut v_curr = v_fix_start.wrapping_add(dv_fix.wrapping_mul(i as i32));
    let mut r_curr = r_start + (i as f32) * dr_dx;
    let mut g_curr = g_start + (i as f32) * dg_dx;
    let mut b_curr = b_start + (i as f32) * db_dx;

    while i < len {
        let depth_val = zb_slice.get_unchecked_mut(i);
        if z_curr < *depth_val {
            let color = texture.get_pixel_bilinear_fixed(u_curr, v_curr);

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
        z_curr += dz_dx;
        u_curr = u_curr.wrapping_add(du_fix);
        v_curr = v_curr.wrapping_add(dv_fix);
        r_curr += dr_dx;
        g_curr += dg_dx;
        b_curr += db_dx;
        i += 1;
    }
}
