#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a separable box blur to a floating-point buffer.
///
/// # Arguments
/// * `src` - Source buffer (modified in-place to contain result).
/// * `dest` - Scratch buffer for intermediate horizontal pass.
/// * `acc_buffer` - Scratch buffer for vertical pass accumulators.
/// * `width` - Width of the buffer.
/// * `height` - Height of the buffer.
pub fn box_blur_f32(
    src: &mut [f32],
    dest: &mut [f32],
    acc_buffer: &mut [f32],
    width: usize,
    height: usize,
) {
    if width == 0 || height == 0 {
        return;
    }

    let radius = 2; // 5x5 kernel

    // 1. Horizontal pass: src -> dest
    box_blur_f32_horizontal_scalar(src, dest, width, height, radius);

    // 2. Vertical pass: dest -> src
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                box_blur_f32_vertical_avx2(dest, src, acc_buffer, width, height, radius);
            }
            return;
        }
    }
    box_blur_f32_vertical_scalar(dest, src, acc_buffer, width, height, radius);
}

fn box_blur_f32_horizontal_scalar(
    src: &[f32],
    dest: &mut [f32],
    width: usize,
    height: usize,
    radius: usize,
) {
    let scale = 1.0 / (radius as f32 * 2.0 + 1.0);

    // If width is too small, fallback to checked loop
    if width <= 2 * radius + 1 {
        for y in 0..height {
            let row_start = y * width;
            let src_row = &src[row_start..row_start + width];
            let dest_row = &mut dest[row_start..row_start + width];

            let mut acc = 0.0;
            let first = src_row[0];
            for _ in 0..=radius {
                acc += first;
            }
            for x in 1..=radius {
                acc += src_row[x.min(width - 1)];
            }

            for (x, dest_val) in dest_row.iter_mut().enumerate() {
                *dest_val = acc * scale;
                let out_idx = (x as isize - radius as isize).max(0) as usize;
                let in_idx = (x + radius + 1).min(width - 1);
                acc -= src_row[out_idx];
                acc += src_row[in_idx];
            }
        }
        return;
    }

    for y in 0..height {
        let row_start = y * width;
        let src_row = &src[row_start..row_start + width];
        let dest_row = &mut dest[row_start..row_start + width];

        let mut acc = 0.0;

        // Pre-fill
        let first = src_row[0];
        acc += first * (radius as f32 + 1.0);
        acc += src_row[1..=radius].iter().sum::<f32>();

        // Head: Left edge where outgoing pixel is clamped to 0
        for x in 0..radius {
            dest_row[x] = acc * scale;
            acc -= first;
            acc += src_row[x + radius + 1];
        }

        // Body: No clamping needed
        let limit = width - radius - 1;
        for x in radius..limit {
            dest_row[x] = acc * scale;
            acc -= src_row[x - radius];
            acc += src_row[x + radius + 1];
        }

        // Tail: Right edge where incoming pixel is clamped to width-1
        let last = src_row[width - 1];
        for x in limit..width {
            dest_row[x] = acc * scale;
            acc -= src_row[x - radius];
            acc += last;
        }
    }
}

fn box_blur_f32_vertical_scalar(
    src: &[f32],
    dest: &mut [f32],
    acc: &mut [f32],
    width: usize,
    height: usize,
    radius: usize,
) {
    let scale = 1.0 / (radius as f32 * 2.0 + 1.0);
    // Reset accumulators
    acc.fill(0.0);

    // Pre-fill accumulators
    let row0 = &src[0..width];
    for x in 0..width {
        let val = row0[x];
        for _ in 0..=radius {
            acc[x] += val;
        }
    }
    for y in 1..=radius {
        let row_idx = y.min(height - 1);
        let row = &src[row_idx * width..(row_idx + 1) * width];
        for x in 0..width {
            acc[x] += row[x];
        }
    }

    for y in 0..height {
        let dest_row_start = y * width;
        let dest_row = &mut dest[dest_row_start..dest_row_start + width];

        let out_y = (y as isize - radius as isize).max(0) as usize;
        let in_y = (y + radius + 1).min(height - 1);

        let out_row = &src[out_y * width..(out_y + 1) * width];
        let in_row = &src[in_y * width..(in_y + 1) * width];

        for x in 0..width {
            dest_row[x] = acc[x] * scale;
            acc[x] -= out_row[x];
            acc[x] += in_row[x];
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn box_blur_f32_vertical_avx2(
    src: &[f32],
    dest: &mut [f32],
    acc: &mut [f32],
    width: usize,
    height: usize,
    radius: usize,
) {
    use std::arch::x86_64::{
        _mm256_add_ps, _mm256_loadu_ps, _mm256_mul_ps, _mm256_set1_ps, _mm256_storeu_ps,
        _mm256_sub_ps,
    };

    unsafe {
        let scale = 1.0 / (radius as f32 * 2.0 + 1.0);
        let scale_vec = _mm256_set1_ps(scale);

        // Reset accumulators
        acc.fill(0.0);

        // Pre-fill accumulators (Scalar loop is fine here, it's O(W*R))
        // We could SIMD this too but it runs once per frame.
        let row0 = &src[0..width];
        for x in 0..width {
            let val = row0[x];
            for _ in 0..=radius {
                acc[x] += val;
            }
        }
        for y in 1..=radius {
            let row_idx = y.min(height - 1);
            let row = &src[row_idx * width..(row_idx + 1) * width];
            for x in 0..width {
                acc[x] += row[x];
            }
        }

        for y in 0..height {
            let dest_row_start = y * width;
            let dest_row = &mut dest[dest_row_start..dest_row_start + width];

            let out_y = (y as isize - radius as isize).max(0) as usize;
            let in_y = (y + radius + 1).min(height - 1);

            let out_row = &src[out_y * width..(out_y + 1) * width];
            let in_row = &src[in_y * width..(in_y + 1) * width];

            let mut x = 0;
            while x + 8 <= width {
                // Load
                let a = _mm256_loadu_ps(acc.as_ptr().add(x));
                let o = _mm256_loadu_ps(out_row.as_ptr().add(x));
                let i = _mm256_loadu_ps(in_row.as_ptr().add(x));

                // Update acc = acc - out + in
                let a_new = _mm256_add_ps(_mm256_sub_ps(a, o), i);
                _mm256_storeu_ps(acc.as_mut_ptr().add(x), a_new);

                // Calc dest
                let d = _mm256_mul_ps(a_new, scale_vec);
                _mm256_storeu_ps(dest_row.as_mut_ptr().add(x), d);

                x += 8;
            }

            // Tail
            for i in x..width {
                acc[i] = acc[i] - out_row[i] + in_row[i];
                dest_row[i] = acc[i] * scale;
            }
        }
    }
}

pub fn box_blur_horizontal(
    src: &[u32],
    dest: &mut [u32],
    width: usize,
    height: usize,
    radius: u32,
) {
    if width == 0 || height == 0 {
        return;
    }

    let radius = radius.min((width.max(height)) as u32);
    let radius = radius as usize;
    // Window size (kernel width)
    let kernel_size = (2 * radius + 1) as u64;
    // Fixed point scale factor (1.0 in 24.24 fixed point is 1<<24)
    // We add kernel_size / 2 for rounding in the division
    let scale = ((1 << 24) + kernel_size / 2) / kernel_size;
    let bias = 1 << 23; // 0.5 in fixed point for rounding

    #[cfg(feature = "parallel")]
    {
        // Suppress unused variable warning for height if parallel is active
        let _ = height;
        dest.par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, dst_row)| {
                let row_offset = y * width;
                let src_row = &src[row_offset..row_offset + width];
                process_row_horizontal(src_row, dst_row, width, radius, scale, bias);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            let row_offset = y * width;
            let src_row = &src[row_offset..row_offset + width];
            let dst_row = &mut dest[row_offset..row_offset + width];
            process_row_horizontal(src_row, dst_row, width, radius, scale, bias);
        }
    }
}

fn process_row_horizontal(
    src_row: &[u32],
    dst_row: &mut [u32],
    width: usize,
    radius: usize,
    scale: u64,
    bias: u64,
) {
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
        // Use u64 for multiplication to avoid overflow
        let r_avg = ((u64::from(r_acc) * scale + bias) >> 24) as u32;
        let g_avg = ((u64::from(g_acc) * scale + bias) >> 24) as u32;
        let b_avg = ((u64::from(b_acc) * scale + bias) >> 24) as u32;
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

pub fn box_blur_vertical(
    src: &[u32],
    dest: &mut [u32],
    acc_buffer: &mut [i32],
    width: usize,
    height: usize,
    radius: u32,
) {
    if width == 0 || height == 0 {
        return;
    }

    let radius = radius.min((width.max(height)) as u32);
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                box_blur_vertical_avx2(src, dest, acc_buffer, width, height, radius);
            }
            return;
        }
    }

    box_blur_vertical_scalar(src, dest, acc_buffer, width, height, radius);
}

fn box_blur_vertical_scalar(
    src: &[u32],
    dest: &mut [u32],
    acc_buffer: &mut [i32],
    width: usize,
    height: usize,
    radius: u32,
) {
    let radius = radius as usize;
    let kernel_size = 2 * radius + 1;
    let scale = 1.0 / (kernel_size as f32);

    // Split accumulators
    // We assume acc_buffer is size 3 * width
    let (r_acc, rest) = acc_buffer.split_at_mut(width);
    let (g_acc, b_acc) = rest.split_at_mut(width);

    // Reset accumulators
    r_acc.fill(0);
    g_acc.fill(0);
    b_acc.fill(0);

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
        r_acc[x] += r as i32 * (radius as i32 + 1);
        g_acc[x] += g as i32 * (radius as i32 + 1);
        b_acc[x] += b as i32 * (radius as i32 + 1);
    }

    for y in 1..=radius {
        let row_idx = y.min(height - 1);
        let row = &src[row_idx * width..(row_idx + 1) * width];
        for x in 0..width {
            let p = row[x];
            r_acc[x] += ((p >> 16) & 0xFF) as i32;
            g_acc[x] += ((p >> 8) & 0xFF) as i32;
            b_acc[x] += (p & 0xFF) as i32;
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

            r_acc[x] = r_acc[x] + ((p_in >> 16) & 0xFF) as i32 - ((p_out >> 16) & 0xFF) as i32;
            g_acc[x] = g_acc[x] + ((p_in >> 8) & 0xFF) as i32 - ((p_out >> 8) & 0xFF) as i32;
            b_acc[x] = b_acc[x] + (p_in & 0xFF) as i32 - (p_out & 0xFF) as i32;
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn box_blur_vertical_avx2(
    src: &[u32],
    dest: &mut [u32],
    acc_buffer: &mut [i32],
    width: usize,
    height: usize,
    radius: u32,
) {
    use std::arch::x86_64::{
        _mm256_add_epi32, _mm256_and_si256, _mm256_cvtepi32_ps, _mm256_cvttps_epi32,
        _mm256_loadu_si256, _mm256_mul_ps, _mm256_mullo_epi32, _mm256_or_si256, _mm256_set1_epi32,
        _mm256_set1_ps, _mm256_slli_epi32, _mm256_srli_epi32, _mm256_storeu_si256,
        _mm256_sub_epi32,
    };

    unsafe {
        let radius = radius as usize;
        let count = radius as i32 + 1;
        let kernel_size = 2 * radius + 1;
        let scale = 1.0 / (kernel_size as f32);
        let scale_vec = _mm256_set1_ps(scale);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        // Accumulators
        let (r_acc, rest) = acc_buffer.split_at_mut(width);
        let (g_acc, b_acc) = rest.split_at_mut(width);

        // Reset
        r_acc.fill(0);
        g_acc.fill(0);
        b_acc.fill(0);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_blur_zero_dimensions() {
        let mut f32_src: Vec<f32> = vec![];
        let mut f32_dest: Vec<f32> = vec![];
        let mut f32_acc: Vec<f32> = vec![];
        box_blur_f32(&mut f32_src, &mut f32_dest, &mut f32_acc, 0, 0);

        let u32_src: Vec<u32> = vec![];
        let mut u32_dest: Vec<u32> = vec![];
        let mut i32_acc: Vec<i32> = vec![];
        box_blur_horizontal(&u32_src, &mut u32_dest, 0, 0, 5);
        box_blur_vertical(&u32_src, &mut u32_dest, &mut i32_acc, 0, 0, 5);
    }

    #[test]
    fn test_box_blur_f32_correctness() {
        let width = 5;
        let height = 5;
        let mut src = vec![0.0; width * height];
        let mut dest = vec![0.0; width * height];
        let mut acc = vec![0.0; width];

        // Set center pixel to 25.0
        src[2 * width + 2] = 25.0;

        box_blur_f32(&mut src, &mut dest, &mut acc, width, height);

        // Check center
        assert!(
            (src[2 * width + 2] - 1.0).abs() < 1e-4,
            "Center pixel should be 1.0"
        );

        // Check corner (0,0)
        // Expected 1.0 with clamp-to-edge logic.
        let val = src[0];
        assert!(
            (val - 1.0).abs() < 1e-4,
            "Corner pixel mismatch. Got {}, expected 1.0",
            val
        );
    }
}
