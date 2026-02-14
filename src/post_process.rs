//! Post-processing effects.
//!
//! Functions to apply full-screen effects to a `Framebuffer`.
//!
//! # Examples
//!
//! ```
//! use abrash::framebuffer::Framebuffer;
//! use abrash::post_process::{apply_grayscale, apply_scanlines, apply_invert};
//!
//! let mut fb = Framebuffer::new(100, 100).unwrap();
//! // ... render something ...
//!
//! // Apply effects
//! apply_grayscale(&mut fb);
//! apply_scanlines(&mut fb);
//! apply_invert(&mut fb);
//! ```

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

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

    // 1. Extract bright pixels
    // We reuse a scratch buffer for this to avoid allocating multiple full-frame buffers.
    // We need at least one full-frame buffer for the extracted/blurred result.
    // We'll use two buffers for the separable blur (ping-pong).
    let mut bright_pixels = vec![0u32; width * height];
    let mut scratch_buffer = vec![0u32; width * height];

    extract_bright_pixels(pixels, &mut bright_pixels, threshold);

    // 2. Blur the bright pixels
    // Horizontal pass: bright_pixels -> scratch_buffer
    box_blur_horizontal(&bright_pixels, &mut scratch_buffer, width, height, blur_radius);
    // Vertical pass: scratch_buffer -> bright_pixels
    box_blur_vertical(&scratch_buffer, &mut bright_pixels, width, height, blur_radius);

    // 3. Composite back
    blend_additive(pixels, &bright_pixels, intensity);
}

fn extract_bright_pixels(src: &[u32], dest: &mut [u32], threshold: u8) {
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            let len = src.len();
            let simd_len = len & !7;
            unsafe {
                extract_bright_pixels_avx2(
                    &src[..simd_len],
                    &mut dest[..simd_len],
                    threshold,
                );
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

    // Reuse luminance weights from grayscale
    // W0=29(B), W1=150(G), W2=77(R), W3=0(A)
    let weights = _mm256_set1_epi64x(0x0000_004D_0096_001D);
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
    let black_pixel = _mm256_set1_epi32(0xFF00_0000u32 as i32);
    let threshold_vec = _mm256_set1_epi32(threshold as i32);

    let len = src.len();
    let mut s_ptr = src.as_ptr();
    let mut d_ptr = dest.as_mut_ptr();
    let end_ptr = s_ptr.add(len);

    while s_ptr < end_ptr {
        let chunk = _mm256_loadu_si256(s_ptr.cast());

        // 1. Calculate Luminance (same as grayscale)
        let alphas = _mm256_and_si256(chunk, alpha_mask);
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

fn box_blur_horizontal(src: &[u32], dest: &mut [u32], width: usize, height: usize, radius: u32) {
    let radius = radius as usize;
    // Window size (kernel width)
    let kernel_size = 2 * radius + 1;
    let scale = 1.0 / (kernel_size as f32);
    // Use fixed point for accumulation? No, f32 is fine for simplicity in green phase.
    // Optimization: Precompute integer scale? (x * mult) >> shift
    // For now, float is safe.

    for y in 0..height {
        let row_offset = y * width;
        let src_row = &src[row_offset..row_offset + width];
        let dst_row = &mut dest[row_offset..row_offset + width];

        // Initialize accumulator
        let mut r_acc = 0;
        let mut g_acc = 0;
        let mut b_acc = 0;

        // Pre-fill accumulator with left-boundary pixels (clamped to first pixel)
        // For x < 0, use src_row[0]
        let first_pixel = src_row[0];
        let r_first = (first_pixel >> 16) & 0xFF;
        let g_first = (first_pixel >> 8) & 0xFF;
        let b_first = first_pixel & 0xFF;

        for _ in 0..=radius {
            r_acc += r_first;
            g_acc += g_first;
            b_acc += b_first;
        }

        // Add initial right side
        for x in 1..=radius {
            let p = src_row[x.min(width - 1)];
            r_acc += (p >> 16) & 0xFF;
            g_acc += (p >> 8) & 0xFF;
            b_acc += p & 0xFF;
        }

        for x in 0..width {
            // Write current blurred pixel
            let r_avg = (r_acc as f32 * scale) as u32;
            let g_avg = (g_acc as f32 * scale) as u32;
            let b_avg = (b_acc as f32 * scale) as u32;
            dst_row[x] = 0xFF00_0000 | (r_avg << 16) | (g_avg << 8) | b_avg;

            // Shift window
            // Remove outgoing pixel (x - radius)
            let outgoing_idx = (x as isize - radius as isize).max(0) as usize;
            let p_out = src_row[outgoing_idx];
            r_acc -= (p_out >> 16) & 0xFF;
            g_acc -= (p_out >> 8) & 0xFF;
            b_acc -= p_out & 0xFF;

            // Add incoming pixel (x + radius + 1)
            let incoming_idx = (x + radius + 1).min(width - 1);
            let p_in = src_row[incoming_idx];
            r_acc += (p_in >> 16) & 0xFF;
            g_acc += (p_in >> 8) & 0xFF;
            b_acc += p_in & 0xFF;
        }
    }
}

fn box_blur_vertical(src: &[u32], dest: &mut [u32], width: usize, height: usize, radius: u32) {
    let radius = radius as usize;
    let kernel_size = 2 * radius + 1;
    let scale = 1.0 / (kernel_size as f32);

    for x in 0..width {
        // Initialize accumulator
        let mut r_acc = 0;
        let mut g_acc = 0;
        let mut b_acc = 0;

        let first_pixel = src[x]; // (0, x)
        let r_first = (first_pixel >> 16) & 0xFF;
        let g_first = (first_pixel >> 8) & 0xFF;
        let b_first = first_pixel & 0xFF;

        for _ in 0..=radius {
            r_acc += r_first;
            g_acc += g_first;
            b_acc += b_first;
        }

        for y in 1..=radius {
            let p = src[y.min(height - 1) * width + x];
            r_acc += (p >> 16) & 0xFF;
            g_acc += (p >> 8) & 0xFF;
            b_acc += p & 0xFF;
        }

        for y in 0..height {
            let r_avg = (r_acc as f32 * scale) as u32;
            let g_avg = (g_acc as f32 * scale) as u32;
            let b_avg = (b_acc as f32 * scale) as u32;

            dest[y * width + x] = 0xFF00_0000 | (r_avg << 16) | (g_avg << 8) | b_avg;

            // Shift window
            let outgoing_y = (y as isize - radius as isize).max(0) as usize;
            let p_out = src[outgoing_y * width + x];
            r_acc -= (p_out >> 16) & 0xFF;
            g_acc -= (p_out >> 8) & 0xFF;
            b_acc -= p_out & 0xFF;

            let incoming_y = (y + radius + 1).min(height - 1);
            let p_in = src[incoming_y * width + x];
            r_acc += (p_in >> 16) & 0xFF;
            g_acc += (p_in >> 8) & 0xFF;
            b_acc += p_in & 0xFF;
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
        _mm256_add_epi16, _mm256_castsi256_si128, _mm256_cvtepu8_epi16, _mm256_extracti128_si256,
        _mm256_inserti128_si256, _mm256_loadu_si256, _mm256_mullo_epi16, _mm256_or_si256,
        _mm256_packus_epi16, _mm256_permute4x64_epi64, _mm256_set1_epi16, _mm256_set1_epi32,
        _mm256_srai_epi16, _mm256_storeu_si256, _mm256_and_si256,
    };

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
        let rgb_mask = _mm256_set1_epi32(0x00FFFFFF);
        let rgb_result = _mm256_and_si256(permuted, rgb_mask);
        let final_result = _mm256_or_si256(rgb_result, d_alpha);

        _mm256_storeu_si256(d_ptr.cast(), final_result);

        d_ptr = d_ptr.add(8);
        s_ptr = s_ptr.add(8);
    }
}

/// Applies a grayscale filter to the framebuffer in-place.
///
/// Uses a fixed-point approximation of the luminance formula:
/// `Y = 0.299*R + 0.587*G + 0.114*B`
///
/// Approximated as: `Y = (77*R + 150*G + 29*B) >> 8`
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::apply_grayscale;
///
/// let mut fb = Framebuffer::new(1, 1).unwrap();
/// fb.set_pixel(0, 0, 0xFFFF0000); // Red
/// apply_grayscale(&mut fb);
/// // Red component is 255. 77*255/256 = 76.
/// // Result should be grey (76, 76, 76).
/// let p = fb.get_pixel(0, 0).unwrap();
/// assert_eq!(p & 0xFF, 76);
/// ```
pub fn apply_grayscale(fb: &mut Framebuffer) {
    let pixels = fb.as_mut_slice();

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe { apply_grayscale_avx2(pixels) };
            return;
        }
    }

    for pixel in pixels.iter_mut() {
        // Format: 0xAARRGGBB
        let p = *pixel;

        // Fixed-point luminance calculation
        let luminance = pixel_luminance(p) as u32;

        // Preserve Alpha, set RGB to luminance
        *pixel = (p & 0xFF00_0000) | (luminance << 16) | (luminance << 8) | luminance;
    }
}

/// Simulates CRT scanlines by darkening every odd row.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::apply_scanlines;
///
/// let mut fb = Framebuffer::new(1, 2).unwrap();
/// fb.clear(0xFFFFFFFF); // White
/// apply_scanlines(&mut fb);
///
/// // Row 0 is untouched
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);
///
/// // Row 1 is darkened (halved)
/// // 0xFF >> 1 = 0x7F
/// assert_eq!(fb.get_pixel(0, 1).unwrap(), 0xFF7F7F7F);
/// ```
pub fn apply_scanlines(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Iterate over odd rows only
    for y in (1..height).step_by(2) {
        let start = y * width;
        let end = start + width;
        let row = &mut pixels[start..end];
        for pixel in row.iter_mut() {
            let p = *pixel;
            // Halve RGB components: (color >> 1) & mask
            // Preserve Alpha: (p & 0xFF00_0000)
            *pixel = ((p >> 1) & 0x7F7F_7F7F) | (p & 0xFF00_0000);
        }
    }
}

/// Inverts the colors of the framebuffer in-place.
///
/// This effect negates the RGB channels while preserving the Alpha channel.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::apply_invert;
///
/// let mut fb = Framebuffer::new(1, 1).unwrap();
/// fb.set_pixel(0, 0, 0xFF000000); // Black
/// apply_invert(&mut fb);
///
/// // Alpha is preserved (FF), color is inverted (000000 -> FFFFFF)
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF); // White
/// ```
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_invert_avx2(pixels: &mut [u32]) {
    for pixel in pixels.iter_mut() {
        *pixel = *pixel ^ 0x00FF_FFFF;
    }
}

pub fn apply_invert(fb: &mut Framebuffer) {
    let pixels = fb.as_mut_slice();

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe { apply_invert_avx2(pixels) };
            return;
        }
    }

    for pixel in pixels.iter_mut() {
        let p = *pixel;
        *pixel = p ^ 0x00FF_FFFF;
    }
}

/// Applies a sepia tone effect to the framebuffer in-place.
///
/// Converts the image to sepia using standard luminance weights and tinting.
///
/// Formula:
/// ```text
/// NewR = (0.393 * R + 0.769 * G + 0.189 * B)
/// NewG = (0.349 * R + 0.686 * G + 0.168 * B)
/// NewB = (0.272 * R + 0.534 * G + 0.131 * B)
/// ```
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::apply_sepia;
///
/// let mut fb = Framebuffer::new(1, 1).unwrap();
/// fb.set_pixel(0, 0, 0xFFFFFFFF); // White
/// apply_sepia(&mut fb);
/// // Result is tinted yellowish-brown.
/// ```
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_sepia_avx2(pixels: &mut [u32]) {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_castsi256_si128, _mm256_cvtepu8_epi16, _mm256_extracti128_si256,
        _mm256_hadd_epi32, _mm256_loadu_si256, _mm256_madd_epi16, _mm256_min_epi32,
        _mm256_or_si256, _mm256_permute4x64_epi64, _mm256_set1_epi32, _mm256_set1_epi64x,
        _mm256_slli_epi32, _mm256_srai_epi32, _mm256_storeu_si256,
    };

    // Weights for Sepia
    // NewR = (402 * R + 787 * G + 194 * B) >> 10
    // NewG = (357 * R + 702 * G + 172 * B) >> 10
    // NewB = (279 * R + 547 * G + 134 * B) >> 10

    // Memory layout: B G R A (little endian)
    // Madd takes pairs: (B, G) and (R, A)
    // Weights are stored as i16 in 64-bit blocks: W3 W2 W1 W0

    // Weights for Red
    // B*194 + G*787 -> W0=194(0xC2), W1=787(0x313)
    // R*402 + A*0   -> W2=402(0x192), W3=0
    let w_r = unsafe { _mm256_set1_epi64x(0x0000_0192_0313_00C2) };

    // Weights for Green
    // B*172 + G*702 -> W0=172(0xAC), W1=702(0x2BE)
    // R*357 + A*0   -> W2=357(0x165), W3=0
    let w_g = unsafe { _mm256_set1_epi64x(0x0000_0165_02BE_00AC) };

    // Weights for Blue
    // B*134 + G*547 -> W0=134(0x86), W1=547(0x223)
    // R*279 + A*0   -> W2=279(0x117), W3=0
    let w_b = unsafe { _mm256_set1_epi64x(0x0000_0117_0223_0086) };

    let alpha_mask = unsafe { _mm256_set1_epi32(0xFF00_0000u32 as i32) };
    let max_val = unsafe { _mm256_set1_epi32(255) };

    let len = pixels.len();
    let simd_len = len & !7;
    let mut ptr = pixels.as_mut_ptr();
    let end_ptr = unsafe { ptr.add(simd_len) };

    while ptr < end_ptr {
        unsafe {
            let chunk = _mm256_loadu_si256(ptr.cast());

            // Extract Alpha
            let alphas = _mm256_and_si256(chunk, alpha_mask);

            // Unpack to i16 (0..255)
            let lo_128 = _mm256_castsi256_si128(chunk);
            let hi_128 = _mm256_extracti128_si256(chunk, 1);
            let v_lo = _mm256_cvtepu8_epi16(lo_128);
            let v_hi = _mm256_cvtepu8_epi16(hi_128);

            // Compute Red
            let r_lo = _mm256_madd_epi16(v_lo, w_r);
            let r_hi = _mm256_madd_epi16(v_hi, w_r);
            let r_sum = _mm256_hadd_epi32(r_lo, r_hi);
            let r_ord = _mm256_permute4x64_epi64(r_sum, 0xD8);
            let r_val = _mm256_srai_epi32(r_ord, 10);
            let r_clamped = _mm256_min_epi32(r_val, max_val);

            // Compute Green
            let g_lo = _mm256_madd_epi16(v_lo, w_g);
            let g_hi = _mm256_madd_epi16(v_hi, w_g);
            let g_sum = _mm256_hadd_epi32(g_lo, g_hi);
            let g_ord = _mm256_permute4x64_epi64(g_sum, 0xD8);
            let g_val = _mm256_srai_epi32(g_ord, 10);
            let g_clamped = _mm256_min_epi32(g_val, max_val);

            // Compute Blue
            let b_lo = _mm256_madd_epi16(v_lo, w_b);
            let b_hi = _mm256_madd_epi16(v_hi, w_b);
            let b_sum = _mm256_hadd_epi32(b_lo, b_hi);
            let b_ord = _mm256_permute4x64_epi64(b_sum, 0xD8);
            let b_val = _mm256_srai_epi32(b_ord, 10);
            let b_clamped = _mm256_min_epi32(b_val, max_val);

            // Pack: B | G<<8 | R<<16 | A
            let g_shift = _mm256_slli_epi32(g_clamped, 8);
            let r_shift = _mm256_slli_epi32(r_clamped, 16);

            let result = _mm256_or_si256(
                b_clamped,
                _mm256_or_si256(g_shift, _mm256_or_si256(r_shift, alphas)),
            );

            _mm256_storeu_si256(ptr.cast(), result);
            ptr = ptr.add(8);
        }
    }
}

fn apply_sepia_scalar(pixels: &mut [u32]) {
    for pixel in pixels.iter_mut() {
        let p = *pixel;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        // Fixed-point arithmetic (scaled by 1024)
        let new_r = (402 * r + 787 * g + 194 * b) >> 10;
        let new_g = (357 * r + 702 * g + 172 * b) >> 10;
        let new_b = (279 * r + 547 * g + 134 * b) >> 10;

        let new_r = new_r.min(255);
        let new_g = new_g.min(255);
        let new_b = new_b.min(255);

        *pixel = (p & 0xFF00_0000) | (new_r << 16) | (new_g << 8) | new_b;
    }
}

pub fn apply_sepia(fb: &mut Framebuffer) {
    let pixels = fb.as_mut_slice();

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            let len = pixels.len();
            let simd_len = len & !7;

            // Process multiple of 8 with AVX2
            // SAFETY: We checked feature detection and pass a valid mutable slice.
            unsafe { apply_sepia_avx2(&mut pixels[..simd_len]) };

            // Process the tail with scalar
            apply_sepia_scalar(&mut pixels[simd_len..]);
            return;
        }
    }

    // Scalar fallback
    apply_sepia_scalar(pixels);
}

/// Applies chromatic aberration by shifting Red and Blue channels.
///
/// *   Red channel is shifted left by `offset`.
/// *   Blue channel is shifted right by `offset`.
/// *   Green channel remains unchanged.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::apply_chromatic_aberration;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// fb.set_pixel(50, 50, 0xFFFFFFFF); // White
/// apply_chromatic_aberration(&mut fb, 5);
/// ```
pub fn apply_chromatic_aberration(fb: &mut Framebuffer, offset: u32) {
    if offset == 0 {
        return;
    }
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let offset = offset as usize;

    let mut row_buffer = vec![0u32; width];
    let pixels = fb.as_mut_slice();

    for y in 0..height {
        let row_start = y * width;
        let row_end = row_start + width;
        let row_pixels = &mut pixels[row_start..row_end];

        // Copy current row to scratch buffer
        row_buffer.copy_from_slice(row_pixels);

        for x in 0..width {
            // Green (G) from current pixel
            let g = (row_buffer[x] >> 8) & 0xFF;
            // Alpha (A) from current pixel
            let a = (row_buffer[x] >> 24) & 0xFF;

            // Red (R) from left (x - offset)
            let r = if x >= offset {
                (row_buffer[x - offset] >> 16) & 0xFF
            } else {
                0
            };

            // Blue (B) from right (x + offset)
            let b = if x + offset < width {
                row_buffer[x + offset] & 0xFF
            } else {
                0
            };

            row_pixels[x] = (a << 24) | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_grayscale_avx2(pixels: &mut [u32]) {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_castsi256_si128, _mm256_cvtepu8_epi16, _mm256_extracti128_si256,
        _mm256_hadd_epi32, _mm256_loadu_si256, _mm256_madd_epi16, _mm256_or_si256,
        _mm256_permute4x64_epi64, _mm256_set1_epi32, _mm256_set1_epi64x, _mm256_slli_epi32,
        _mm256_srai_epi32, _mm256_storeu_si256,
    };

    // Weights: B=29, G=150, R=77, A=0
    // Memory layout: B G R A
    // Pair 1: B, G -> Weights 29, 150
    // Pair 2: R, A -> Weights 77, 0
    // _mm256_set1_epi64x replicates 64-bit value to all 4 positions.
    // 64 bits = 4 * 16 bits: W3 W2 W1 W0
    // W0=29, W1=150, W2=77, W3=0
    // 0x0000_004D_0096_001D (hex)
    // 77=0x4D, 150=0x96, 29=0x1D
    let weights = _mm256_set1_epi64x(0x0000_004D_0096_001D);
    let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

    let len = pixels.len();
    let simd_len = len & !7;
    let mut ptr = pixels.as_mut_ptr();

    // SAFETY: We perform pointer arithmetic within bounds of the slice.
    unsafe {
        let end_ptr = ptr.add(simd_len);

        while ptr < end_ptr {
            let chunk = _mm256_loadu_si256(ptr.cast());

            // Extract Alpha
            let alphas = _mm256_and_si256(chunk, alpha_mask);

            // Unpack to i16 (0..255)
            let lo_128 = _mm256_castsi256_si128(chunk);
            let hi_128 = _mm256_extracti128_si256(chunk, 1);

            let v_lo = _mm256_cvtepu8_epi16(lo_128);
            let v_hi = _mm256_cvtepu8_epi16(hi_128);

            // Multiply and horizontal add pairs
            // Result: 32-bit integers.
            // E.g. low dword: (B*29 + G*150)
            // Next dword: (R*77 + A*0)
            let prod_lo = _mm256_madd_epi16(v_lo, weights);
            let prod_hi = _mm256_madd_epi16(v_hi, weights);

            // Horizontal add to combine (BG) + (RA)
            // hadd_epi32(a, b) -> a0+a1, a2+a3, b0+b1, b2+b3 ...
            let sums_scrambled = _mm256_hadd_epi32(prod_lo, prod_hi);

            // Fix order: Swap (P4, P5) with (P2, P3).
            // P4, P5 is Low Lane High Part (Idx 1).
            // P2, P3 is High Lane Low Part (Idx 2).
            // Target: Idx 0, Idx 2, Idx 1, Idx 3.
            // Control: 0xD8 (11 01 10 00).
            let sums = _mm256_permute4x64_epi64(sums_scrambled, 0xD8);

            // Shift right by 8 to divide by 256
            let luma = _mm256_srai_epi32(sums, 8);

            // Reconstruct pixel: 0x00LLLLLL
            let luma8 = _mm256_slli_epi32(luma, 8);
            let luma16 = _mm256_slli_epi32(luma, 16);

            let gray_pixels = _mm256_or_si256(luma, _mm256_or_si256(luma8, luma16));

            // Combine with Alpha
            let result = _mm256_or_si256(gray_pixels, alphas);

            _mm256_storeu_si256(ptr.cast(), result);
            ptr = ptr.add(8);
        }
    }

    // Handle remaining pixels scalar
    for pixel in pixels[simd_len..].iter_mut() {
        let p = *pixel;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;
        let luminance = (77 * r + 150 * g + 29 * b) >> 8;
        *pixel = (p & 0xFF00_0000) | (luminance << 16) | (luminance << 8) | luminance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_invert() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFF00_0000); // Black
        fb.set_pixel(1, 0, 0xFFFF_FFFF); // White
        fb.set_pixel(0, 1, 0xFFFF_0000); // Red
        fb.set_pixel(1, 1, 0xFF00_FF00); // Green

        apply_invert(&mut fb);

        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_FFFF); // White
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFF00_0000); // Black
        assert_eq!(fb.get_pixel(0, 1).unwrap(), 0xFF00_FFFF); // Cyan
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFFFF_00FF); // Magenta
    }

    #[test]
    fn test_apply_grayscale() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_0000); // Red
        apply_grayscale(&mut fb);
        // Red component is 255. 77*255/256 = 76.
        // Result should be grey (76, 76, 76).
        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p & 0xFF, 76);
        assert_eq!((p >> 8) & 0xFF, 76);
        assert_eq!((p >> 16) & 0xFF, 76);
    }

    #[test]
    fn test_apply_grayscale_simd() {
        let width = 16;
        let mut fb = Framebuffer::new(width, 1).unwrap();
        // Set gradient: Use different Red values to catch pixel reordering bugs.
        // We use multiples of 16 to ensure distinct grayscale values.
        for x in 0..width as i32 {
            let val = (x * 10) as u32;
            fb.set_pixel(x, 0, 0xFF00_0000 | (val << 16));
        }

        apply_grayscale(&mut fb);

        for x in 0..width as i32 {
            let val = (x * 10) as u32;
            // Expected: (77 * val + 0 + 0) >> 8
            let expected_gray = (77 * val) >> 8;
            let p = fb.get_pixel(x, 0).unwrap();
            let r = (p >> 16) & 0xFF;
            assert_eq!(
                r, expected_gray,
                "Pixel {x} mismatch. Got {r}, expected {expected_gray}",
            );
        }
    }

    #[test]
    fn test_apply_sepia() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        // Set pixel to white (255, 255, 255)
        fb.set_pixel(0, 0, 0xFFFFFFFF);
        apply_sepia(&mut fb);

        // Expected values for White input (255, 255, 255):
        // R: (0.393 + 0.769 + 0.189) * 255 = 1.351 * 255 = 344.5 -> 255
        // G: (0.349 + 0.686 + 0.168) * 255 = 1.203 * 255 = 306.7 -> 255
        // B: (0.272 + 0.534 + 0.131) * 255 = 0.937 * 255 = 238.9 -> 238 (approx)

        let p = fb.get_pixel(0, 0).unwrap();
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        assert_eq!(r, 255, "Red channel mismatch");
        assert_eq!(g, 255, "Green channel mismatch");
        assert!(b >= 235 && b <= 240, "Blue channel mismatch, got {}", b);

        // Test with Red (255, 0, 0)
        fb.set_pixel(0, 0, 0xFFFF0000);
        apply_sepia(&mut fb);
        // R: 0.393 * 255 = 100
        // G: 0.349 * 255 = 89
        // B: 0.272 * 255 = 69
        let p = fb.get_pixel(0, 0).unwrap();
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        assert!(
            (r as i32 - 100).abs() <= 2,
            "Red mismatch for red pixel, got {}",
            r
        );
        assert!(
            (g as i32 - 89).abs() <= 2,
            "Green mismatch for red pixel, got {}",
            g
        );
        assert!(
            (b as i32 - 69).abs() <= 2,
            "Blue mismatch for red pixel, got {}",
            b
        );
    }
}
