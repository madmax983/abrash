use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;
use std::cell::RefCell;

thread_local! {
    static SOBEL_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Applies a Sobel edge detection filter to the framebuffer in-place.
///
/// Detects edges by calculating the gradient magnitude of the image luminance.
/// The result is a grayscale image where brighter pixels represent stronger edges.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::sobel::apply_sobel;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// // Draw something...
/// apply_sobel(&mut fb);
/// ```
pub fn apply_sobel(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let needed_size = width * height;

    // Use SOBEL_BUFFER for luminance data
    // We need 32 bytes padding for SIMD later.
    let buffer_size = needed_size + 32;

    SOBEL_BUFFER.with(|buf| {
        let mut lum_buffer = buf.borrow_mut();
        if lum_buffer.len() < buffer_size {
            lum_buffer.resize(buffer_size, 0);
        }

        let lum_slice = &mut lum_buffer[..buffer_size]; // Allow access to padding

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if std::is_x86_feature_detected!("avx2") {
                unsafe { apply_sobel_avx2(pixels, lum_slice, width, height) };
                return;
            }
        }

        // 1. Convert to Luminance (Scalar)
        for (i, p) in pixels.iter().enumerate() {
            lum_slice[i] = pixel_luminance(*p);
        }

        // 2. Apply Sobel
        // We skip the 1-pixel border
        for y in 1..height - 1 {
            let row_offset = y * width;
            for x in 1..width - 1 {
                let idx = row_offset + x;

                // Neighborhood
                let tl = i32::from(lum_slice[idx - width - 1]);
                let t = i32::from(lum_slice[idx - width]);
                let tr = i32::from(lum_slice[idx - width + 1]);
                let l = i32::from(lum_slice[idx - 1]);
                let r = i32::from(lum_slice[idx + 1]);
                let bl = i32::from(lum_slice[idx + width - 1]);
                let b = i32::from(lum_slice[idx + width]);
                let br = i32::from(lum_slice[idx + width + 1]);

                // Gx Kernel
                let gx = (tr + 2 * r + br) - (tl + 2 * l + bl);

                // Gy Kernel
                let gy = (bl + 2 * b + br) - (tl + 2 * t + tr);

                // Magnitude
                let mag = (gx.abs() + gy.abs()).min(255) as u32;

                // Write back (Gray + Alpha)
                let original_alpha = pixels[idx] & 0xFF00_0000;
                pixels[idx] = original_alpha | (mag << 16) | (mag << 8) | mag;
            }
        }

        // Zero out borders
        for x in 0..width {
            pixels[x] &= 0xFF00_0000;
            pixels[(height - 1) * width + x] &= 0xFF00_0000;
        }
        for y in 0..height {
            pixels[y * width] &= 0xFF00_0000;
            pixels[y * width + width - 1] &= 0xFF00_0000;
        }
    });
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_sobel_avx2(pixels: &mut [u32], lum_buffer: &mut [u8], width: usize, height: usize) {
    use std::arch::x86_64::*;

    unsafe {
    // 1. RGB -> Luminance
    {
        let len = width * height;
        let mut s_ptr = pixels.as_ptr();
        let mut d_ptr = lum_buffer.as_mut_ptr();

        let weights = _mm256_set1_epi64x(0x0000_004D_0096_001D);
        let perm_mask = _mm256_setr_epi32(0, 4, 1, 5, 2, 6, 3, 7);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        let mut i = 0;
        while i + 32 <= len {
            // Unroll 4x
            let p0 = _mm256_loadu_si256(s_ptr.add(i).cast());
            let p1 = _mm256_loadu_si256(s_ptr.add(i + 8).cast());
            let p2 = _mm256_loadu_si256(s_ptr.add(i + 16).cast());
            let p3 = _mm256_loadu_si256(s_ptr.add(i + 24).cast());

            // Helper closure for luma calc (returns 8 i32s)
            // Can't use closure with target_feature in unsafe fn easily in Rust versions
            // So inline it or use macro. Inlining manually.

            // Luma 0
            let l0 = {
                let lo_128 = _mm256_castsi256_si128(p0);
                let hi_128 = _mm256_extracti128_si256(p0, 1);
                let v_lo = _mm256_cvtepu8_epi16(lo_128);
                let v_hi = _mm256_cvtepu8_epi16(hi_128);
                let prod_lo = _mm256_madd_epi16(v_lo, weights);
                let prod_hi = _mm256_madd_epi16(v_hi, weights);
                let sums = _mm256_hadd_epi32(prod_lo, prod_hi);
                let scrambled = _mm256_permute4x64_epi64(sums, 0xD8);
                _mm256_srai_epi32(scrambled, 8)
            };

            // Luma 1
            let l1 = {
                let lo_128 = _mm256_castsi256_si128(p1);
                let hi_128 = _mm256_extracti128_si256(p1, 1);
                let v_lo = _mm256_cvtepu8_epi16(lo_128);
                let v_hi = _mm256_cvtepu8_epi16(hi_128);
                let prod_lo = _mm256_madd_epi16(v_lo, weights);
                let prod_hi = _mm256_madd_epi16(v_hi, weights);
                let sums = _mm256_hadd_epi32(prod_lo, prod_hi);
                let scrambled = _mm256_permute4x64_epi64(sums, 0xD8);
                _mm256_srai_epi32(scrambled, 8)
            };

            // Luma 2
            let l2 = {
                let lo_128 = _mm256_castsi256_si128(p2);
                let hi_128 = _mm256_extracti128_si256(p2, 1);
                let v_lo = _mm256_cvtepu8_epi16(lo_128);
                let v_hi = _mm256_cvtepu8_epi16(hi_128);
                let prod_lo = _mm256_madd_epi16(v_lo, weights);
                let prod_hi = _mm256_madd_epi16(v_hi, weights);
                let sums = _mm256_hadd_epi32(prod_lo, prod_hi);
                let scrambled = _mm256_permute4x64_epi64(sums, 0xD8);
                _mm256_srai_epi32(scrambled, 8)
            };

            // Luma 3
            let l3 = {
                let lo_128 = _mm256_castsi256_si128(p3);
                let hi_128 = _mm256_extracti128_si256(p3, 1);
                let v_lo = _mm256_cvtepu8_epi16(lo_128);
                let v_hi = _mm256_cvtepu8_epi16(hi_128);
                let prod_lo = _mm256_madd_epi16(v_lo, weights);
                let prod_hi = _mm256_madd_epi16(v_hi, weights);
                let sums = _mm256_hadd_epi32(prod_lo, prod_hi);
                let scrambled = _mm256_permute4x64_epi64(sums, 0xD8);
                _mm256_srai_epi32(scrambled, 8)
            };

            let l01_16 = _mm256_packus_epi32(l0, l1);
            let l23_16 = _mm256_packus_epi32(l2, l3);
            let packed = _mm256_packus_epi16(l01_16, l23_16);
            let final_u8 = _mm256_permutevar8x32_epi32(packed, perm_mask);

            _mm256_storeu_si256(d_ptr.add(i).cast(), final_u8);
            i += 32;
        }

        // Tail
        for j in i..len {
            *d_ptr.add(j) = pixel_luminance(*s_ptr.add(j));
        }
    }

    // 2. Apply Sobel
    {
        // Vectors for weighting
        let two = _mm256_set1_epi16(2);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        for y in 1..height - 1 {
            let top_offset = (y - 1) * width;
            let mid_offset = y * width;
            let bot_offset = (y + 1) * width;

            let mut x = 1;
            while x + 16 < width - 1 {
                // Load neighborhood (16 pixels)
                // We need TL, T, TR etc.
                // TL starts at x-1. T starts at x. TR starts at x+1.
                // We load 16 bytes at once from x-1, x, x+1?
                // Actually, just load u8s and convert to i16.

                // Helper to load 16 bytes and convert to 16 i16s
                let load_i16 = |ptr: *const u8, offset: usize| {
                    let v8 = _mm_loadu_si128(ptr.add(offset).cast());
                    _mm256_cvtepu8_epi16(v8)
                };

                let ptr = lum_buffer.as_ptr();

                let tl = load_i16(ptr, top_offset + x - 1);
                let t = load_i16(ptr, top_offset + x);
                let tr = load_i16(ptr, top_offset + x + 1);

                let l = load_i16(ptr, mid_offset + x - 1);
                // let c  = load_i16(ptr, mid_offset + x); // Center unused
                let r = load_i16(ptr, mid_offset + x + 1);

                let bl = load_i16(ptr, bot_offset + x - 1);
                let b = load_i16(ptr, bot_offset + x);
                let br = load_i16(ptr, bot_offset + x + 1);

                // Gx = (TR + 2*R + BR) - (TL + 2*L + BL)
                let right_part =
                    _mm256_add_epi16(_mm256_add_epi16(tr, br), _mm256_mullo_epi16(r, two));
                let left_part =
                    _mm256_add_epi16(_mm256_add_epi16(tl, bl), _mm256_mullo_epi16(l, two));
                let gx = _mm256_sub_epi16(right_part, left_part);

                // Gy = (BL + 2*B + BR) - (TL + 2*T + TR)
                let bot_part =
                    _mm256_add_epi16(_mm256_add_epi16(bl, br), _mm256_mullo_epi16(b, two));
                let top_part =
                    _mm256_add_epi16(_mm256_add_epi16(tl, tr), _mm256_mullo_epi16(t, two));
                let gy = _mm256_sub_epi16(bot_part, top_part);

                // Magnitude = abs(Gx) + abs(Gy)
                let gx_abs = _mm256_abs_epi16(gx);
                let gy_abs = _mm256_abs_epi16(gy);
                let mag = _mm256_add_epi16(gx_abs, gy_abs); // i16
                // Saturating pack to u8
                // packus_epi16 packs 2 256-bit vecs to 1 256-bit vec.
                // We have 1 256-bit vec (mag).
                // We can use zero for the second argument?
                // result = packus(mag, zero).
                // Output: [mag_lo, zero_lo, mag_hi, zero_hi] (128-bit lanes).
                // We need to permute to get [mag_lo, mag_hi, zero_lo, zero_hi].
                let zero = _mm256_setzero_si256();
                let packed = _mm256_packus_epi16(mag, zero);
                let perm = _mm256_permute4x64_epi64(packed, 0xD8);
                // Now first 128 bits (16 bytes) are our result.
                let mag_u8 = _mm256_castsi256_si128(perm);

                // Expand to u32 pixels
                // mag_u8 contains 16 magnitude values.
                // We need to expand to 16 u32 pixels.
                // 16 pixels = 2 YMM registers (8 each).

                // Low 8 bytes -> YMM 0
                let mag_lo = _mm256_cvtepu8_epi32(mag_u8);
                // High 8 bytes -> YMM 1
                let mag_hi = _mm256_cvtepu8_epi32(_mm_srli_si128(mag_u8, 8));

                // Prepare pixels: | M | M | M | A | ? No A | M | M | M
                // 0xFF00_0000 | (mag << 16) | (mag << 8) | mag
                // Construct (mag << 16) | (mag << 8) | mag
                let m_sh16 = _mm256_slli_epi32(mag_lo, 16);
                let m_sh8 = _mm256_slli_epi32(mag_lo, 8);
                let gray_lo = _mm256_or_si256(mag_lo, _mm256_or_si256(m_sh16, m_sh8));

                let m_sh16_hi = _mm256_slli_epi32(mag_hi, 16);
                let m_sh8_hi = _mm256_slli_epi32(mag_hi, 8);
                let gray_hi = _mm256_or_si256(mag_hi, _mm256_or_si256(m_sh16_hi, m_sh8_hi));

                // Load original alphas
                let dest_ptr = pixels.as_mut_ptr().add(mid_offset + x);
                let orig_lo = _mm256_loadu_si256(dest_ptr.cast());
                let orig_hi = _mm256_loadu_si256(dest_ptr.add(8).cast());

                let alpha_lo = _mm256_and_si256(orig_lo, alpha_mask);
                let alpha_hi = _mm256_and_si256(orig_hi, alpha_mask);

                let final_lo = _mm256_or_si256(gray_lo, alpha_lo);
                let final_hi = _mm256_or_si256(gray_hi, alpha_hi);

                _mm256_storeu_si256(dest_ptr.cast(), final_lo);
                _mm256_storeu_si256(dest_ptr.add(8).cast(), final_hi);

                x += 16;
            }

            // Tail
            for cx in x..width - 1 {
                let idx = mid_offset + cx;
                let tl = i32::from(lum_buffer[top_offset + cx - 1]);
                let t = i32::from(lum_buffer[top_offset + cx]);
                let tr = i32::from(lum_buffer[top_offset + cx + 1]);
                let l = i32::from(lum_buffer[mid_offset + cx - 1]);
                let r = i32::from(lum_buffer[mid_offset + cx + 1]);
                let bl = i32::from(lum_buffer[bot_offset + cx - 1]);
                let b = i32::from(lum_buffer[bot_offset + cx]);
                let br = i32::from(lum_buffer[bot_offset + cx + 1]);

                let gx = (tr + 2 * r + br) - (tl + 2 * l + bl);
                let gy = (bl + 2 * b + br) - (tl + 2 * t + tr);
                let mag = (gx.abs() + gy.abs()).min(255) as u32;

                let original_alpha = pixels[idx] & 0xFF00_0000;
                pixels[idx] = original_alpha | (mag << 16) | (mag << 8) | mag;
            }
        }
    }
    }
}
