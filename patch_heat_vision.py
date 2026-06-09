import re

with open("crates/abrash-render/src/heat_vision.rs", "r") as f:
    content = f.read()

# Remove generate_lut
content = re.sub(r'const fn generate_lut\(\).*?\}\nlut\n\}\n', '', content, flags=re.DOTALL)

# Modify apply_heat_vision
content = re.sub(r'const LUT: \[u32; 1024\] = generate_lut\(\);\n\n    if fb.width\(\)', 'if fb.width()', content)
content = re.sub(r'        apply_heat_vision_simd\(pixels, depths, min_z, scale, &LUT\);', '        apply_heat_vision_simd(pixels, depths, min_z, scale);', content)

scalar_loop = """
    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010; // Very Dark Blue Background
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023); // Clamp strictly to 1023

        // SAFETY: t is strictly clamped to 1023 above, which is within the bounds of the 1024-element LUT.
        *pixel = unsafe { *LUT.get_unchecked(t as usize) };
    }
"""

new_scalar_loop = """
    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010; // Very Dark Blue Background
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        let (r, g, b) = if t < 256 {
            (255, t, 0)
        } else if t < 512 {
            (255 - (t - 256), 255, 0)
        } else if t < 768 {
            (0, 255, t - 512)
        } else {
            (0, 255 - (t - 768), 255)
        };

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }
"""
content = content.replace(scalar_loop, new_scalar_loop)

# Modify SIMD
simd_sig = """unsafe fn apply_heat_vision_simd(
    pixels: &mut [u32],
    depths: &[f32],
    min_z: f32,
    scale: f32,
    lut: &[u32; 1024],
) {"""

new_simd_sig = """unsafe fn apply_heat_vision_simd(
    pixels: &mut [u32],
    depths: &[f32],
    min_z: f32,
    scale: f32,
) {"""
content = content.replace(simd_sig, new_simd_sig)


# The simd gather part
simd_gather = """
    let min_z_vec = _mm256_set1_ps(min_z);
    let scale_vec = _mm256_set1_ps(scale);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let max_t_vec = _mm256_set1_epi32(1023);
    let bg_color = _mm256_set1_epi32(0xFF00_0010_u32 as i32);
    let lut_ptr = lut.as_ptr().cast::<i32>();

    while i + 8 <= len {
        let depth_ptr = depths.as_ptr().add(i);
        let depth_val = _mm256_loadu_ps(depth_ptr);

        // depth == f32::INFINITY
        let is_inf = _mm256_cmp_ps(depth_val, inf_vec, _CMP_EQ_OQ);
        let is_inf_int = _mm256_castps_si256(is_inf);

        // t = (depth - min_z) * scale
        let t_f32 = _mm256_mul_ps(_mm256_sub_ps(depth_val, min_z_vec), scale_vec);

        // t_u32 = t_f32 as i32
        let t_i32 = _mm256_cvttps_epi32(t_f32);

        // Ensure not negative
        let zero_vec = _mm256_setzero_si256();
        let t_clamped_low = _mm256_max_epi32(t_i32, zero_vec);

        // Clamp to 1023
        let t_clamped = _mm256_min_epi32(t_clamped_low, max_t_vec);

        // Gather from LUT
        // SAFETY: t_clamped is strictly between 0 and 1023, lut is 1024 elements
        let gathered = _mm256_i32gather_epi32::<4>(lut_ptr, t_clamped);

        // Blend: if is_inf, use bg_color, else use gathered color
        let final_color = _mm256_blendv_epi8(gathered, bg_color, is_inf_int);

        // Store to framebuffer
        #[allow(clippy::cast_ptr_alignment)]
        let fb_ptr = pixels.as_mut_ptr().add(i).cast::<__m256i>();
        _mm256_storeu_si256(fb_ptr, final_color);

        i += 8;
    }

    // Scalar tail
    for (pixel, &depth) in pixels[i..len].iter_mut().zip(depths[i..len].iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}"""

new_simd_gather = """
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        _mm256_cmpgt_epi32, _mm256_slli_epi32, _mm256_or_si256, _mm256_and_si256, _mm256_sub_epi32,
    };

    let min_z_vec = _mm256_set1_ps(min_z);
    let scale_vec = _mm256_set1_ps(scale);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let max_t_vec = _mm256_set1_epi32(1023);
    let bg_color = _mm256_set1_epi32(0xFF00_0010_u32 as i32);

    let c256_vec = _mm256_set1_epi32(256);
    let c512_vec = _mm256_set1_epi32(512);
    let c768_vec = _mm256_set1_epi32(768);
    let c255_vec = _mm256_set1_epi32(255);
    let alpha_vec = _mm256_set1_epi32(0xFF00_0000_u32 as i32);
    let zero_vec = _mm256_setzero_si256();

    while i + 8 <= len {
        let depth_ptr = depths.as_ptr().add(i);
        let depth_val = _mm256_loadu_ps(depth_ptr);

        let is_inf = _mm256_cmp_ps(depth_val, inf_vec, _CMP_EQ_OQ);
        let is_inf_int = _mm256_castps_si256(is_inf);

        let t_f32 = _mm256_mul_ps(_mm256_sub_ps(depth_val, min_z_vec), scale_vec);
        let t_i32 = _mm256_cvttps_epi32(t_f32);

        let t_clamped_low = _mm256_max_epi32(t_i32, zero_vec);
        let t = _mm256_min_epi32(t_clamped_low, max_t_vec);

        // Masks for each segment
        // t < 256
        let mask0 = _mm256_cmpgt_epi32(c256_vec, t);

        // 256 <= t < 512
        let mask1_ge = _mm256_cmpgt_epi32(t, _mm256_sub_epi32(c256_vec, _mm256_set1_epi32(1)));
        let mask1_lt = _mm256_cmpgt_epi32(c512_vec, t);
        let mask1 = _mm256_and_si256(mask1_ge, mask1_lt);

        // 512 <= t < 768
        let mask2_ge = _mm256_cmpgt_epi32(t, _mm256_sub_epi32(c512_vec, _mm256_set1_epi32(1)));
        let mask2_lt = _mm256_cmpgt_epi32(c768_vec, t);
        let mask2 = _mm256_and_si256(mask2_ge, mask2_lt);

        // t >= 768
        let mask3 = _mm256_cmpgt_epi32(t, _mm256_sub_epi32(c768_vec, _mm256_set1_epi32(1)));

        // Calculate RGB for Segment 0 (t < 256)
        // r = 255, g = t, b = 0
        let r0 = c255_vec;
        let g0 = t;
        let b0 = zero_vec;

        // Calculate RGB for Segment 1 (256 <= t < 512)
        // r = 255 - (t - 256), g = 255, b = 0
        let t_sub_256 = _mm256_sub_epi32(t, c256_vec);
        let r1 = _mm256_sub_epi32(c255_vec, t_sub_256);
        let g1 = c255_vec;
        let b1 = zero_vec;

        // Calculate RGB for Segment 2 (512 <= t < 768)
        // r = 0, g = 255, b = t - 512
        let r2 = zero_vec;
        let g2 = c255_vec;
        let b2 = _mm256_sub_epi32(t, c512_vec);

        // Calculate RGB for Segment 3 (t >= 768)
        // r = 0, g = 255 - (t - 768), b = 255
        let r3 = zero_vec;
        let t_sub_768 = _mm256_sub_epi32(t, c768_vec);
        let g3 = _mm256_sub_epi32(c255_vec, t_sub_768);
        let b3 = c255_vec;

        // Blend R
        let r_01 = _mm256_blendv_epi8(r1, r0, mask0);
        let r_23 = _mm256_blendv_epi8(r3, r2, mask2);
        let r = _mm256_blendv_epi8(r_23, r_01, _mm256_or_si256(mask0, mask1));

        // Blend G
        let g_01 = _mm256_blendv_epi8(g1, g0, mask0);
        let g_23 = _mm256_blendv_epi8(g3, g2, mask2);
        let g = _mm256_blendv_epi8(g_23, g_01, _mm256_or_si256(mask0, mask1));

        // Blend B
        let b_01 = _mm256_blendv_epi8(b1, b0, mask0);
        let b_23 = _mm256_blendv_epi8(b3, b2, mask2);
        let b = _mm256_blendv_epi8(b_23, b_01, _mm256_or_si256(mask0, mask1));

        // Pack to ARGB
        let r_shifted = _mm256_slli_epi32::<16>(r);
        let g_shifted = _mm256_slli_epi32::<8>(g);
        let rgb = _mm256_or_si256(_mm256_or_si256(r_shifted, g_shifted), b);
        let argb = _mm256_or_si256(alpha_vec, rgb);

        let final_color = _mm256_blendv_epi8(argb, bg_color, is_inf_int);

        #[allow(clippy::cast_ptr_alignment)]
        let fb_ptr = pixels.as_mut_ptr().add(i).cast::<__m256i>();
        _mm256_storeu_si256(fb_ptr, final_color);

        i += 8;
    }

    for (pixel, &depth) in pixels[i..len].iter_mut().zip(depths[i..len].iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        let (r, g, b) = if t < 256 {
            (255, t, 0)
        } else if t < 512 {
            (255 - (t - 256), 255, 0)
        } else if t < 768 {
            (0, 255, t - 512)
        } else {
            (0, 255 - (t - 768), 255)
        };

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }
}"""
content = content.replace(simd_gather, new_simd_gather)

with open("crates/abrash-render/src/heat_vision.rs", "w") as f:
    f.write(content)
