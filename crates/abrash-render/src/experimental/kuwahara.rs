//! Kuwahara Filter Module
//!
//! A non-photorealistic post-processing filter that gives images a painterly,
//! oil-painting-like aesthetic while preserving hard edges.
//!
//! The filter works by calculating the mean and variance of colors in four
//! overlapping rectangular regions around each pixel. The pixel is then assigned
//! the mean color of the region with the lowest variance.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static KUWAHARA_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

#[allow(clippy::too_many_arguments)]

/// Applies a Kuwahara filter to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `radius` - The radius of the Kuwahara kernel (e.g., 2 means 5x5 total window size).
#[allow(clippy::too_many_arguments)]
fn process_kuwahara_region(
    src_fb: &[u32],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    dx_start: i32,
    dx_end: i32,
    dy_start: i32,
    dy_end: i32,
    radius: i32,
    best_num: &mut u64,
    best_den: &mut u64,
    best_color: &mut u32,
) {
    let mut sum_r = 0;
    let mut sum_g = 0;
    let mut sum_b = 0;
    let mut sum_r2 = 0;
    let mut sum_g2 = 0;
    let mut sum_b2 = 0;
    let mut count = 0;

    if y >= radius && y < height - radius && x >= radius && x < width - radius {
        let py_start = y + dy_start;
        let py_end = y + dy_end;
        let px_start = x + dx_start;
        let px_end = x + dx_end;

        for py in py_start..=py_end {
            let row_offset = (py * width) as usize;
            let px_start_u = px_start as usize;
            let px_end_u = px_end as usize;

            for &pixel in &src_fb[row_offset + px_start_u..=row_offset + px_end_u] {
                let r = (pixel >> 16) & 0xFF;
                let g = (pixel >> 8) & 0xFF;
                let b = pixel & 0xFF;

                sum_r += r;
                sum_g += g;
                sum_b += b;

                sum_r2 += r * r;
                sum_g2 += g * g;
                sum_b2 += b * b;

                count += 1;
            }
        }
    } else {
        let py_start = (y + dy_start).max(0).min(height - 1);
        let py_end = (y + dy_end).max(0).min(height - 1);
        let px_start = (x + dx_start).max(0).min(width - 1);
        let px_end = (x + dx_end).max(0).min(width - 1);

        for py in py_start..=py_end {
            let row_offset = (py * width) as usize;
            for px in px_start..=px_end {
                let pixel = src_fb[row_offset + px as usize];

                let r = (pixel >> 16) & 0xFF;
                let g = (pixel >> 8) & 0xFF;
                let b = pixel & 0xFF;

                sum_r += r;
                sum_g += g;
                sum_b += b;

                sum_r2 += r * r;
                sum_g2 += g * g;
                sum_b2 += b * b;

                count += 1;
            }
        }
    }

    if count > 0 {
        let count_u64 = u64::from(count);
        let sum_r_sq = u64::from(sum_r) * u64::from(sum_r);
        let sum_g_sq = u64::from(sum_g) * u64::from(sum_g);
        let sum_b_sq = u64::from(sum_b) * u64::from(sum_b);

        let scaled_var_r = count_u64 * u64::from(sum_r2) - sum_r_sq;
        let scaled_var_g = count_u64 * u64::from(sum_g2) - sum_g_sq;
        let scaled_var_b = count_u64 * u64::from(sum_b2) - sum_b_sq;

        let total_variance_num = scaled_var_r + scaled_var_g + scaled_var_b;
        let total_variance_den = count_u64 * count_u64;

        if u128::from(total_variance_num) * u128::from(*best_den)
            < u128::from(*best_num) * u128::from(total_variance_den)
        {
            *best_num = total_variance_num;
            *best_den = total_variance_den;

            let out_r = sum_r / count;
            let out_g = sum_g / count;
            let out_b = sum_b / count;

            *best_color = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
        }
    }
}

pub fn apply_kuwahara(fb: &mut Framebuffer, radius: i32) {
    if radius <= 0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // We must read from the original pixels and write to a new buffer
    // since the filter requires unmodified neighboring pixels.

    KUWAHARA_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = (width * height) as usize;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            dest_pixels
                .par_chunks_exact_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;

                    for (x_usize, pixel_out) in row.iter_mut().enumerate() {
                        let x = x_usize as i32;

                        // The four regions around the center pixel (x, y):
                        // 0: Top-Left
                        // 1: Top-Right
                        // 2: Bottom-Left
                        // 3: Bottom-Right
                        let mut best_num = u64::MAX;
                        let mut best_den = 1u64;
                        let mut best_color = 0u32;

                        let regions = [
                            (-radius, 0, -radius, 0),
                            (0, radius, -radius, 0),
                            (-radius, 0, 0, radius),
                            (0, radius, 0, radius),
                        ];

                        for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                            process_kuwahara_region(
                                src_fb,
                                width,
                                height,
                                x,
                                y,
                                dx_start,
                                dx_end,
                                dy_start,
                                dy_end,
                                radius,
                                &mut best_num,
                                &mut best_den,
                                &mut best_color,
                            );
                        }

                        *pixel_out = best_color;
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            dest_pixels
                .chunks_exact_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;

                    for (x_usize, pixel_out) in row.iter_mut().enumerate() {
                        let x = x_usize as i32;

                        // The four regions around the center pixel (x, y):
                        // 0: Top-Left
                        // 1: Top-Right
                        // 2: Bottom-Left
                        // 3: Bottom-Right
                        let mut best_num = u64::MAX;
                        let mut best_den = 1u64;
                        let mut best_color = 0u32;

                        let regions = [
                            (-radius, 0, -radius, 0),
                            (0, radius, -radius, 0),
                            (-radius, 0, 0, radius),
                            (0, radius, 0, radius),
                        ];

                        for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                            process_kuwahara_region(
                                src_fb,
                                width,
                                height,
                                x,
                                y,
                                dx_start,
                                dx_end,
                                dy_start,
                                dy_end,
                                radius,
                                &mut best_num,
                                &mut best_den,
                                &mut best_color,
                            );
                        }

                        *pixel_out = best_color;
                    }
                });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_kuwahara_bounds() {
        // Test with different framebuffer sizes to ensure no out-of-bounds panics
        let sizes = [(1, 1), (10, 10), (100, 1), (1, 100), (33, 47)];

        for (w, h) in sizes {
            let mut fb = Framebuffer::new(w, h).unwrap();
            fb.clear(0xFFFF_FFFF);

            // Should not panic with varying radii
            for radius in [1, 2, 5, 10] {
                apply_kuwahara(&mut fb, radius);
            }
        }
    }
}
