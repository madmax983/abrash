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

/// Applies a Kuwahara filter to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `radius` - The radius of the Kuwahara kernel (e.g., 2 means 5x5 total window size).
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
                .par_chunks_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;

                    for x in 0..width {
                        // The four regions around the center pixel (x, y):
                        // 0: Top-Left
                        // 1: Top-Right
                        // 2: Bottom-Left
                        // 3: Bottom-Right

                        let mut best_num = u64::MAX;
                        let mut best_den = 1u64;
                        let mut best_color = 0u32;

                        // Region definitions (dx_start, dx_end, dy_start, dy_end)
                        let regions = [
                            (-radius, 0, -radius, 0), // Top-Left
                            (0, radius, -radius, 0),  // Top-Right
                            (-radius, 0, 0, radius),  // Bottom-Left
                            (0, radius, 0, radius),   // Bottom-Right
                        ];

                        for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                            let mut sum_r = 0;
                            let mut sum_g = 0;
                            let mut sum_b = 0;
                            let mut sum_r2 = 0;
                            let mut sum_g2 = 0;
                            let mut sum_b2 = 0;
                            let mut count = 0;

                            // Compute true y bounds
                            let py_start = (y + dy_start).max(0).min(height - 1);
                            let py_end = (y + dy_end).max(0).min(height - 1);
                            let px_start = (x + dx_start).max(0).min(width - 1);
                            let px_end = (x + dx_end).max(0).min(width - 1);

                            for py in py_start..=py_end {
                                let row_offset = (py * width) as usize;
                                for px in px_start..=px_end {
                                    // Using get_unchecked since we clamped
                                    let pixel =
                                        unsafe { *src_fb.get_unchecked(row_offset + px as usize) };

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

                            if count > 0 {
                                // Use integer math for variance to avoid per-pixel f32 casts
                                // Var = (sum(x^2)/n) - (sum(x)/n)^2
                                // Scaled_Var = n * sum(x^2) - sum(x)^2  (which equals n^2 * Var)
                                let count_u64 = u64::from(count);
                                let sum_r_sq = u64::from(sum_r) * u64::from(sum_r);
                                let sum_g_sq = u64::from(sum_g) * u64::from(sum_g);
                                let sum_b_sq = u64::from(sum_b) * u64::from(sum_b);

                                let scaled_var_r = count_u64 * u64::from(sum_r2) - sum_r_sq;
                                let scaled_var_g = count_u64 * u64::from(sum_g2) - sum_g_sq;
                                let scaled_var_b = count_u64 * u64::from(sum_b2) - sum_b_sq;

                                // Total variance (luminance could also be used here, but sum of channel variances is simple)
                                // Convert to f32 once per region to compare across potentially different count sizes near edges
                                let total_variance_num = scaled_var_r + scaled_var_g + scaled_var_b;
                                let total_variance_den = count_u64 * count_u64;

                                if u128::from(total_variance_num) * u128::from(best_den)
                                    < u128::from(best_num) * u128::from(total_variance_den)
                                {
                                    best_num = total_variance_num;
                                    best_den = total_variance_den;

                                    // Integer division is sufficient for the final mean
                                    let out_r = sum_r / count;
                                    let out_g = sum_g / count;
                                    let out_b = sum_b / count;

                                    best_color = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                                }
                            }
                        }

                        row[x as usize] = best_color;
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            dest_pixels
                .chunks_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;

                    for x in 0..width {
                        // The four regions around the center pixel (x, y):
                        // 0: Top-Left
                        // 1: Top-Right
                        // 2: Bottom-Left
                        // 3: Bottom-Right

                        let mut best_num = u64::MAX;
                        let mut best_den = 1u64;
                        let mut best_color = 0u32;

                        // Region definitions (dx_start, dx_end, dy_start, dy_end)
                        let regions = [
                            (-radius, 0, -radius, 0), // Top-Left
                            (0, radius, -radius, 0),  // Top-Right
                            (-radius, 0, 0, radius),  // Bottom-Left
                            (0, radius, 0, radius),   // Bottom-Right
                        ];

                        for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                            let mut sum_r = 0;
                            let mut sum_g = 0;
                            let mut sum_b = 0;
                            let mut sum_r2 = 0;
                            let mut sum_g2 = 0;
                            let mut sum_b2 = 0;
                            let mut count = 0;

                            // Compute true y bounds
                            let py_start = (y + dy_start).max(0).min(height - 1);
                            let py_end = (y + dy_end).max(0).min(height - 1);
                            let px_start = (x + dx_start).max(0).min(width - 1);
                            let px_end = (x + dx_end).max(0).min(width - 1);

                            for py in py_start..=py_end {
                                let row_offset = (py * width) as usize;
                                for px in px_start..=px_end {
                                    // Using get_unchecked since we clamped
                                    let pixel =
                                        unsafe { *src_fb.get_unchecked(row_offset + px as usize) };

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

                            if count > 0 {
                                // Use integer math for variance to avoid per-pixel f32 casts
                                // Var = (sum(x^2)/n) - (sum(x)/n)^2
                                // Scaled_Var = n * sum(x^2) - sum(x)^2  (which equals n^2 * Var)
                                let count_u64 = u64::from(count);
                                let sum_r_sq = u64::from(sum_r) * u64::from(sum_r);
                                let sum_g_sq = u64::from(sum_g) * u64::from(sum_g);
                                let sum_b_sq = u64::from(sum_b) * u64::from(sum_b);

                                let scaled_var_r = count_u64 * u64::from(sum_r2) - sum_r_sq;
                                let scaled_var_g = count_u64 * u64::from(sum_g2) - sum_g_sq;
                                let scaled_var_b = count_u64 * u64::from(sum_b2) - sum_b_sq;

                                // Total variance (luminance could also be used here, but sum of channel variances is simple)
                                // Convert to f32 once per region to compare across potentially different count sizes near edges
                                let total_variance_num = scaled_var_r + scaled_var_g + scaled_var_b;
                                let total_variance_den = count_u64 * count_u64;

                                if u128::from(total_variance_num) * u128::from(best_den)
                                    < u128::from(best_num) * u128::from(total_variance_den)
                                {
                                    best_num = total_variance_num;
                                    best_den = total_variance_den;

                                    // Integer division is sufficient for the final mean
                                    let out_r = sum_r / count;
                                    let out_g = sum_g / count;
                                    let out_b = sum_b / count;

                                    best_color = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                                }
                            }
                        }

                        row[x as usize] = best_color;
                    }
                });
        }
    });
}
