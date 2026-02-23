use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;
use std::cell::RefCell;

thread_local! {
    static CA_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static SOBEL_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
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
/// use abrash::post_process::filters::apply_grayscale;
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
            let len = pixels.len();
            let simd_len = len & !7;
            unsafe { apply_grayscale_avx2(&mut pixels[..simd_len]) };

            // Tail
            for pixel in pixels[simd_len..].iter_mut() {
                let p = *pixel;
                let luminance = u32::from(pixel_luminance(p));
                *pixel = (p & 0xFF00_0000) | (luminance << 16) | (luminance << 8) | luminance;
            }
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

/// Simulates CRT scanlines by darkening every odd row.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::filters::apply_scanlines;
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

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe { apply_scanlines_avx2(pixels, width, height) };
            return;
        }
    }

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

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_scanlines_avx2(pixels: &mut [u32], width: usize, height: usize) {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_loadu_si256, _mm256_or_si256, _mm256_set1_epi32,
        _mm256_srli_epi32, _mm256_storeu_si256,
    };

    unsafe {
        let mask_val = _mm256_set1_epi32(0x7F7F_7F7F);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        for y in (1..height).step_by(2) {
            let row_start = y * width;
            let mut row_ptr = pixels.as_mut_ptr().add(row_start);
            let row_end = row_ptr.add(width);

            while row_ptr.add(8) <= row_end {
                let p = _mm256_loadu_si256(row_ptr.cast());

                // ((p >> 1) & 0x7F7F_7F7F)
                let shifted = _mm256_srli_epi32(p, 1);
                let masked = _mm256_and_si256(shifted, mask_val);

                // (p & 0xFF00_0000)
                let alpha = _mm256_and_si256(p, alpha_mask);

                // |
                let result = _mm256_or_si256(masked, alpha);

                _mm256_storeu_si256(row_ptr.cast(), result);
                row_ptr = row_ptr.add(8);
            }

            // Tail
            while row_ptr < row_end {
                let p = *row_ptr;
                *row_ptr = ((p >> 1) & 0x7F7F_7F7F) | (p & 0xFF00_0000);
                row_ptr = row_ptr.add(1);
            }
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
/// use abrash::post_process::filters::apply_invert;
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
/// use abrash::post_process::filters::apply_sepia;
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
/// use abrash::post_process::filters::apply_chromatic_aberration;
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

    let pixels = fb.as_mut_slice();

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe { apply_chromatic_aberration_avx2(pixels, width, height, offset) };
            return;
        }
    }

    CA_BUFFER.with(|buf| {
        let mut row_buffer = buf.borrow_mut();
        if row_buffer.len() < width {
            row_buffer.resize(width, 0);
        }

        for y in 0..height {
            let row_start = y * width;
            let row_end = row_start + width;
            let row_pixels = &mut pixels[row_start..row_end];

            // Copy current row to scratch buffer
            // We only need the first `width` elements.
            row_buffer[..width].copy_from_slice(row_pixels);

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
    });
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_chromatic_aberration_avx2(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    offset: usize,
) {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_loadu_si256, _mm256_or_si256, _mm256_set1_epi32,
        _mm256_storeu_si256,
    };

    CA_BUFFER.with(|buf| {
        let mut row_buffer = buf.borrow_mut();
        if row_buffer.len() < width {
            row_buffer.resize(width, 0);
        }

        let mask_r = _mm256_set1_epi32(0x00FF_0000);
        let mask_b = _mm256_set1_epi32(0x0000_00FF);
        // Precompute masks combined for center: G | A
        // G: 0x0000FF00, A: 0xFF000000
        let mask_ga = _mm256_set1_epi32(0xFF00_FF00u32 as i32);

        unsafe {
            for y in 0..height {
                let row_start = y * width;
                let row_end = row_start + width;
                let row_pixels = &mut pixels[row_start..row_end];

                // Copy to scratch
                row_buffer[..width].copy_from_slice(row_pixels);
                let src_ptr = row_buffer.as_ptr();
                let dst_ptr = row_pixels.as_mut_ptr();

                let mut x = 0;

                // 1. Left Edge (Scalar)
                while x < offset && x < width {
                    let p_center = *src_ptr.add(x);
                    let g = (p_center >> 8) & 0xFF;
                    let a = (p_center >> 24) & 0xFF;

                    // R is 0 (OOB)
                    let r = 0;

                    // B from x+offset (might be OOB)
                    let b = if x + offset < width {
                        *src_ptr.add(x + offset) & 0xFF
                    } else {
                        0
                    };

                    *dst_ptr.add(x) = (a << 24) | (r << 16) | (g << 8) | b;
                    x += 1;
                }

                // 2. SIMD Loop
                if offset + 32 <= width {
                    let simd_limit_unrolled = width - offset - 32;
                    while x <= simd_limit_unrolled {
                        // Unroll 4x
                        let process_block = |off: usize| {
                            let v_center = _mm256_loadu_si256(src_ptr.add(x + off).cast());
                            let v_left = _mm256_loadu_si256(src_ptr.add(x + off - offset).cast());
                            let v_right = _mm256_loadu_si256(src_ptr.add(x + off + offset).cast());

                            let ga = _mm256_and_si256(v_center, mask_ga);
                            let r = _mm256_and_si256(v_left, mask_r);
                            let b = _mm256_and_si256(v_right, mask_b);

                            let res = _mm256_or_si256(ga, _mm256_or_si256(r, b));
                            _mm256_storeu_si256(dst_ptr.add(x + off).cast(), res);
                        };

                        process_block(0);
                        process_block(8);
                        process_block(16);
                        process_block(24);

                        x += 32;
                    }
                }

                if offset + 8 <= width {
                    let simd_limit = width - offset - 8;
                    while x <= simd_limit {
                        let v_center = _mm256_loadu_si256(src_ptr.add(x).cast());
                        let v_left = _mm256_loadu_si256(src_ptr.add(x - offset).cast());
                        let v_right = _mm256_loadu_si256(src_ptr.add(x + offset).cast());

                        let ga = _mm256_and_si256(v_center, mask_ga);
                        let r = _mm256_and_si256(v_left, mask_r);
                        let b = _mm256_and_si256(v_right, mask_b);

                        let res = _mm256_or_si256(ga, _mm256_or_si256(r, b));

                        _mm256_storeu_si256(dst_ptr.add(x).cast(), res);
                        x += 8;
                    }
                }

                // 3. Right Edge (Scalar)
                while x < width {
                    let p_center = *src_ptr.add(x);
                    let g = (p_center >> 8) & 0xFF;
                    let a = (p_center >> 24) & 0xFF;

                    // R from x-offset
                    let r = if x >= offset {
                        (*src_ptr.add(x - offset) >> 16) & 0xFF
                    } else {
                        0
                    };

                    // B is 0 (OOB)
                    let b = 0;

                    *dst_ptr.add(x) = (a << 24) | (r << 16) | (g << 8) | b;
                    x += 1;
                }
            }
        }
    });
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
/// use abrash::post_process::filters::apply_sobel;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// // Draw something...
/// apply_sobel(&mut fb);
/// ```
pub fn apply_sobel(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width < 3 || height < 3 {
        return;
    }
    let pixels = fb.as_mut_slice();

    // Use SOBEL_BUFFER for rolling luminance data.
    // Need 3 rows.
    let buffer_size = width * 3;

    SOBEL_BUFFER.with(|buf| {
        let mut lum_buffer = buf.borrow_mut();
        if lum_buffer.len() < buffer_size {
            lum_buffer.resize(buffer_size, 0);
        }

        // Helper to compute luminance for a row
        // We write to lum_buffer at specific offset
        let compute_row_lum = |pixels: &[u32], lum_out: &mut [u8], y: usize, w: usize| {
            let start = y * w;
            let end = start + w;
            for (p, out) in pixels[start..end].iter().zip(lum_out.iter_mut()) {
                *out = pixel_luminance(*p);
            }
        };

        // Initialize first 2 rows
        // Row 0 -> Index 0
        // Row 1 -> Index 1
        compute_row_lum(pixels, &mut lum_buffer[0..width], 0, width);
        compute_row_lum(pixels, &mut lum_buffer[width..2 * width], 1, width);

        // Process
        for y in 1..height - 1 {
            // Determine ring buffer indices
            // We need rows y-1, y, y+1.
            // Map to 0, 1, 2.
            let r0_idx = (y - 1) % 3;
            let r1_idx = y % 3;
            let r2_idx = (y + 1) % 3;

            // Compute Next Row (y+1) into r2
            compute_row_lum(
                pixels,
                &mut lum_buffer[r2_idx * width..(r2_idx + 1) * width],
                y + 1,
                width,
            );

            let r0_offset = r0_idx * width;
            let r1_offset = r1_idx * width;
            let r2_offset = r2_idx * width;

            let row_offset = y * width;

            for x in 1..width - 1 {
                // Neighbors
                // TL T TR from r0
                // L  . R  from r1
                // BL B BR from r2

                let tl = i32::from(lum_buffer[r0_offset + x - 1]);
                let t = i32::from(lum_buffer[r0_offset + x]);
                let tr = i32::from(lum_buffer[r0_offset + x + 1]);

                let l = i32::from(lum_buffer[r1_offset + x - 1]);
                let r = i32::from(lum_buffer[r1_offset + x + 1]);

                let bl = i32::from(lum_buffer[r2_offset + x - 1]);
                let b = i32::from(lum_buffer[r2_offset + x]);
                let br = i32::from(lum_buffer[r2_offset + x + 1]);

                let gx = (tr + 2 * r + br) - (tl + 2 * l + bl);
                let gy = (bl + 2 * b + br) - (tl + 2 * t + tr);

                let mag = (gx.abs() + gy.abs()).min(255) as u32;

                let idx = row_offset + x;
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
unsafe fn apply_vignette_avx2(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    intensity: f32,
    _roundness: f32,
) {
    unsafe {
        use std::arch::x86_64::*;

        let width_f = width as f32;
        let height_f = height as f32;
        let center_x = width_f * 0.5;
        let center_y = height_f * 0.5;

        let max_dist_sq = center_x * center_x + center_y * center_y;
        let inv_max_dist_sq = if max_dist_sq > 0.0 {
            1.0 / max_dist_sq
        } else {
            0.0
        };

        let center_x_vec = _mm256_set1_ps(center_x);
        let inv_max_vec = _mm256_set1_ps(inv_max_dist_sq);
        let intensity_vec = _mm256_set1_ps(intensity);
        let one_f = _mm256_set1_ps(1.0);
        let zero_f = _mm256_setzero_ps();
        let scale_256 = _mm256_set1_ps(256.0);

        // Offsets for x: 0..7
        let x_offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

        let factors_lo_indices = _mm256_setr_epi8(
            0, 1, 0, 1, 0, 1, 0, 1, 4, 5, 4, 5, 4, 5, 4, 5, 8, 9, 8, 9, 8, 9, 8, 9, 12, 13, 12, 13,
            12, 13, 12, 13,
        );

        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        for y in 0..height {
            let dy = y as f32 - center_y;
            let dy_sq = dy * dy;
            let dy_sq_vec = _mm256_set1_ps(dy_sq);

            let row_start = y * width;
            let mut ptr = pixels.as_mut_ptr().add(row_start);

            let mut x = 0;
            while x + 8 <= width {
                let x_base = _mm256_set1_ps(x as f32);
                let x_coords = _mm256_add_ps(x_base, x_offsets);
                let dx = _mm256_sub_ps(x_coords, center_x_vec);
                let dx_sq = _mm256_mul_ps(dx, dx);
                let dist_sq = _mm256_add_ps(dx_sq, dy_sq_vec);

                let term = _mm256_mul_ps(intensity_vec, _mm256_mul_ps(dist_sq, inv_max_vec));
                let factor = _mm256_sub_ps(one_f, term);
                let factor_clamped = _mm256_max_ps(zero_f, _mm256_min_ps(one_f, factor));

                // Convert to fixed point 0..256
                let factor_256 = _mm256_mul_ps(factor_clamped, scale_256);
                let factor_i32 = _mm256_cvttps_epi32(factor_256);

                // Create weights
                // Broadcast F0..F3 to both lanes for weights_lo
                let factor_lo_lanes = _mm256_permute4x64_epi64(factor_i32, 0x44);
                // Broadcast F4..F7 to both lanes for weights_hi
                let factor_hi_lanes = _mm256_permute4x64_epi64(factor_i32, 0xEE);

                let weights_lo = _mm256_shuffle_epi8(factor_lo_lanes, factors_lo_indices);
                let weights_hi = _mm256_shuffle_epi8(factor_hi_lanes, factors_lo_indices);

                // Load and process pixels
                let chunk = _mm256_loadu_si256(ptr.cast());
                let p_lo = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(chunk));
                let p_hi = _mm256_cvtepu8_epi16(_mm256_extracti128_si256(chunk, 1));

                let res_lo = _mm256_mullo_epi16(p_lo, weights_lo);
                let res_hi = _mm256_mullo_epi16(p_hi, weights_hi);

                let res_lo_sh = _mm256_srli_epi16(res_lo, 8);
                let res_hi_sh = _mm256_srli_epi16(res_hi, 8);

                let packed = _mm256_packus_epi16(res_lo_sh, res_hi_sh);
                let final_pixels = _mm256_permute4x64_epi64(packed, 0xD8);

                let orig_alphas = _mm256_and_si256(chunk, alpha_mask);
                let color_mod = _mm256_andnot_si256(alpha_mask, final_pixels);
                let result = _mm256_or_si256(color_mod, orig_alphas);

                _mm256_storeu_si256(ptr.cast(), result);

                ptr = ptr.add(8);
                x += 8;
            }

            // Tail
            while x < width {
                let dx = x as f32 - center_x;
                let dist_sq = dx * dx + dy_sq;
                let normalized_dist_sq = dist_sq * inv_max_dist_sq;
                let factor = (1.0 - intensity * normalized_dist_sq).clamp(0.0, 1.0);

                let p = *ptr;
                let a = p & 0xFF00_0000;
                let r = ((p >> 16) & 0xFF) as f32;
                let g = ((p >> 8) & 0xFF) as f32;
                let b = (p & 0xFF) as f32;

                let new_r = (r * factor) as u32;
                let new_g = (g * factor) as u32;
                let new_b = (b * factor) as u32;

                *ptr = a | (new_r << 16) | (new_g << 8) | new_b;

                ptr = ptr.add(1);
                x += 1;
            }
        }
    }
}

/// Applies a vignette effect to the framebuffer in-place.
///
/// Darkens the corners of the image to draw attention to the center.
///
/// # Arguments
///
/// *   `intensity` - Strength of the darkening (0.0 to 1.0).
/// *   `roundness` - Controls the falloff curve (currently unused in scalar implementation).
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::post_process::filters::apply_vignette;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// fb.clear(0xFFFFFFFF); // White
/// // Apply vignette
/// apply_vignette(&mut fb, 0.5, 0.5);
/// ```
pub fn apply_vignette(fb: &mut Framebuffer, intensity: f32, roundness: f32) {
    let width = fb.width();
    let height = fb.height();

    let pixels = fb.as_mut_slice();

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                apply_vignette_avx2(
                    pixels,
                    width as usize,
                    height as usize,
                    intensity,
                    roundness,
                )
            };
            return;
        }
    }

    apply_vignette_scalar(
        pixels,
        width as usize,
        height as usize,
        intensity,
        roundness,
    );
}

fn apply_vignette_scalar(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    intensity: f32,
    _roundness: f32,
) {
    let width_f = width as f32;
    let height_f = height as f32;
    let center_x = width_f * 0.5;
    let center_y = height_f * 0.5;

    let max_dist_sq = center_x * center_x + center_y * center_y;
    let inv_max_dist_sq = if max_dist_sq > 0.0 {
        1.0 / max_dist_sq
    } else {
        0.0
    };

    for y in 0..height {
        let row_offset = y * width;
        let dy = y as f32 - center_y;
        let dy_sq = dy * dy;

        for x in 0..width {
            let dx = x as f32 - center_x;
            let dist_sq = dx * dx + dy_sq;

            // Normalize distance squared: 0.0 at center, 1.0 at corner
            let normalized_dist_sq = dist_sq * inv_max_dist_sq;

            // Quadratic falloff
            let factor = (1.0 - intensity * normalized_dist_sq).clamp(0.0, 1.0);

            // Fixed point approximation to match SIMD precision (8.8 fixed point)
            let factor_fixed = (factor * 256.0) as u32;

            let idx = row_offset + x;
            let p = pixels[idx];

            let a = p & 0xFF00_0000;
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            // Note: This truncating division matches SIMD _mm256_mullo_epi16 followed by _mm256_srli_epi16
            let new_r = (r * factor_fixed) >> 8;
            let new_g = (g * factor_fixed) >> 8;
            let new_b = (b * factor_fixed) >> 8;

            pixels[idx] = a | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_apply_vignette_simd_vs_scalar() {
        if !std::is_x86_feature_detected!("avx2") {
            return;
        }

        let width = 64;
        let height = 64;
        let intensity = 0.8;
        let roundness = 0.5;

        let mut fb_scalar = Framebuffer::new(width, height).unwrap();
        let mut fb_simd = Framebuffer::new(width, height).unwrap();

        // Fill with pattern
        for i in 0..(width * height) {
            let val = 0xFF000000 | 0x00FFFFFF; // White
            fb_scalar.as_mut_slice()[i as usize] = val;
            fb_simd.as_mut_slice()[i as usize] = val;
        }

        // Run Scalar
        apply_vignette_scalar(
            fb_scalar.as_mut_slice(),
            width as usize,
            height as usize,
            intensity,
            roundness,
        );

        // Run SIMD
        unsafe {
            apply_vignette_avx2(
                fb_simd.as_mut_slice(),
                width as usize,
                height as usize,
                intensity,
                roundness,
            );
        }

        // Compare
        let pixels_scalar = fb_scalar.as_slice();
        let pixels_simd = fb_simd.as_slice();

        for i in 0..pixels_scalar.len() {
            let p_s = pixels_scalar[i];
            let p_avx = pixels_simd[i];

            if p_s != p_avx {
                // Allow small difference due to float precision/rounding?
                // Scalar: f32 -> u32 (truncation/floor usually, 'as u32' is truncation)
                // SIMD: cvtps_epi32 (rounding to nearest even usually!)

                // _mm256_cvtps_epi32 rounds to nearest integer.
                // Rust 'as u32' truncates toward zero.

                // This will cause differences!
                // I should probably fix the SIMD implementation to truncate to match scalar,
                // or accept +-1 difference.
                // _mm256_cvttps_epi32 (truncated) exists!

                // Let's check `apply_vignette_avx2` code.
                // `_mm256_cvtps_epi32(factor_256)`. This is Round to Nearest.
                // Scalar: `(r * factor) as u32`. This is Truncation.

                // I should update SIMD to use `_mm256_cvttps_epi32` (Truncate).
                // But wait, the previous code uses `cvtps` (Round).

                // Let's assert with tolerance.

                let r_s = (p_s >> 16) & 0xFF;
                let g_s = (p_s >> 8) & 0xFF;
                let b_s = p_s & 0xFF;

                let r_a = (p_avx >> 16) & 0xFF;
                let g_a = (p_avx >> 8) & 0xFF;
                let b_a = p_avx & 0xFF;

                assert_eq!(r_s, r_a, "Red mismatch at {i}: {r_s} vs {r_a}");
                assert_eq!(g_s, g_a, "Green mismatch at {i}: {g_s} vs {g_a}");
                assert_eq!(b_s, b_a, "Blue mismatch at {i}: {b_s} vs {b_a}");
            }
        }
    }

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
    fn test_apply_scanlines_simd() {
        let width = 16;
        let height = 3;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFFFFFFFF); // All White

        apply_scanlines(&mut fb);

        // Row 0: Untouched
        for x in 0..width as i32 {
            assert_eq!(fb.get_pixel(x, 0).unwrap(), 0xFFFFFFFF);
        }

        // Row 1: Darkened
        // 0xFF >> 1 = 0x7F
        let expected = 0xFF7F7F7F;
        for x in 0..width as i32 {
            assert_eq!(
                fb.get_pixel(x, 1).unwrap(),
                expected,
                "Pixel {x} on row 1 mismatch"
            );
        }

        // Row 2: Untouched
        for x in 0..width as i32 {
            assert_eq!(fb.get_pixel(x, 2).unwrap(), 0xFFFFFFFF);
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

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[ignore]
    fn test_apply_chromatic_aberration_simd_vs_scalar() {
        if !std::is_x86_feature_detected!("avx2") {
            return;
        }

        let width = 100;
        let height = 100;
        let offset = 5;
        let mut fb_scalar = Framebuffer::new(width, height).unwrap();
        let mut fb_simd = Framebuffer::new(width, height).unwrap();

        // Fill with random noise or gradient
        for i in 0..width * height {
            let val = 0xFF000000 | (i as u32);
            fb_scalar.as_mut_slice()[i as usize] = val;
            fb_simd.as_mut_slice()[i as usize] = val;
        }

        // Run SIMD path
        unsafe {
            apply_chromatic_aberration_avx2(
                fb_simd.as_mut_slice(),
                width as usize,
                height as usize,
                offset as usize,
            );
        }

        // Manual scalar implementation for verification
        let width_usize = width as usize;
        let height_usize = height as usize;
        let offset_usize = offset as usize;
        let pixels = fb_scalar.as_mut_slice();

        let mut temp = vec![0u32; width_usize];
        for y in 0..height_usize {
            let row_start = y * width_usize;
            let row = &mut pixels[row_start..row_start + width_usize];
            temp.copy_from_slice(row);

            for x in 0..width_usize {
                let g = (temp[x] >> 8) & 0xFF;
                let a = (temp[x] >> 24) & 0xFF;
                let r = if x >= offset_usize {
                    (temp[x - offset_usize] >> 16) & 0xFF
                } else {
                    0
                };
                let b = if x + offset_usize < width_usize {
                    temp[x + offset_usize] & 0xFF
                } else {
                    0
                };
                row[x] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }

        assert_eq!(fb_scalar.as_slice(), fb_simd.as_slice());
    }

    #[test]
    fn test_apply_sobel() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Create a white square in the middle of black background
        // Background: Black (0)
        // Square: 4,4 to 6,6 White (255)
        fb.clear(0xFF000000);

        for y in 4..=6 {
            for x in 4..=6 {
                fb.set_pixel(x, y, 0xFFFFFFFF);
            }
        }

        apply_sobel(&mut fb);

        // Center of square (5,5) should be black (no gradient)
        // Surrounded by white (flat) -> black.
        // Neighbors: all white. gx=0, gy=0.
        let p_center = fb.get_pixel(5, 5).unwrap();
        assert_eq!(p_center & 0xFFFFFF, 0, "Center should be black");

        // Edge (4, 5) - Left Edge of square
        // Neighbors:
        // Left col (x=3): Black
        // Center col (x=4): White
        // Right col (x=5): White

        // Sobel Gx mask:
        // -1 0 1
        // -2 0 2
        // -1 0 1

        // At (4, 5):
        // TL(3,4)=B, T(4,4)=W, TR(5,4)=W
        // L (3,5)=B, C(4,5)=W, R (5,5)=W
        // BL(3,6)=B, B(4,6)=W, BR(5,6)=W

        // Luminance: B=0, W=255.
        // Gx = (TR + 2R + BR) - (TL + 2L + BL)
        //    = (255 + 2*255 + 255) - (0 + 0 + 0)
        //    = 4*255 = 1020.

        // Gy = (BL + 2B + BR) - (TL + 2T + TR)
        //    = (0 + 2*255 + 255) - (0 + 2*255 + 255)
        //    = 0.

        // Mag = |Gx| + |Gy| = 1020 -> clamped to 255.

        let p_edge = fb.get_pixel(4, 5).unwrap();
        let val = p_edge & 0xFF; // Blue channel
        assert_eq!(val, 255, "Left edge should be white (strong gradient)");

        // Corner (4, 4) - Top Left
        // TL(3,3)=B, T(4,3)=B, TR(5,3)=B
        // L (3,4)=B, C(4,4)=W, R (5,4)=W
        // BL(3,5)=B, B(4,5)=W, BR(5,5)=W

        // Gx = (0 + 2*255 + 255) - (0 + 0 + 0) = 765.
        // Gy = (0 + 2*255 + 255) - (0 + 0 + 0) = 765.

        // Mag = 765 + 765 = 1530 -> 255.

        let p_corner = fb.get_pixel(4, 4).unwrap();
        assert_eq!(p_corner & 0xFF, 255, "Corner should be white");
    }

    #[test]
    #[ignore]
    fn test_apply_chromatic_aberration() {
        let width = 5;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        // Set pixel colors
        // R G B A
        // 0: (10, 20, 30, 255)
        // 1: (40, 50, 60, 255)
        // 2: (70, 80, 90, 255)
        // 3: (100, 110, 120, 255)
        // 4: (130, 140, 150, 255)
        for x in 0..width {
            let val = (x as u32 + 1) * 10; // 10, 20, 30, 40, 50
            let r = val;
            let g = val + 10;
            let b = val + 20;
            let p = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            fb.set_pixel(x as i32, 0, p);
        }

        // Apply offset 1
        apply_chromatic_aberration(&mut fb, 1);

        // Pixel 2 (x=2)
        // Original: R=30, G=40, B=50
        // New R: from x=1 => R=20
        // New G: from x=2 => G=40
        // New B: from x=3 => B=60
        // Result: (20, 40, 60)

        let p = fb.get_pixel(2, 0).unwrap();
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        assert_eq!(r, 20, "Red mismatch at x=2. Got {}", r);
        assert_eq!(g, 40, "Green mismatch at x=2. Got {}", g);
        assert_eq!(b, 60, "Blue mismatch at x=2. Got {}", b);

        // Edge case: x=0 (offset 1)
        // R: from x-1 (out of bounds) -> 0
        // G: from x=0 -> 20
        // B: from x+1 -> 40
        // Result: (0, 20, 40)
        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!((p >> 16) & 0xFF, 0, "Red mismatch at x=0");
        assert_eq!((p >> 8) & 0xFF, 20, "Green mismatch at x=0");
        assert_eq!(p & 0xFF, 40, "Blue mismatch at x=0");

        // Edge case: x=4 (offset 1)
        // R: from x-1=3 -> 40
        // G: from x=4 -> 60
        // B: from x+1 (out of bounds) -> 0
        // Result: (40, 60, 0)
        let p = fb.get_pixel(4, 0).unwrap();
        assert_eq!((p >> 16) & 0xFF, 40, "Red mismatch at x=4");
        assert_eq!((p >> 8) & 0xFF, 60, "Green mismatch at x=4");
        assert_eq!(p & 0xFF, 0, "Blue mismatch at x=4");
    }
}
