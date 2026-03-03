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
    let offset = offset as usize;

    // Optimization: Allocate the scratch buffer once instead of per-row or creating it inside the loop
    let mut row_buffer = vec![0u32; width];
    let pixels = fb.as_mut_slice();

    for row_pixels in pixels.chunks_exact_mut(width) {
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
