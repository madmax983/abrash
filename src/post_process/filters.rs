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
            let s_ptr = pixels.as_ptr();
            let d_ptr = lum_buffer.as_mut_ptr();

            let weights = _mm256_set1_epi64x(0x0000_004D_0096_001D);
            let perm_mask = _mm256_setr_epi32(0, 4, 1, 5, 2, 6, 3, 7);
            let _alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

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
