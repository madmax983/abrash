//! Post-processing effects.
//!
//! Functions to apply full-screen effects to a `Framebuffer`.
//!
//! # Examples
//!
//! ```
//! use abrash::framebuffer::Framebuffer;
//! use abrash::post_process::{apply_grayscale, apply_scanlines};
//!
//! let mut fb = Framebuffer::new(100, 100).unwrap();
//! // ... render something ...
//!
//! // Apply effects
//! apply_grayscale(&mut fb);
//! apply_scanlines(&mut fb);
//! ```

use crate::framebuffer::Framebuffer;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

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
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe {
                apply_grayscale_avx2(fb);
            }
            return;
        }
    }

    let pixels = fb.as_mut_slice();
    for pixel in pixels.iter_mut() {
        // Format: 0xAARRGGBB
        let p = *pixel;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        // Fixed-point luminance calculation
        let luminance = (77 * r + 150 * g + 29 * b) >> 8;

        // Preserve Alpha, set RGB to luminance
        *pixel = (p & 0xFF00_0000) | (luminance << 16) | (luminance << 8) | luminance;
    }
}

/// AVX2 optimized grayscale implementation
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn apply_grayscale_avx2(fb: &mut Framebuffer) {
    let pixels = fb.as_mut_slice();
    let mut i = 0;
    let len = pixels.len();

    // Constants for luminance calculation
    let w_r = _mm256_set1_epi32(77);
    let w_g = _mm256_set1_epi32(150);
    let w_b = _mm256_set1_epi32(29);
    let mask_ff = _mm256_set1_epi32(0xFF);
    let mask_alpha = _mm256_set1_epi32(0xFF000000u32 as i32);

    while i + 8 <= len {
        unsafe {
            // Load 8 pixels (32 bytes)
            let p = _mm256_loadu_si256(pixels.as_ptr().add(i) as *const __m256i);

            // Extract R, G, B components
            // R: (p >> 16) & 0xFF
            let r = _mm256_and_si256(_mm256_srli_epi32(p, 16), mask_ff);
            // G: (p >> 8) & 0xFF
            let g = _mm256_and_si256(_mm256_srli_epi32(p, 8), mask_ff);
            // B: p & 0xFF
            let b = _mm256_and_si256(p, mask_ff);

            // Calculate luminance: (77*R + 150*G + 29*B)
            let lum_r = _mm256_mullo_epi32(r, w_r);
            let lum_g = _mm256_mullo_epi32(g, w_g);
            let lum_b = _mm256_mullo_epi32(b, w_b);

            let sum = _mm256_add_epi32(lum_r, _mm256_add_epi32(lum_g, lum_b));
            let lum = _mm256_srli_epi32(sum, 8); // >> 8

            // Reconstruct pixel: (Alpha & p) | (lum << 16) | (lum << 8) | lum
            let p_alpha = _mm256_and_si256(p, mask_alpha);
            let p_lum_r = _mm256_slli_epi32(lum, 16);
            let p_lum_g = _mm256_slli_epi32(lum, 8);

            let result = _mm256_or_si256(
                p_alpha,
                _mm256_or_si256(p_lum_r, _mm256_or_si256(p_lum_g, lum))
            );

            // Store result
            _mm256_storeu_si256(pixels.as_mut_ptr().add(i) as *mut __m256i, result);
        }
        i += 8;
    }

    // Scalar fallback for remaining pixels
    for pixel in pixels[i..].iter_mut() {
        let p = *pixel;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;
        let luminance = (77 * r + 150 * g + 29 * b) >> 8;
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
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe {
                apply_scanlines_avx2(fb);
            }
            return;
        }
    }

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

/// AVX2 optimized scanlines implementation
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn apply_scanlines_avx2(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    let mask_7f = _mm256_set1_epi32(0x7F7F7F7F);
    let mask_alpha = _mm256_set1_epi32(0xFF000000u32 as i32);

    for y in (1..height).step_by(2) {
        let start = y * width;
        let end = start + width;
        let row = &mut pixels[start..end];
        let mut i = 0;
        let len = row.len();

        while i + 8 <= len {
             unsafe {
                 let p = _mm256_loadu_si256(row.as_ptr().add(i) as *const __m256i);
                 let shifted = _mm256_srli_epi32(p, 1);
                 let masked = _mm256_and_si256(shifted, mask_7f);
                 let alpha = _mm256_and_si256(p, mask_alpha);
                 let result = _mm256_or_si256(masked, alpha);
                 _mm256_storeu_si256(row.as_mut_ptr().add(i) as *mut __m256i, result);
             }
             i += 8;
        }

        // Scalar fallback
        for pixel in row[i..].iter_mut() {
            let p = *pixel;
            *pixel = ((p >> 1) & 0x7F7F_7F7F) | (p & 0xFF00_0000);
        }
    }
}

/// Inverts the colors of the framebuffer in-place.
///
/// Each color channel (R, G, B) is inverted (255 - value).
/// The alpha channel is preserved.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::apply_invert;
///
/// let mut fb = Framebuffer::new(1, 1).unwrap();
/// fb.set_pixel(0, 0, 0xFFFFFFFF); // White
/// apply_invert(&mut fb);
/// // Result should be black (0, 0, 0) with full alpha
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF000000);
/// ```
pub fn apply_invert(fb: &mut Framebuffer) {
    // Note: Manual AVX2 optimization was attempted but found to be slower (~8% regression)
    // than LLVM autovectorization for this simple XOR operation.
    // Memory bandwidth is the bottleneck here.
    let pixels = fb.as_mut_slice();
    for pixel in pixels.iter_mut() {
        // Invert RGB components, preserve Alpha
        // !pixel inverts all bits.
        // We want to keep the original alpha, so we mask it out from the inverted value
        // and combine it with the original alpha.
        // Or simpler: XOR with 0x00FFFFFF.
        *pixel ^= 0x00FFFFFF;
    }
}
