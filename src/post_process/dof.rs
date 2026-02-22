use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;
use super::blur::{box_blur_horizontal, box_blur_vertical};

thread_local! {
    static DOF_CONTEXT: RefCell<DofContext> = RefCell::new(DofContext::default());
}

struct DofContext {
    blurred_buffer: Vec<u32>,
    scratch_buffer: Vec<u32>,
    acc_buffer: Vec<i32>,
}

impl Default for DofContext {
    fn default() -> Self {
        Self {
            blurred_buffer: Vec::new(),
            scratch_buffer: Vec::new(),
            acc_buffer: Vec::new(),
        }
    }
}

/// Applies depth of field effect.
///
/// # Arguments
/// * `fb` - The framebuffer (modified in-place).
/// * `zb` - The depth buffer.
/// * `focus_dist` - The depth at which objects are perfectly in focus (0.0 - 1.0 in non-linear z-buffer space).
/// * `focus_range` - The range of depth that remains reasonably sharp.
/// * `blur_radius` - The radius of the blur for out-of-focus areas.
pub fn apply_depth_of_field(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    focus_dist: f32,
    focus_range: f32,
    blur_radius: u32,
) {
    if blur_radius == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let needed_size = width * height;
    let acc_needed_size = width * 3;

    DOF_CONTEXT.with(|ctx_ref| {
        let mut ctx = ctx_ref.borrow_mut();

        if ctx.blurred_buffer.len() < needed_size {
            ctx.blurred_buffer.resize(needed_size, 0);
        }
        if ctx.scratch_buffer.len() < needed_size {
            ctx.scratch_buffer.resize(needed_size, 0);
        }
        if ctx.acc_buffer.len() < acc_needed_size {
            ctx.acc_buffer.resize(acc_needed_size, 0);
        }

        let DofContext {
            blurred_buffer,
            scratch_buffer,
            acc_buffer,
        } = &mut *ctx;

        let blurred_slice = &mut blurred_buffer[..needed_size];
        let scratch_slice = &mut scratch_buffer[..needed_size];
        let acc_slice = &mut acc_buffer[..acc_needed_size];
        let original_pixels = fb.as_mut_slice();

        // 1. Create blurred copy
        // Horizontal pass: original -> scratch
        box_blur_horizontal(original_pixels, scratch_slice, width, height, blur_radius);
        // Vertical pass: scratch -> blurred
        box_blur_vertical(scratch_slice, blurred_slice, acc_slice, width, height, blur_radius);

        // 2. Blend based on depth
        // We iterate over the original buffer and the blurred buffer
        let zb_slice = zb.as_slice();

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if std::is_x86_feature_detected!("avx2") {
                unsafe {
                    apply_dof_avx2(
                        original_pixels,
                        blurred_slice,
                        zb_slice,
                        focus_dist,
                        focus_range,
                    );
                }
                return;
            }
        }

        apply_dof_scalar(
            original_pixels,
            blurred_slice,
            zb_slice,
            focus_dist,
            focus_range,
        );
    });
}

fn apply_dof_scalar(
    original_pixels: &mut [u32],
    blurred_pixels: &[u32],
    depths: &[f32],
    focus_dist: f32,
    focus_range: f32,
) {
    let len = original_pixels
        .len()
        .min(depths.len())
        .min(blurred_pixels.len());

    for i in 0..len {
        let depth = depths[i];

        // Skip infinite depth (skybox) if desired, or treat as far.
        let z = if depth.is_infinite() { 1000.0 } else { depth };

        let dist = (z - focus_dist).abs();

        // Calculate blur factor (0.0 = sharp, 1.0 = full blur)
        let factor = ((dist - focus_range) / focus_range).clamp(0.0, 1.0);

        if factor > 0.0 {
            let orig = original_pixels[i];
            let blur = blurred_pixels[i];

            let r_o = ((orig >> 16) & 0xFF) as f32;
            let g_o = ((orig >> 8) & 0xFF) as f32;
            let b_o = (orig & 0xFF) as f32;

            let r_b = ((blur >> 16) & 0xFF) as f32;
            let g_b = ((blur >> 8) & 0xFF) as f32;
            let b_b = (blur & 0xFF) as f32;

            let r_new = lerp(r_o, r_b, factor) as u32;
            let g_new = lerp(g_o, g_b, factor) as u32;
            let b_new = lerp(b_o, b_b, factor) as u32;

            // Preserve alpha
            original_pixels[i] = (orig & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;
        }
    }
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_dof_avx2(
    original_pixels: &mut [u32],
    blurred_pixels: &[u32],
    depths: &[f32],
    focus_dist: f32,
    focus_range: f32,
) {
    use std::arch::x86_64::*;

    let len = original_pixels
        .len()
        .min(depths.len())
        .min(blurred_pixels.len());
    let mut i = 0;

    // SAFETY: This function is unsafe because it calls unsafe AVX2 intrinsics.
    // The caller must ensure that the CPU supports AVX2.
    unsafe {
        // Constants
        let focus_dist_vec = _mm256_set1_ps(focus_dist);
        let focus_range_vec = _mm256_set1_ps(focus_range);
        let one = _mm256_set1_ps(1.0);
        let zero = _mm256_setzero_ps();
        let minus_zero = _mm256_set1_ps(-0.0);
        let inv_range = _mm256_div_ps(one, focus_range_vec);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
        let infinity = f32::INFINITY;
        let thousand = _mm256_set1_ps(1000.0);

        let inf_vec = _mm256_set1_ps(infinity);
        let mask_ff = _mm256_set1_epi32(0xFF);

        while i + 8 <= len {
            // Load depths
            let d = _mm256_loadu_ps(depths.as_ptr().add(i));

            // Handle infinity: if d == inf, use 1000.0
            let mask_inf = _mm256_cmp_ps(d, inf_vec, _CMP_EQ_OQ);
            let z = _mm256_blendv_ps(d, thousand, mask_inf);

            // dist = abs(z - focus_dist)
            let diff = _mm256_sub_ps(z, focus_dist_vec);
            let dist = _mm256_andnot_ps(minus_zero, diff); // abs

            // factor = ((dist - focus_range) / focus_range).clamp(0.0, 1.0)
            let f1 = _mm256_sub_ps(dist, focus_range_vec);
            let f2 = _mm256_mul_ps(f1, inv_range);
            let f3 = _mm256_max_ps(f2, zero);
            let factor = _mm256_min_ps(f3, one);

            // Check if ANY factor > 0
            let mask_gt0 = _mm256_cmp_ps(factor, zero, _CMP_GT_OQ);
            if _mm256_movemask_ps(mask_gt0) == 0 {
                i += 8;
                continue;
            }

            // Load pixels
            let orig_i = _mm256_loadu_si256(original_pixels.as_ptr().add(i).cast());
            let blur_i = _mm256_loadu_si256(blurred_pixels.as_ptr().add(i).cast());

            // Unpack u32 pixels to R, G, B planes (8 elements each)
            // Note: _mm256_srli_epi32 shifts each 32-bit element right
            let b_o_i = _mm256_and_si256(orig_i, mask_ff);
            let g_o_i = _mm256_and_si256(_mm256_srli_epi32(orig_i, 8), mask_ff);
            let r_o_i = _mm256_and_si256(_mm256_srli_epi32(orig_i, 16), mask_ff);

            let b_b_i = _mm256_and_si256(blur_i, mask_ff);
            let g_b_i = _mm256_and_si256(_mm256_srli_epi32(blur_i, 8), mask_ff);
            let r_b_i = _mm256_and_si256(_mm256_srli_epi32(blur_i, 16), mask_ff);

            let b_o_f = _mm256_cvtepi32_ps(b_o_i);
            let g_o_f = _mm256_cvtepi32_ps(g_o_i);
            let r_o_f = _mm256_cvtepi32_ps(r_o_i);

            let b_b_f = _mm256_cvtepi32_ps(b_b_i);
            let g_b_f = _mm256_cvtepi32_ps(g_b_i);
            let r_b_f = _mm256_cvtepi32_ps(r_b_i);

            // Lerp: a + (b - a) * t
            let r_diff = _mm256_sub_ps(r_b_f, r_o_f);
            let g_diff = _mm256_sub_ps(g_b_f, g_o_f);
            let b_diff = _mm256_sub_ps(b_b_f, b_o_f);

            // a + diff * factor
            let r_new_f = _mm256_add_ps(r_o_f, _mm256_mul_ps(r_diff, factor));
            let g_new_f = _mm256_add_ps(g_o_f, _mm256_mul_ps(g_diff, factor));
            let b_new_f = _mm256_add_ps(b_o_f, _mm256_mul_ps(b_diff, factor));

            // Convert back to i32 (truncate)
            let r_new_i = _mm256_cvttps_epi32(r_new_f);
            let g_new_i = _mm256_cvttps_epi32(g_new_f);
            let b_new_i = _mm256_cvttps_epi32(b_new_f);

            // Pack
            let r_sh = _mm256_slli_epi32(r_new_i, 16);
            let g_sh = _mm256_slli_epi32(g_new_i, 8);

            // Preserve Alpha
            let a_o_i = _mm256_and_si256(orig_i, alpha_mask);

            let result = _mm256_or_si256(
                a_o_i,
                _mm256_or_si256(r_sh, _mm256_or_si256(g_sh, b_new_i)),
            );

            _mm256_storeu_si256(original_pixels.as_mut_ptr().add(i).cast(), result);

            i += 8;
        }
    }

    // Tail loop (scalar)
    while i < len {
        let depth = depths[i];
        let z = if depth.is_infinite() { 1000.0 } else { depth };
        let dist = (z - focus_dist).abs();
        let factor = ((dist - focus_range) / focus_range).clamp(0.0, 1.0);

        if factor > 0.0 {
            let orig = original_pixels[i];
            let blur = blurred_pixels[i];

            let r_o = ((orig >> 16) & 0xFF) as f32;
            let g_o = ((orig >> 8) & 0xFF) as f32;
            let b_o = (orig & 0xFF) as f32;

            let r_b = ((blur >> 16) & 0xFF) as f32;
            let g_b = ((blur >> 8) & 0xFF) as f32;
            let b_b = (blur & 0xFF) as f32;

            let r_new = lerp(r_o, r_b, factor) as u32;
            let g_new = lerp(g_o, g_b, factor) as u32;
            let b_new = lerp(b_o, b_b, factor) as u32;

            original_pixels[i] = (orig & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_dof_changes_pixels() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Fill with white
        fb.clear(0xFFFFFFFF);

        // Fill ZBuffer:
        // Left half: depth 1.0 (focus)
        // Right half: depth 10.0 (out of focus)
        for y in 0..height {
            for x in 0..width {
                let depth = if x < width / 2 { 1.0 } else { 10.0 };
                zb.test_and_set(x as i32, y as i32, depth);
            }
        }

        // Apply pattern to FB to see blur
        // Checkerboard
        for y in 0..height {
            for x in 0..width {
                if (x as u32 + y as u32) % 2 == 0 {
                    fb.set_pixel(x as i32, y as i32, 0xFF000000);
                }
            }
        }

        let original_pixel = fb.get_pixel(width as i32 - 1, height as i32 - 1).unwrap();

        // Apply DoF
        // Focus at 1.0, range 1.0. Right half should blur.
        apply_depth_of_field(&mut fb, &zb, 1.0, 1.0, 2);

        // Check if out-of-focus pixel changed
        let new_pixel = fb.get_pixel(width as i32 - 1, height as i32 - 1).unwrap();

        assert_ne!(original_pixel, new_pixel, "Out of focus pixel should be modified by blur");

        // Check if in-focus pixel is UNCHANGED (or minimally changed)
        let focus_pixel_orig = 0xFF000000; // (0,0) is black
        let focus_pixel_new = fb.get_pixel(0, 0).unwrap();
        // Since factor should be 0.0 for dist=0, it should be exact.
        assert_eq!(focus_pixel_orig, focus_pixel_new, "In focus pixel should not change");
    }

    #[test]
    fn test_apply_dof_simd_vs_scalar() {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if !std::is_x86_feature_detected!("avx2") {
                println!("Skipping AVX2 test on non-AVX2 hardware");
                return;
            }

            let len = 100; // Includes tail
            let mut original_scalar = vec![0xFFFFFFFF; len];
            let mut original_simd = vec![0xFFFFFFFF; len];

            let blurred = vec![0xFF000000; len]; // Black blurred pixels

            // Generate depths
            let mut depths = vec![0.0; len];
            for i in 0..len {
                depths[i] = i as f32 * 0.1; // 0.0, 0.1, ...
            }

            let focus_dist = 2.0;
            let focus_range = 1.0;

            apply_dof_scalar(
                &mut original_scalar,
                &blurred,
                &depths,
                focus_dist,
                focus_range,
            );

            unsafe {
                apply_dof_avx2(
                    &mut original_simd,
                    &blurred,
                    &depths,
                    focus_dist,
                    focus_range,
                );
            }

            for i in 0..len {
                let s = original_scalar[i];
                let v = original_simd[i];

                let s_r = (s >> 16) & 0xFF;
                let v_r = (v >> 16) & 0xFF;
                let s_g = (s >> 8) & 0xFF;
                let v_g = (v >> 8) & 0xFF;
                let s_b = s & 0xFF;
                let v_b = v & 0xFF;

                // Allow +/- 1 difference due to rounding
                assert!(
                    (s_r as i32 - v_r as i32).abs() <= 1,
                    "Pixel {} Red mismatch: Scalar {:X} vs SIMD {:X}",
                    i,
                    s,
                    v
                );
                assert!(
                    (s_g as i32 - v_g as i32).abs() <= 1,
                    "Pixel {} Green mismatch: Scalar {:X} vs SIMD {:X}",
                    i,
                    s,
                    v
                );
                assert!(
                    (s_b as i32 - v_b as i32).abs() <= 1,
                    "Pixel {} Blue mismatch: Scalar {:X} vs SIMD {:X}",
                    i,
                    s,
                    v
                );
            }
        }
    }
}
