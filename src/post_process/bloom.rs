use crate::framebuffer::Framebuffer;
use crate::post_process::blur::{box_blur_horizontal, box_blur_vertical};
use crate::utils::pixel_luminance;
use std::cell::RefCell;

thread_local! {
    static BLOOM_BUFFERS: RefCell<BloomContext> = RefCell::new(BloomContext::default());
}

struct BloomContext {
    bright_pixels: Vec<u32>,
    scratch_buffer: Vec<u32>,
    acc_buffer: Vec<i32>,
}

impl Default for BloomContext {
    fn default() -> Self {
        Self {
            bright_pixels: Vec::new(),
            scratch_buffer: Vec::new(),
            acc_buffer: Vec::new(),
        }
    }
}

/// Applies a bloom effect to the framebuffer in-place.
///
/// Bloom creates a glow effect around bright areas of the image.
///
/// # Arguments
///
/// *   `fb` - The framebuffer to apply the effect to.
/// *   `threshold` - Minimum luminance (0-255) for a pixel to contribute to bloom.
/// *   `blur_radius` - Radius of the box blur kernel.
/// *   `intensity` - Multiplier for the bloom intensity.
pub fn apply_bloom(fb: &mut Framebuffer, threshold: u8, blur_radius: u32, intensity: f32) {
    if blur_radius == 0 || intensity <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let needed_size = width * height;
    let acc_needed_size = width * 3;

    BLOOM_BUFFERS.with(|buffers| {
        let mut ctx = buffers.borrow_mut();

        // Ensure buffers are large enough
        if ctx.bright_pixels.len() < needed_size {
            ctx.bright_pixels.resize(needed_size, 0);
        }
        if ctx.scratch_buffer.len() < needed_size {
            ctx.scratch_buffer.resize(needed_size, 0);
        }
        if ctx.acc_buffer.len() < acc_needed_size {
            ctx.acc_buffer.resize(acc_needed_size, 0);
        }

        let BloomContext {
            bright_pixels,
            scratch_buffer,
            acc_buffer,
        } = &mut *ctx;

        let bright_slice = &mut bright_pixels[..needed_size];
        let scratch_slice = &mut scratch_buffer[..needed_size];
        let acc_slice = &mut acc_buffer[..acc_needed_size];

        // 1. Extract bright pixels
        extract_bright_pixels(pixels, bright_slice, threshold);

        // 2. Blur the bright pixels
        // Horizontal pass: bright_pixels -> scratch_buffer
        box_blur_horizontal(bright_slice, scratch_slice, width, height, blur_radius);
        // Vertical pass: scratch_buffer -> bright_pixels
        box_blur_vertical(
            scratch_slice,
            bright_slice,
            acc_slice,
            width,
            height,
            blur_radius,
        );

        // 3. Composite back
        blend_additive(pixels, bright_slice, intensity);
    });
}

fn extract_bright_pixels(src: &[u32], dest: &mut [u32], threshold: u8) {
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            let len = src.len();
            let simd_len = len & !7;
            unsafe {
                extract_bright_pixels_avx2(&src[..simd_len], &mut dest[..simd_len], threshold);
            }
            // Tail
            for (s, d) in src[simd_len..].iter().zip(dest[simd_len..].iter_mut()) {
                let lum = pixel_luminance(*s);
                if lum > threshold {
                    *d = *s;
                } else {
                    *d = 0xFF00_0000;
                }
            }
            return;
        }
    }

    for (s, d) in src.iter().zip(dest.iter_mut()) {
        let lum = pixel_luminance(*s);
        if lum > threshold {
            *d = *s;
        } else {
            *d = 0xFF00_0000; // Black (with full alpha)
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn extract_bright_pixels_avx2(src: &[u32], dest: &mut [u32], threshold: u8) {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_blendv_epi8, _mm256_castsi256_si128, _mm256_cmpgt_epi32,
        _mm256_cvtepu8_epi16, _mm256_extracti128_si256, _mm256_hadd_epi32, _mm256_loadu_si256,
        _mm256_madd_epi16, _mm256_permute4x64_epi64, _mm256_set1_epi32, _mm256_set1_epi64x,
        _mm256_srai_epi32, _mm256_storeu_si256,
    };

    unsafe {
        // Reuse luminance weights from grayscale
        // W0=29(B), W1=150(G), W2=77(R), W3=0(A)
        let weights = _mm256_set1_epi64x(0x0000_004D_0096_001D);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
        let black_pixel = _mm256_set1_epi32(0xFF00_0000u32 as i32);
        let threshold_vec = _mm256_set1_epi32(i32::from(threshold));

        let len = src.len();
        let mut s_ptr = src.as_ptr();
        let mut d_ptr = dest.as_mut_ptr();
        let end_ptr = s_ptr.add(len);

        while s_ptr < end_ptr {
            let chunk = _mm256_loadu_si256(s_ptr.cast());

            // 1. Calculate Luminance (same as grayscale)
            let _alphas = _mm256_and_si256(chunk, alpha_mask);
            let lo_128 = _mm256_castsi256_si128(chunk);
            let hi_128 = _mm256_extracti128_si256(chunk, 1);
            let v_lo = _mm256_cvtepu8_epi16(lo_128);
            let v_hi = _mm256_cvtepu8_epi16(hi_128);

            let prod_lo = _mm256_madd_epi16(v_lo, weights);
            let prod_hi = _mm256_madd_epi16(v_hi, weights);

            let sums_scrambled = _mm256_hadd_epi32(prod_lo, prod_hi);
            let sums = _mm256_permute4x64_epi64(sums_scrambled, 0xD8);
            let luma = _mm256_srai_epi32(sums, 8); // Luminance 0..255

            // 2. Threshold
            // Compare > threshold. _mm256_cmpgt_epi32 returns 0xFFFFFFFF if true, 0 if false.
            let mask = _mm256_cmpgt_epi32(luma, threshold_vec);

            // 3. Select
            // If > threshold, keep original chunk. Else black.
            // blendv_epi8 selects second arg (chunk) if mask MSB is 1.
            // Wait, mask is 32-bit. blendv_epi8 works on bytes.
            // Since mask is all 1s or all 0s per 32-bit lane, it works for blendv_epi8 too.
            let result = _mm256_blendv_epi8(black_pixel, chunk, mask);

            _mm256_storeu_si256(d_ptr.cast(), result);
            s_ptr = s_ptr.add(8);
            d_ptr = d_ptr.add(8);
        }
    }
}

fn blend_additive(dest: &mut [u32], src: &[u32], intensity: f32) {
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            let len = dest.len();
            let simd_len = len & !7;
            unsafe {
                blend_additive_avx2(&mut dest[..simd_len], &src[..simd_len], intensity);
            }
            // Tail
            let intensity_scale = (intensity * 256.0) as u32;
            for (d, s) in dest[simd_len..].iter_mut().zip(src[simd_len..].iter()) {
                let d_val = *d;
                let s_val = *s;
                let r_d = (d_val >> 16) & 0xFF;
                let g_d = (d_val >> 8) & 0xFF;
                let b_d = d_val & 0xFF;
                let r_s = (s_val >> 16) & 0xFF;
                let g_s = (s_val >> 8) & 0xFF;
                let b_s = s_val & 0xFF;
                let r_new = (r_d + ((r_s * intensity_scale) >> 8)).min(255);
                let g_new = (g_d + ((g_s * intensity_scale) >> 8)).min(255);
                let b_new = (b_d + ((b_s * intensity_scale) >> 8)).min(255);
                *d = (d_val & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;
            }
            return;
        }
    }

    // Convert intensity to fixed point 8.8 (approx) or just use floats for now.
    // For Green phase, correctness first.
    let intensity_scale = (intensity * 256.0) as u32;

    for (d, s) in dest.iter_mut().zip(src.iter()) {
        let d_val = *d;
        let s_val = *s;

        let r_d = (d_val >> 16) & 0xFF;
        let g_d = (d_val >> 8) & 0xFF;
        let b_d = d_val & 0xFF;

        let r_s = (s_val >> 16) & 0xFF;
        let g_s = (s_val >> 8) & 0xFF;
        let b_s = s_val & 0xFF;

        // Additive blend: dest + src * intensity
        let r_new = (r_d + ((r_s * intensity_scale) >> 8)).min(255);
        let g_new = (g_d + ((g_s * intensity_scale) >> 8)).min(255);
        let b_new = (b_d + ((b_s * intensity_scale) >> 8)).min(255);

        *d = (d_val & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn blend_additive_avx2(dest: &mut [u32], src: &[u32], intensity: f32) {
    use std::arch::x86_64::{
        _mm256_add_epi16, _mm256_and_si256, _mm256_castsi256_si128, _mm256_cvtepu8_epi16,
        _mm256_extracti128_si256, _mm256_loadu_si256, _mm256_mullo_epi16, _mm256_or_si256,
        _mm256_packus_epi16, _mm256_permute4x64_epi64, _mm256_set1_epi16, _mm256_set1_epi32,
        _mm256_srai_epi16, _mm256_storeu_si256,
    };

    unsafe {
        let scale = (intensity * 256.0) as i16;
        let scale_vec = _mm256_set1_epi16(scale);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        let len = dest.len();
        let mut d_ptr = dest.as_mut_ptr();
        let mut s_ptr = src.as_ptr();
        let end_ptr = d_ptr.add(len);

        while d_ptr < end_ptr {
            let s_chunk = _mm256_loadu_si256(s_ptr.cast());
            let d_chunk = _mm256_loadu_si256(d_ptr.cast());

            // Preserve dest alpha
            let d_alpha = _mm256_and_si256(d_chunk, alpha_mask);

            // Unpack to 16-bit
            let s_lo = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(s_chunk));
            let s_hi = _mm256_cvtepu8_epi16(_mm256_extracti128_si256(s_chunk, 1));

            let d_lo = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(d_chunk));
            let d_hi = _mm256_cvtepu8_epi16(_mm256_extracti128_si256(d_chunk, 1));

            // Multiply src * scale
            let s_lo_scaled = _mm256_mullo_epi16(s_lo, scale_vec);
            let s_hi_scaled = _mm256_mullo_epi16(s_hi, scale_vec);

            // Divide by 256
            let s_lo_final = _mm256_srai_epi16(s_lo_scaled, 8);
            let s_hi_final = _mm256_srai_epi16(s_hi_scaled, 8);

            // Add dest
            let res_lo = _mm256_add_epi16(d_lo, s_lo_final);
            let res_hi = _mm256_add_epi16(d_hi, s_hi_final);

            // Pack back to u8 (saturates)
            // packus_epi16 packs 16-bit signed integers to 8-bit unsigned integers with saturation.
            // It packs [a_lo, a_hi] -> [a_packed]
            // result layout: [a_lo_packed, b_lo_packed, a_hi_packed, b_hi_packed]
            // So we need to be careful with lane ordering.
            // _mm256_packus_epi16(a, b) packs a and b.
            // We have res_lo (first 4 pixels expanded) and res_hi (next 4 pixels expanded).
            let packed = _mm256_packus_epi16(res_lo, res_hi);

            // Fix lane ordering:
            // packus output: [res_lo_0..7, res_hi_0..7, res_lo_8..15, res_hi_8..15]
            // But res_lo is 256-bit containing 8 pixels worth of u16 data? No.
            // res_lo was derived from cvtepu8_epi16(lo_128). So it contains 8 pixels expanded to 16-bit.
            // Wait, 128 bits of u8 = 16 pixels.
            // cvtepu8_epi16 takes __m128i. It unpacks low 8 bytes (64 bits) to 8 words (128 bits)?
            // No, it converts 8 packed 8-bit integers to 8 packed 16-bit integers.
            // _mm256_cvtepu8_epi16 takes __m128i (16 bytes). It converts lower 8 bytes? Or all 16?
            // Documentation: "Zero extend packed unsigned 8-bit integers in a to packed 16-bit integers".
            // Output is __m256i (16 words = 32 bytes). Input is __m128i (16 bytes).
            // It converts ALL 16 bytes of input to 16 words?
            // Yes: 16 * 8 bits = 128 bits input. 16 * 16 bits = 256 bits output.
            // So `_mm256_cvtepu8_epi16` converts 16 pixels at once?
            // Let's re-read carefully.
            // "Zero extend the lower 8 packed 8-bit integers in a to packed 16-bit integers..." NO.
            // `_mm256_cvtepu8_epi16` (AVX2): "Zero extend packed unsigned 8-bit integers in a to packed 16-bit integers... and store in dst".
            // Input `__m128i` has 16 bytes. Output `__m256i` has 16 words.
            // So it converts 16 pixels at once!
            //
            // My previous code:
            // `let lo_128 = _mm256_castsi256_si128(chunk);` (Low 16 bytes = 4 pixels of u32 (RGBA))
            // `let v_lo = _mm256_cvtepu8_epi16(lo_128);`
            // This converts 16 bytes (4 pixels * 4 components) to 16 words.
            // So `v_lo` contains 4 pixels worth of data expanded to u16.
            // `hi_128` contains the next 4 pixels.
            // So `v_hi` contains next 4 pixels.
            // Total 8 pixels processed.
            //
            // Back to `packus`:
            // `packed = _mm256_packus_epi16(res_lo, res_hi)`
            // `res_lo` has 4 pixels (16 words). `res_hi` has 4 pixels (16 words).
            // `packus` saturates 16-bit -> 8-bit.
            // It takes 2 256-bit vectors.
            // Result is 256-bit vector (32 bytes).
            // The instruction packs lanes independently?
            // AVX2 `vpackuswb`:
            // Dst[0..63] = Saturate(a[0..3], a[4..7]...) - Low 64 bits of `a` packed.
            // Dst[64..127] = Saturate(b[0..3]...) - Low 64 bits of `b` packed.
            // Dst[128..191] = Saturate(a[high]...) - High 64 bits of `a`.
            // Dst[192..255] = Saturate(b[high]...) - High 64 bits of `b`.
            //
            // `res_lo` corresponds to first 4 pixels (Bytes 0..15).
            // `res_hi` corresponds to next 4 pixels (Bytes 16..31).
            // We want output: [P0..P3, P4..P7].
            //
            // `res_lo` (256 bits):
            // Lane 0 (Low 128): Words for P0, P1.
            // Lane 1 (High 128): Words for P2, P3.
            //
            // `res_hi` (256 bits):
            // Lane 0: Words for P4, P5.
            // Lane 1: Words for P6, P7.
            //
            // `packus(res_lo, res_hi)`:
            // Output Low 128 (Lane 0): Pack(res_lo.Lane0, res_hi.Lane0).
            // -> P0, P1 followed by P4, P5.
            // Output High 128 (Lane 1): Pack(res_lo.Lane1, res_hi.Lane1).
            // -> P2, P3 followed by P6, P7.
            //
            // So we get: [P0, P1, P4, P5, P2, P3, P6, P7].
            // We want: [P0, P1, P2, P3, P4, P5, P6, P7].
            //
            // We need to permute.
            // Current: 0, 1, 4, 5, 2, 3, 6, 7 (in terms of 64-bit blocks? No, 32-bit pixels).
            // Blocks of 64 bits (2 pixels):
            // Lane 0: [P0 P1] [P4 P5]
            // Lane 1: [P2 P3] [P6 P7]
            //
            // Permute over 64-bit blocks `_mm256_permute4x64_epi64`.
            // Indices: 0, 2, 1, 3.
            // 0 -> [P0 P1]
            // 2 -> [P2 P3] (From Lane 1 low)
            // 1 -> [P4 P5] (From Lane 0 high)
            // 3 -> [P6 P7]
            //
            // So `_mm256_permute4x64_epi64(packed, 0xD8)` (11 01 10 00 -> 3, 1, 2, 0)
            // Wait: `PERM` control is `q3 q2 q1 q0` (bits 7:6, 5:4, 3:2, 1:0) selecting source qwords.
            // We want dest q0 to be source q0 (0).
            // Dest q1 to be source q2 (2).
            // Dest q2 to be source q1 (1).
            // Dest q3 to be source q3 (3).
            // Order: 3 1 2 0 -> 11 01 10 00 = 0xD8.
            let permuted = _mm256_permute4x64_epi64(packed, 0xD8);

            // Restore Alpha (since additive blend usually shouldn't mess up alpha, or should it?)
            // The implementation does: `*d = (d_val & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;`
            // So it preserves Dest Alpha.
            // Our SIMD calculation saturated dest + src*intensity.
            // If src alpha is 0, we added 0. If dest alpha is 255, we might have added something and saturated.
            // But we want to strictly preserve dest alpha bitwise.
            let rgb_mask = _mm256_set1_epi32(0x00FF_FFFFu32 as i32);
            let rgb_result = _mm256_and_si256(permuted, rgb_mask);
            let final_result = _mm256_or_si256(rgb_result, d_alpha);

            _mm256_storeu_si256(d_ptr.cast(), final_result);

            d_ptr = d_ptr.add(8);
            s_ptr = s_ptr.add(8);
        }
    }
}
