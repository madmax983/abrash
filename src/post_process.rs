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
use crate::math::{Mat4, Vec3};
use crate::utils::pixel_luminance;
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;

thread_local! {
    static BLOOM_BUFFERS: RefCell<(Vec<u32>, Vec<u32>)> = const { RefCell::new((Vec::new(), Vec::new())) };
    static SSAO_CONTEXT: RefCell<SsaoContext> = RefCell::new(SsaoContext::default());
}

const KERNEL_SIZE: usize = 16;
const NOISE_SIZE: usize = 4;

struct SsaoContext {
    occlusion_buffer: Vec<f32>,
    scratch_buffer: Vec<f32>,
    kernel: [Vec3; KERNEL_SIZE],
    noise: [Vec3; NOISE_SIZE * NOISE_SIZE],
    initialized: bool,
}

impl Default for SsaoContext {
    fn default() -> Self {
        Self {
            occlusion_buffer: Vec::new(),
            scratch_buffer: Vec::new(),
            kernel: [Vec3::default(); KERNEL_SIZE],
            noise: [Vec3::default(); NOISE_SIZE * NOISE_SIZE],
            initialized: false,
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

    BLOOM_BUFFERS.with(|buffers| {
        let mut borrowed = buffers.borrow_mut();
        let (bright_pixels, scratch_buffer) = &mut *borrowed;

        // Ensure buffers are large enough
        if bright_pixels.len() < needed_size {
            bright_pixels.resize(needed_size, 0);
        }
        if scratch_buffer.len() < needed_size {
            scratch_buffer.resize(needed_size, 0);
        }

        let bright_slice = &mut bright_pixels[..needed_size];
        let scratch_slice = &mut scratch_buffer[..needed_size];

        // 1. Extract bright pixels
        extract_bright_pixels(pixels, bright_slice, threshold);

        // 2. Blur the bright pixels
        // Horizontal pass: bright_pixels -> scratch_buffer
        box_blur_horizontal(bright_slice, scratch_slice, width, height, blur_radius);
        // Vertical pass: scratch_buffer -> bright_pixels
        box_blur_vertical(scratch_slice, bright_slice, width, height, blur_radius);

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

fn box_blur_horizontal(src: &[u32], dest: &mut [u32], width: usize, height: usize, radius: u32) {
    let radius = radius as usize;
    // Window size (kernel width)
    let kernel_size = 2 * radius + 1;
    let scale = 1.0 / (kernel_size as f32);

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

        for (x, dst_pixel) in dst_row.iter_mut().enumerate() {
            // Write current blurred pixel
            let r_avg = (r_acc as f32 * scale) as u32;
            let g_avg = (g_acc as f32 * scale) as u32;
            let b_avg = (b_acc as f32 * scale) as u32;
            *dst_pixel = 0xFF00_0000 | (r_avg << 16) | (g_avg << 8) | b_avg;

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
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                box_blur_vertical_avx2(src, dest, width, height, radius);
            }
            return;
        }
    }

    box_blur_vertical_scalar(src, dest, width, height, radius);
}

fn box_blur_vertical_scalar(
    src: &[u32],
    dest: &mut [u32],
    width: usize,
    height: usize,
    radius: u32,
) {
    let radius = radius as usize;
    let kernel_size = 2 * radius + 1;
    let scale = 1.0 / (kernel_size as f32);

    // Optimized vertical blur: Iterate over Y, update all X.
    // Improves cache locality.

    // Accumulators for each column
    let mut r_acc = vec![0u32; width];
    let mut g_acc = vec![0u32; width];
    let mut b_acc = vec![0u32; width];

    // Pre-fill accumulators
    // For y=0 window is [-r, r]
    // Add src[0] (r+1) times (clamped top)
    // Add src[1..=r] (1 time each)
    let row0 = &src[0..width];
    for x in 0..width {
        let p = row0[x];
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;
        // (radius + 1) copies of row 0
        r_acc[x] += r * (radius as u32 + 1);
        g_acc[x] += g * (radius as u32 + 1);
        b_acc[x] += b * (radius as u32 + 1);
    }

    for y in 1..=radius {
        let row_idx = y.min(height - 1);
        let row = &src[row_idx * width..(row_idx + 1) * width];
        for x in 0..width {
            let p = row[x];
            r_acc[x] += (p >> 16) & 0xFF;
            g_acc[x] += (p >> 8) & 0xFF;
            b_acc[x] += p & 0xFF;
        }
    }

    for y in 0..height {
        let dst_row_start = y * width;
        let dst_row = &mut dest[dst_row_start..dst_row_start + width];

        for (x, dst_pixel) in dst_row.iter_mut().enumerate() {
            let r = (r_acc[x] as f32 * scale) as u32;
            let g = (g_acc[x] as f32 * scale) as u32;
            let b = (b_acc[x] as f32 * scale) as u32;
            *dst_pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }

        // Update accumulators for next row
        // Outgoing: y - radius
        let out_y = (y as isize - radius as isize).max(0) as usize;
        let out_row = &src[out_y * width..(out_y + 1) * width];

        // Incoming: y + radius + 1
        let in_y = (y + radius + 1).min(height - 1);
        let in_row = &src[in_y * width..(in_y + 1) * width];

        for x in 0..width {
            let p_out = out_row[x];
            let p_in = in_row[x];

            r_acc[x] = r_acc[x] + ((p_in >> 16) & 0xFF) - ((p_out >> 16) & 0xFF);
            g_acc[x] = g_acc[x] + ((p_in >> 8) & 0xFF) - ((p_out >> 8) & 0xFF);
            b_acc[x] = b_acc[x] + (p_in & 0xFF) - (p_out & 0xFF);
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn box_blur_vertical_avx2(
    src: &[u32],
    dest: &mut [u32],
    width: usize,
    height: usize,
    radius: u32,
) {
    use std::arch::x86_64::{
        _mm256_add_epi32, _mm256_and_si256, _mm256_castsi256_si128, _mm256_cvtepi32_ps,
        _mm256_cvtepu8_epi32, _mm256_cvttps_epi32, _mm256_loadu_si256, _mm256_mul_ps,
        _mm256_mullo_epi32, _mm256_or_si256, _mm256_packus_epi16, _mm256_packus_epi32,
        _mm256_permute4x64_epi64, _mm256_set1_epi32, _mm256_set1_ps, _mm256_slli_epi32,
        _mm256_srai_epi32, _mm256_srli_epi32, _mm256_storeu_si256, _mm256_sub_epi32,
    };

    unsafe {
        let radius = radius as usize;
        let count = radius as i32 + 1;
        let kernel_size = 2 * radius + 1;
        let scale = 1.0 / (kernel_size as f32);
        let scale_vec = _mm256_set1_ps(scale);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        // Accumulators
        let mut r_acc = vec![0i32; width];
        let mut g_acc = vec![0i32; width];
        let mut b_acc = vec![0i32; width];

        // Helper to add a row to accumulators (SIMD)
        // Note: We use i32 for accumulators to prevent overflow.
        // Max sum = 255 * (2*radius + 1). If radius=10, max=5355. Fits in i16 too, but i32 is safer and easier.
        // AVX2 doesn't have unpack u8 to i32 directly.
        // We can use `_mm256_cvtepu8_epi32` which takes __m128i (16 bytes, 4 pixels? No, 128 bits = 16 bytes).
        // `cvtepu8_epi32` converts low 8 bytes (8 ints) to 8 i32s (256 bits).
        // So we can process 8 pixels at a time. Perfect.

        // 1. Pre-fill accumulators
        {
            // Add row 0 (radius+1 times)
            let row0_ptr = src.as_ptr();
            let mut x = 0;
            let count_vec = _mm256_set1_epi32(count);

            while x + 8 <= width {
                // Load 8 pixels (32 bytes)
                // We need to extract R, G, B separately.
                // Pixel: A R G B
                // _mm256_loadu_si256 loads 8 pixels.
                let pixels = _mm256_loadu_si256(row0_ptr.add(x).cast());

                // Mask and shift to get channels as i32
                // B: pixels & 0xFF
                // G: (pixels >> 8) & 0xFF
                // R: (pixels >> 16) & 0xFF
                // Since we need them as i32, we can't just mask.
                // Actually, we can use shift + mask.
                // But doing this for 8 pixels in parallel is tricky because channels are interleaved.
                // Alternative:
                // Load 8 pixels.
                // Use `vpand` to get B (if aligned). No.
                //
                // Better: use `_mm256_cvtepu8_epi32`?
                // That takes packed u8s. Our u8s are interleaved.
                //
                // We can use `vpshufb` (shuffle bytes) to deinterleave?
                //
                // Or just use shifts and masks.
                // B = pixels & 0xFF.
                // G = (pixels >> 8) & 0xFF.
                // R = (pixels >> 16) & 0xFF.
                //
                // `_mm256_and_si256` works on whole vector.
                // `_mm256_srli_epi32` works on each 32-bit element (pixel).
                // So:
                // b_vals = _mm256_and_si256(pixels, 0xFF);
                // g_vals = _mm256_and_si256(_mm256_srli_epi32(pixels, 8), 0xFF);
                // r_vals = _mm256_and_si256(_mm256_srli_epi32(pixels, 16), 0xFF);
                // This works perfectly!

                let mask_ff = _mm256_set1_epi32(0xFF);
                let b_vals = _mm256_and_si256(pixels, mask_ff);
                let g_vals = _mm256_and_si256(_mm256_srli_epi32(pixels, 8), mask_ff);
                let r_vals = _mm256_and_si256(_mm256_srli_epi32(pixels, 16), mask_ff);

                // Multiply by count
                // Since max val is 255*count, it fits in i32. `mullo_epi32` (AVX2).
                let b_added = _mm256_mullo_epi32(b_vals, count_vec); // Wait, AVX2 doesn't have mullo_epi32?
                // AVX2 has `_mm256_mullo_epi32` (VPMULLD). Yes it does.

                let g_added = _mm256_mullo_epi32(g_vals, count_vec);
                let r_added = _mm256_mullo_epi32(r_vals, count_vec); // Actually we can just multiply accumulators once at start?
                // No, we are adding to accumulators.
                // Here we are initializing.

                _mm256_storeu_si256(r_acc.as_mut_ptr().add(x).cast(), r_added);
                _mm256_storeu_si256(g_acc.as_mut_ptr().add(x).cast(), g_added);
                _mm256_storeu_si256(b_acc.as_mut_ptr().add(x).cast(), b_added);

                x += 8;
            }

            // Tail
            for i in x..width {
                let p = *row0_ptr.add(i);
                r_acc[i] = ((p >> 16) & 0xFF) as i32 * count;
                g_acc[i] = ((p >> 8) & 0xFF) as i32 * count;
                b_acc[i] = (p & 0xFF) as i32 * count;
            }

            // Add remaining rows [1..=radius]
            for y in 1..=radius {
                let row_idx = y.min(height - 1);
                let row_ptr = src.as_ptr().add(row_idx * width);
                let mut x = 0;
                let mask_ff = _mm256_set1_epi32(0xFF);

                while x + 8 <= width {
                    let pixels = _mm256_loadu_si256(row_ptr.add(x).cast());
                    let b_vals = _mm256_and_si256(pixels, mask_ff);
                    let g_vals = _mm256_and_si256(_mm256_srli_epi32(pixels, 8), mask_ff);
                    let r_vals = _mm256_and_si256(_mm256_srli_epi32(pixels, 16), mask_ff);

                    let r_curr = _mm256_loadu_si256(r_acc.as_ptr().add(x).cast());
                    let g_curr = _mm256_loadu_si256(g_acc.as_ptr().add(x).cast());
                    let b_curr = _mm256_loadu_si256(b_acc.as_ptr().add(x).cast());

                    _mm256_storeu_si256(
                        r_acc.as_mut_ptr().add(x).cast(),
                        _mm256_add_epi32(r_curr, r_vals),
                    );
                    _mm256_storeu_si256(
                        g_acc.as_mut_ptr().add(x).cast(),
                        _mm256_add_epi32(g_curr, g_vals),
                    );
                    _mm256_storeu_si256(
                        b_acc.as_mut_ptr().add(x).cast(),
                        _mm256_add_epi32(b_curr, b_vals),
                    );

                    x += 8;
                }

                for i in x..width {
                    let p = *row_ptr.add(i);
                    r_acc[i] += ((p >> 16) & 0xFF) as i32;
                    g_acc[i] += ((p >> 8) & 0xFF) as i32;
                    b_acc[i] += (p & 0xFF) as i32;
                }
            }
        }

        // 2. Main loop
        for y in 0..height {
            let dst_ptr = dest.as_mut_ptr().add(y * width);

            let out_y = (y as isize - radius as isize).max(0) as usize;
            let in_y = (y + radius + 1).min(height - 1);

            let out_ptr = src.as_ptr().add(out_y * width);
            let in_ptr = src.as_ptr().add(in_y * width);

            let mut x = 0;
            let mask_ff = _mm256_set1_epi32(0xFF);

            while x + 8 <= width {
                // 1. Write current accumulator to dest
                let r_curr = _mm256_loadu_si256(r_acc.as_ptr().add(x).cast());
                let g_curr = _mm256_loadu_si256(g_acc.as_ptr().add(x).cast());
                let b_curr = _mm256_loadu_si256(b_acc.as_ptr().add(x).cast());

                // Convert to float, scale, convert back
                let r_f = _mm256_cvtepi32_ps(r_curr);
                let g_f = _mm256_cvtepi32_ps(g_curr);
                let b_f = _mm256_cvtepi32_ps(b_curr);

                let r_scaled = _mm256_mul_ps(r_f, scale_vec);
                let g_scaled = _mm256_mul_ps(g_f, scale_vec);
                let b_scaled = _mm256_mul_ps(b_f, scale_vec);

                // Convert to i32 (truncating is fine, or round?)
                // cvttps_epi32 truncates.
                let r_out = _mm256_cvttps_epi32(r_scaled);
                let g_out = _mm256_cvttps_epi32(g_scaled);
                let b_out = _mm256_cvttps_epi32(b_scaled);

                // Pack back to u32 pixel: 0xFFRRGGBB
                // r << 16 | g << 8 | b | 0xFF000000
                let r_shifted = _mm256_slli_epi32(r_out, 16);
                let g_shifted = _mm256_slli_epi32(g_out, 8);
                let pixel = _mm256_or_si256(
                    r_shifted,
                    _mm256_or_si256(g_shifted, _mm256_or_si256(b_out, alpha_mask)),
                );
                _mm256_storeu_si256(dst_ptr.add(x).cast(), pixel);

                // 2. Update accumulators
                // Load outgoing
                let out_pixels = _mm256_loadu_si256(out_ptr.add(x).cast());
                let out_b = _mm256_and_si256(out_pixels, mask_ff);
                let out_g = _mm256_and_si256(_mm256_srli_epi32(out_pixels, 8), mask_ff);
                let out_r = _mm256_and_si256(_mm256_srli_epi32(out_pixels, 16), mask_ff);

                // Load incoming
                let in_pixels = _mm256_loadu_si256(in_ptr.add(x).cast());
                let in_b = _mm256_and_si256(in_pixels, mask_ff);
                let in_g = _mm256_and_si256(_mm256_srli_epi32(in_pixels, 8), mask_ff);
                let in_r = _mm256_and_si256(_mm256_srli_epi32(in_pixels, 16), mask_ff);

                // Update: acc = acc + in - out
                // Combine: diff = in - out
                let diff_r = _mm256_sub_epi32(in_r, out_r);
                let diff_g = _mm256_sub_epi32(in_g, out_g);
                let diff_b = _mm256_sub_epi32(in_b, out_b);

                let r_new = _mm256_add_epi32(r_curr, diff_r);
                let g_new = _mm256_add_epi32(g_curr, diff_g);
                let b_new = _mm256_add_epi32(b_curr, diff_b);

                _mm256_storeu_si256(r_acc.as_mut_ptr().add(x).cast(), r_new);
                _mm256_storeu_si256(g_acc.as_mut_ptr().add(x).cast(), g_new);
                _mm256_storeu_si256(b_acc.as_mut_ptr().add(x).cast(), b_new);

                x += 8;
            }

            // Tail
            for i in x..width {
                // Write
                let r = (r_acc[i] as f32 * scale) as u32;
                let g = (g_acc[i] as f32 * scale) as u32;
                let b = (b_acc[i] as f32 * scale) as u32;
                *dst_ptr.add(i) = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                // Update
                let p_out = *out_ptr.add(i);
                let p_in = *in_ptr.add(i);
                r_acc[i] = r_acc[i] + ((p_in >> 16) & 0xFF) as i32 - ((p_out >> 16) & 0xFF) as i32;
                g_acc[i] = g_acc[i] + ((p_in >> 8) & 0xFF) as i32 - ((p_out >> 8) & 0xFF) as i32;
                b_acc[i] = b_acc[i] + (p_in & 0xFF) as i32 - (p_out & 0xFF) as i32;
            }
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
        _mm256_extracti128_si256, _mm256_inserti128_si256, _mm256_loadu_si256, _mm256_mullo_epi16,
        _mm256_or_si256, _mm256_packus_epi16, _mm256_permute4x64_epi64, _mm256_set1_epi16,
        _mm256_set1_epi32, _mm256_srai_epi16, _mm256_storeu_si256,
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
        let luminance = u32::from(pixel_luminance(p));

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
        *pixel ^= 0x00FF_FFFF;
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
        *pixel ^= 0x00FF_FFFF;
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

    unsafe {
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
        let w_r = _mm256_set1_epi64x(0x0000_0192_0313_00C2);

        // Weights for Green
        // B*172 + G*702 -> W0=172(0xAC), W1=702(0x2BE)
        // R*357 + A*0   -> W2=357(0x165), W3=0
        let w_g = _mm256_set1_epi64x(0x0000_0165_02BE_00AC);

        // Weights for Blue
        // B*134 + G*547 -> W0=134(0x86), W1=547(0x223)
        // R*279 + A*0   -> W2=279(0x117), W3=0
        let w_b = _mm256_set1_epi64x(0x0000_0117_0223_0086);

        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);
        let max_val = _mm256_set1_epi32(255);

        let len = pixels.len();
        let simd_len = len & !7;
        let mut ptr = pixels.as_mut_ptr();
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

    unsafe {
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
}

/// Generates a deterministic pseudo-random kernel for SSAO sampling.
fn generate_kernel() -> [Vec3; KERNEL_SIZE] {
    let mut kernel = [Vec3::default(); KERNEL_SIZE];
    let mut seed = 123456789;

    for (i, v) in kernel.iter_mut().enumerate() {
        let r1 = rand_f32(&mut seed) * 2.0 - 1.0; // x: -1..1
        let r2 = rand_f32(&mut seed) * 2.0 - 1.0; // y: -1..1
        let r3 = rand_f32(&mut seed); // z: 0..1 (hemisphere)

        let mut sample = Vec3::new(r1, r2, r3).normalize();

        // Scale samples to distribute them within the hemisphere
        let scale = i as f32 / KERNEL_SIZE as f32;
        let scale = lerp(0.1, 1.0, scale * scale);
        sample = sample * scale;

        *v = sample;
    }

    kernel
}

/// Generates a noise texture for kernel rotation.
fn generate_noise() -> [Vec3; NOISE_SIZE * NOISE_SIZE] {
    let mut noise = [Vec3::default(); NOISE_SIZE * NOISE_SIZE];
    let mut seed = 987654321;

    for v in noise.iter_mut() {
        let x = rand_f32(&mut seed) * 2.0 - 1.0;
        let y = rand_f32(&mut seed) * 2.0 - 1.0;
        *v = Vec3::new(x, y, 0.0).normalize();
    }

    noise
}

/// Simple Linear Congruential Generator for deterministic randomness.
fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 9) as f32 / 8388607.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn box_blur_f32(src: &[f32], dest: &mut [f32], width: usize, height: usize) {
    let radius = 2; // 5x5 kernel

    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            let mut count = 0.0;

            for ky in -(radius as isize)..=radius as isize {
                for kx in -(radius as isize)..=radius as isize {
                    let ny = y as isize + ky;
                    let nx = x as isize + kx;

                    if ny >= 0 && ny < height as isize && nx >= 0 && nx < width as isize {
                        sum += src[ny as usize * width + nx as usize];
                        count += 1.0;
                    }
                }
            }
            dest[y * width + x] = sum / count;
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_ssao_avx2(
    occlusion_buffer: &mut [f32],
    zb: &ZBuffer,
    width: usize,
    height: usize,
    proj_m: &[f32; 16],
    kernel: &[Vec3],
    noise: &[Vec3],
    radius: f32,
    bias: f32,
    half_width: f32,
    half_height: f32,
) {
    use std::arch::x86_64::*;

    let p00 = _mm256_set1_ps(proj_m[0]);
    let p11 = _mm256_set1_ps(proj_m[5]);
    let p22 = _mm256_set1_ps(proj_m[10]);
    let p32 = _mm256_set1_ps(proj_m[14]);

    let radius_vec = _mm256_set1_ps(radius);
    let bias_vec = _mm256_set1_ps(bias);
    let half_w_vec = _mm256_set1_ps(half_width);
    let half_h_vec = _mm256_set1_ps(half_height);
    let one = _mm256_set1_ps(1.0);
    let zero = _mm256_setzero_ps();
    let minus_zero = _mm256_set1_ps(-0.0);

    let width_i = _mm256_set1_epi32(width as i32);
    let height_i = _mm256_set1_epi32(height as i32);
    let minus_one_i = _mm256_set1_epi32(-1);

    let zb_data = zb.as_slice();

    for y in 0..height {
        let y_idx = y * width;
        let mut x = 0;

        let noise_y = y % NOISE_SIZE;
        let noise_y_vec = _mm256_set1_epi32((noise_y * NOISE_SIZE) as i32);

        while x + 8 <= width {
            let depth_ptr = zb_data.as_ptr().add(y_idx + x);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Check valid depth (< 1.0)
            let mask_valid = _mm256_cmp_ps(depth_val, one, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask_valid) == 0 {
                _mm256_storeu_ps(occlusion_buffer.as_mut_ptr().add(y_idx + x), zero);
                x += 8;
                continue;
            }

            // Reconstruct View Z
            let denom = _mm256_add_ps(depth_val, p22);
            let z_view = _mm256_div_ps(_mm256_sub_ps(zero, p32), denom);

            // Reconstruct X, Y View
            let x_offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
            let x_base = _mm256_set1_ps(x as f32);
            let x_vals = _mm256_add_ps(x_base, x_offsets);

            let x_ndc = _mm256_sub_ps(_mm256_div_ps(x_vals, half_w_vec), one);
            let y_ndc = _mm256_sub_ps(one, _mm256_div_ps(_mm256_set1_ps(y as f32), half_h_vec));

            let neg_z_view = _mm256_sub_ps(zero, z_view);
            let x_view = _mm256_div_ps(_mm256_mul_ps(x_ndc, neg_z_view), p00);
            let y_view = _mm256_div_ps(_mm256_mul_ps(y_ndc, neg_z_view), p11);

            let mut occlusion = _mm256_setzero_ps();

            // Gather Noise
            let x_i = _mm256_set_epi32(
                x as i32 + 7, x as i32 + 6, x as i32 + 5, x as i32 + 4,
                x as i32 + 3, x as i32 + 2, x as i32 + 1, x as i32,
            );
            let noise_mask = _mm256_set1_epi32(3);
            let noise_x = _mm256_and_si256(x_i, noise_mask);
            let noise_idx = _mm256_add_epi32(noise_y_vec, noise_x);

            let idx_3 = _mm256_mullo_epi32(noise_idx, _mm256_set1_epi32(3)); // stride 3 floats (12 bytes)
            let noise_ptr = noise.as_ptr() as *const f32;
            // Gather X and Y components of noise
            let rx = _mm256_i32gather_ps(noise_ptr, idx_3, 4);
            let ry = _mm256_i32gather_ps(noise_ptr, _mm256_add_epi32(idx_3, _mm256_set1_epi32(1)), 4);

            for k in 0..KERNEL_SIZE {
                let s = kernel[k];
                let sx = _mm256_set1_ps(s.x);
                let sy = _mm256_set1_ps(s.y);
                let sz = _mm256_set1_ps(s.z);

                // Rotate sample
                let rot_x = _mm256_sub_ps(_mm256_mul_ps(sx, rx), _mm256_mul_ps(sy, ry));
                let rot_y = _mm256_add_ps(_mm256_mul_ps(sx, ry), _mm256_mul_ps(sy, rx));
                let rot_z = sz;

                let samp_x = _mm256_fmadd_ps(rot_x, radius_vec, x_view);
                let samp_y = _mm256_fmadd_ps(rot_y, radius_vec, y_view);
                let samp_z = _mm256_fmadd_ps(rot_z, radius_vec, z_view);

                // Project
                let clip_x = _mm256_mul_ps(samp_x, p00);
                let clip_y = _mm256_mul_ps(samp_y, p11);
                let clip_w = _mm256_sub_ps(zero, samp_z);

                let mask_w = _mm256_cmp_ps(clip_w, zero, _CMP_GT_OQ);
                let inv_w = _mm256_div_ps(one, clip_w);

                let ndc_x = _mm256_mul_ps(clip_x, inv_w);
                let ndc_y = _mm256_mul_ps(clip_y, inv_w);

                let s_x_f = _mm256_mul_ps(_mm256_add_ps(ndc_x, one), half_w_vec);
                let s_y_f = _mm256_mul_ps(_mm256_sub_ps(one, ndc_y), half_h_vec);

                let s_x = _mm256_cvttps_epi32(s_x_f);
                let s_y = _mm256_cvttps_epi32(s_y_f);

                // Bounds check
                let mask_x = _mm256_and_si256(_mm256_cmpgt_epi32(s_x, minus_one_i), _mm256_cmpgt_epi32(width_i, s_x));
                let mask_y = _mm256_and_si256(_mm256_cmpgt_epi32(s_y, minus_one_i), _mm256_cmpgt_epi32(height_i, s_y));
                let mask_bounds = _mm256_and_si256(mask_x, mask_y);

                let idx = _mm256_add_epi32(_mm256_mullo_epi32(s_y, width_i), s_x);

                // Gather depths with mask
                let existing_depth = _mm256_mask_i32gather_ps(one, zb_data.as_ptr(), idx, _mm256_castsi256_ps(mask_bounds), 4);

                let existing_z_denom = _mm256_add_ps(existing_depth, p22);
                let existing_view_z = _mm256_div_ps(_mm256_sub_ps(zero, p32), existing_z_denom);

                // Range check
                let dist = _mm256_andnot_ps(minus_zero, _mm256_sub_ps(existing_view_z, samp_z));
                let mask_range = _mm256_cmp_ps(dist, radius_vec, _CMP_LT_OQ);

                // Bias check
                let mask_bias = _mm256_cmp_ps(existing_view_z, _mm256_add_ps(samp_z, bias_vec), _CMP_GE_OQ);

                let mask_total = _mm256_and_ps(mask_w, _mm256_castsi256_ps(mask_bounds));
                let mask_total = _mm256_and_ps(mask_total, mask_range);
                let mask_total = _mm256_and_ps(mask_total, mask_bias);

                let contribution = _mm256_and_ps(mask_total, one);
                occlusion = _mm256_add_ps(occlusion, contribution);
            }

            let final_occ = _mm256_and_ps(occlusion, mask_valid);
            _mm256_storeu_ps(occlusion_buffer.as_mut_ptr().add(y_idx + x), final_occ);

            x += 8;
        }

        // Tail
        while x < width {
            let noise_x = x % NOISE_SIZE;
            let noise_idx = noise_y * NOISE_SIZE + noise_x;
            let random_vec = noise[noise_idx];

            let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

            if depth_val >= 1.0 {
                occlusion_buffer[y * width + x] = 0.0;
                x += 1;
                continue;
            }

            let z_view = -proj_m[14] / (depth_val + proj_m[10]);

            let x_ndc = (x as f32 / half_width) - 1.0;
            let y_ndc = 1.0 - (y as f32 / half_height);
            let x_view = x_ndc * (-z_view) / proj_m[0];
            let y_view = y_ndc * (-z_view) / proj_m[5];
            let pos_view = Vec3::new(x_view, y_view, z_view);

            let rx = random_vec.x;
            let ry = random_vec.y;

            let mut occlusion = 0.0;

            for k in 0..KERNEL_SIZE {
                let s = kernel[k];
                let rotated_sample = Vec3::new(s.x * rx - s.y * ry, s.x * ry + s.y * rx, s.z);
                let sample_pos = pos_view + rotated_sample * radius;

                // Scalar projection
                let (clip, w) = {
                    let x = sample_pos.x * proj_m[0] + sample_pos.y * proj_m[4] + sample_pos.z * proj_m[8] + proj_m[12];
                    let y = sample_pos.x * proj_m[1] + sample_pos.y * proj_m[5] + sample_pos.z * proj_m[9] + proj_m[13];
                    let z = sample_pos.x * proj_m[2] + sample_pos.y * proj_m[6] + sample_pos.z * proj_m[10] + proj_m[14];
                    let w = sample_pos.x * proj_m[3] + sample_pos.y * proj_m[7] + sample_pos.z * proj_m[11] + proj_m[15];
                    (Vec3::new(x, y, z), w)
                };

                if w > 0.0 {
                    let inv_w = 1.0 / w;
                    let s_ndc_x = clip.x * inv_w;
                    let s_ndc_y = clip.y * inv_w;
                    let s_screen_x = ((s_ndc_x + 1.0) * half_width) as i32;
                    let s_screen_y = ((1.0 - s_ndc_y) * half_height) as i32;

                    if s_screen_x >= 0 && s_screen_x < width as i32 && s_screen_y >= 0 && s_screen_y < height as i32 {
                        let idx = s_screen_y as usize * width + s_screen_x as usize;
                        let existing_depth = zb_data[idx];
                        let existing_view_z = -proj_m[14] / (existing_depth + proj_m[10]);
                        let sample_view_z = sample_pos.z;
                        if existing_view_z >= sample_view_z + bias && (existing_view_z - sample_view_z).abs() < radius {
                            occlusion += 1.0;
                        }
                    }
                }
            }
            occlusion_buffer[y * width + x] = occlusion;
            x += 1;
        }
    }
}

/// Applies Screen-Space Ambient Occlusion to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify (darkened by occlusion).
/// * `zb` - The depth buffer (source of geometry).
/// * `proj` - The projection matrix used to render the scene.
/// * `radius` - Sampling radius in view space (e.g., 0.5).
/// * `bias` - Bias to prevent self-occlusion (e.g., 0.025).
/// * `intensity` - Strength of the effect (e.g., 1.0 - 3.0).
pub fn apply_ssao(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    proj: &Mat4,
    radius: f32,
    bias: f32,
    intensity: f32,
) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let needed_size = width * height;

    SSAO_CONTEXT.with(|ctx_ref| {
        let mut ctx_guard = ctx_ref.borrow_mut();
        let ctx = &mut *ctx_guard;

        if !ctx.initialized {
            ctx.kernel = generate_kernel();
            ctx.noise = generate_noise();
            ctx.initialized = true;
        }

        if ctx.occlusion_buffer.len() < needed_size {
            ctx.occlusion_buffer.resize(needed_size, 0.0);
        }
        if ctx.scratch_buffer.len() < needed_size {
            ctx.scratch_buffer.resize(needed_size, 0.0);
        }

        let occlusion_buffer = &mut ctx.occlusion_buffer[..needed_size];
        let scratch_buffer = &mut ctx.scratch_buffer[..needed_size];
        let kernel = &ctx.kernel;
        let noise = &ctx.noise;

        // Projection parameters
        // Flatten matrix for SIMD
        let mut proj_flat = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                proj_flat[i * 4 + j] = proj.m[i][j];
            }
        }

        // Row-Major or Column-Major?
        // Mat4 is [row][col].
        // Flatten: m[0][0], m[0][1] ...
        // My SIMD code expects p00, p11 etc.
        // It mostly uses diagonal and last column.
        // Scalar fallback implementation in AVX2 function reconstructs manually.

        let _half_width = width as f32 * 0.5;
        let _half_height = height as f32 * 0.5;

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if std::is_x86_feature_detected!("avx2") {
                unsafe {
                    apply_ssao_avx2(
                        occlusion_buffer,
                        zb,
                        width,
                        height,
                        &proj_flat,
                        kernel,
                        noise,
                        radius,
                        bias,
                        half_width,
                        half_height,
                    );
                }
            } else {
                apply_ssao_scalar(occlusion_buffer, zb, proj, kernel, noise, width, height, radius, bias);
            }
        }
        #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
        apply_ssao_scalar(occlusion_buffer, zb, proj, kernel, noise, width, height, radius, bias);

        box_blur_f32(occlusion_buffer, scratch_buffer, width, height);

        let pixels = fb.as_mut_slice();
        for (i, p) in pixels.iter_mut().enumerate() {
            let occ = scratch_buffer[i];
            let factor = 1.0 - (occ / KERNEL_SIZE as f32) * intensity;
            let factor = factor.clamp(0.0, 1.0);

            let r = ((*p >> 16) & 0xFF) as f32;
            let g = ((*p >> 8) & 0xFF) as f32;
            let b = (*p & 0xFF) as f32;

            let new_r = (r * factor) as u32;
            let new_g = (g * factor) as u32;
            let new_b = (b * factor) as u32;

            *p = (*p & 0xFF00_0000) | (new_r << 16) | (new_g << 8) | new_b;
        }
    });
}

fn apply_ssao_scalar(
    occlusion_buffer: &mut [f32],
    zb: &ZBuffer,
    proj: &Mat4,
    kernel: &[Vec3],
    noise: &[Vec3],
    width: usize,
    height: usize,
    radius: f32,
    bias: f32,
) {
    // Projection parameters
    let p00 = proj.m[0][0];
    let p11 = proj.m[1][1];
    let p22 = proj.m[2][2];
    let p32 = proj.m[3][2];

    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for y in 0..height {
        let noise_y = y % NOISE_SIZE;
        for x in 0..width {
            let noise_x = x % NOISE_SIZE;
            let noise_idx = noise_y * NOISE_SIZE + noise_x;
            let random_vec = noise[noise_idx];

            let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

            if depth_val >= 1.0 {
                occlusion_buffer[y * width + x] = 0.0;
                continue;
            }

            // Reconstruct View Position
            let z_view = -p32 / (depth_val + p22);
            let x_ndc = (x as f32 / half_width) - 1.0;
            let y_ndc = 1.0 - (y as f32 / half_height);
            let x_view = x_ndc * (-z_view) / p00;
            let y_view = y_ndc * (-z_view) / p11;
            let pos_view = Vec3::new(x_view, y_view, z_view);

            let rx = random_vec.x;
            let ry = random_vec.y;

            let mut occlusion = 0.0;

            for k in 0..KERNEL_SIZE {
                let s = kernel[k];
                let rotated_sample = Vec3::new(s.x * rx - s.y * ry, s.x * ry + s.y * rx, s.z);

                let sample_pos = pos_view + rotated_sample * radius;
                let (sample_clip, sample_w) = proj.transform_point(sample_pos);

                if sample_w > 0.0 {
                    let inv_w = 1.0 / sample_w;
                    let s_ndc_x = sample_clip.x * inv_w;
                    let s_ndc_y = sample_clip.y * inv_w;

                    let s_screen_x = ((s_ndc_x + 1.0) * half_width) as i32;
                    let s_screen_y = ((1.0 - s_ndc_y) * half_height) as i32;

                    if s_screen_x >= 0
                        && s_screen_x < width as i32
                        && s_screen_y >= 0
                        && s_screen_y < height as i32
                    {
                        let existing_depth =
                            zb.get_depth(s_screen_x, s_screen_y).unwrap_or(1.0);
                        let existing_view_z = -p32 / (existing_depth + p22);
                        let sample_view_z = sample_pos.z;
                        let range_check = (existing_view_z - sample_view_z).abs() < radius;

                        if existing_view_z >= sample_view_z + bias && range_check {
                            occlusion += 1.0;
                        }
                    }
                }
            }

            occlusion_buffer[y * width + x] = occlusion;
        }
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
