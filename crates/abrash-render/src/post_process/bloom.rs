//! Bloom lighting effect.
//!
//! Extracts bright areas of the image and applies a blur to simulate light bleeding.

use super::blur::{box_blur_horizontal, box_blur_vertical};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::cell::RefCell;

thread_local! {
    static BLOOM_BUFFERS: RefCell<BloomContext> = RefCell::new(BloomContext::default());
}

#[derive(Default)]
struct BloomContext {
    bright_pixels: Vec<u32>,
    scratch_buffer: Vec<u32>,
    acc_buffer: Vec<i32>,
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
/// Configuration for the Bloom effect.
#[derive(Clone, Copy, Debug)]
pub struct BloomConfig {
    /// Luminance threshold for extracting bright pixels (0-255).
    pub threshold: u8,
    /// Radius of the box blur applied to the bright pixels.
    pub blur_radius: u32,
    /// Multiplier for the bloom intensity when blending.
    pub intensity: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            threshold: 200,
            blur_radius: 5,
            intensity: 1.0,
        }
    }
}

/// Applies a threshold-based bloom filter to the current framebuffer.
///
/// This simulates the physical phenomenon of light bleeding in camera lenses.
/// Bright pixels (intensity > 1.0 threshold) are extracted, heavily blurred,
/// and additively blended back onto the original image.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::post_process::bloom::{BloomConfig, apply_bloom};
///
/// let mut fb = Framebuffer::new(800, 600).unwrap();
/// let config = BloomConfig::default();
/// apply_bloom(&mut fb, &config);
/// ```
pub fn apply_bloom(fb: &mut Framebuffer, config: &BloomConfig) {
    if config.blur_radius == 0 || config.intensity <= 0.0 {
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
        extract_bright_pixels(pixels, bright_slice, config.threshold);

        // 2. Blur the bright pixels
        // Horizontal pass: bright_pixels -> scratch_buffer
        box_blur_horizontal(
            bright_slice,
            scratch_slice,
            width,
            height,
            config.blur_radius,
        );
        // Vertical pass: scratch_buffer -> bright_pixels
        box_blur_vertical(
            scratch_slice,
            bright_slice,
            acc_slice,
            width,
            height,
            config.blur_radius,
        );

        // 3. Composite back
        blend_additive(pixels, bright_slice, config.intensity);
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

    // /// Bolt Performance Optimization:
    // /// Precalculate luminance threshold to avoid per-pixel float/division math.
    // /// The `pixel_luminance` function uses the standard coefficients:
    // /// Y = 0.299*R + 0.587*G + 0.114*B, multiplied by 256 for fixed-point integer math.
    let threshold_u32 = u32::from(threshold);

    for (s, d) in src.iter().zip(dest.iter_mut()) {
        let val = *s;

        let r = (val >> 16) & 0xFF;
        let g = (val >> 8) & 0xFF;
        let b = val & 0xFF;

        // /// Bolt Performance Optimization:
        // /// Approximate luminance using integer arithmetic instead of pixel_luminance
        // /// Matches the standard formula: Y = (77*R + 150*G + 29*B) >> 8
        let luma = (77 * r + 150 * g + 29 * b) >> 8;

        if luma > threshold_u32 {
            *d = val;
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

        // ⚡ Bolt: SWAR (SIMD Within A Register) for per-pixel color scaling.
        // Process Red and Blue channels simultaneously to eliminate intermediate shifts.
        // Using u64 for intermediate math to prevent overflow when intensity_scale > 1.0 (>= 256).
        let d_rb = u64::from(d_val & 0x00FF_00FF);
        let d_g = u64::from(d_val & 0x0000_FF00);

        let s_rb = u64::from(s_val & 0x00FF_00FF);
        let s_g = u64::from(s_val & 0x0000_FF00);

        let intensity_scale_u64 = u64::from(intensity_scale);

        let s_rb_scaled = ((s_rb * intensity_scale_u64) >> 8) & 0x00FF_00FF;
        let s_g_scaled = ((s_g * intensity_scale_u64) >> 8) & 0x0000_FF00;

        // Extract scaled channels (safe to cast back to u32 as mask guarantees bounds)
        let r_s_scaled = (s_rb_scaled >> 16) as u32;
        let g_s_scaled = (s_g_scaled >> 8) as u32;
        let b_s_scaled = (s_rb_scaled & 0xFF) as u32;

        let r_d = ((d_rb >> 16) & 0xFF) as u32;
        let g_d = ((d_g >> 8) & 0xFF) as u32;
        let b_d = (d_rb & 0xFF) as u32;

        // Additive blend with saturation
        let r_new = (r_d + r_s_scaled).min(255);
        let g_new = (g_d + g_s_scaled).min(255);
        let b_new = (b_d + b_s_scaled).min(255);

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
            let packed = _mm256_packus_epi16(res_lo, res_hi);

            // Fix lane ordering:
            let permuted = _mm256_permute4x64_epi64(packed, 0xD8);

            // Restore Alpha
            let rgb_mask = _mm256_set1_epi32(0x00FF_FFFFu32 as i32);
            let rgb_result = _mm256_and_si256(permuted, rgb_mask);
            let final_result = _mm256_or_si256(rgb_result, d_alpha);

            _mm256_storeu_si256(d_ptr.cast(), final_result);

            d_ptr = d_ptr.add(8);
            s_ptr = s_ptr.add(8);
        }
    }
}
