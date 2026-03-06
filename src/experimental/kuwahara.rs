//! Kuwahara Filter Module
//!
//! A non-photorealistic post-processing filter that gives images a painterly,
//! oil-painting-like aesthetic while preserving hard edges.
//!
//! The filter works by calculating the mean and variance of colors in four
//! overlapping rectangular regions around each pixel. The pixel is then assigned
//! the mean color of the region with the lowest variance.

use crate::framebuffer::Framebuffer;
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
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
    let len = (width * height) as usize;

    // We must read from the original pixels and write back to the framebuffer
    // since the filter requires unmodified neighboring pixels.
    // Instead of re-allocating `new_pixels` every frame, we use a zero-cost thread_local buffer.
    BUFFER.with(|buffer| {
        let mut source_pixels = buffer.borrow_mut();

        if source_pixels.len() < len {
            source_pixels.resize(len, 0);
        }

        // Copy original pixels into the buffer so we can modify the framebuffer directly
        source_pixels[..len].copy_from_slice(fb.as_slice());

        let pixels = &source_pixels[..len];
        let new_pixels = fb.as_mut_slice();

        let process_row = |y_usize: usize, row: &mut [u32]| {
            let y = y_usize as i32;

            for x in 0..width {
                // The four regions around the center pixel (x, y):
                // 0: Top-Left
                // 1: Top-Right
                // 2: Bottom-Left
                // 3: Bottom-Right
                let mut min_variance = f32::MAX;
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
                            let pixel = unsafe { *pixels.get_unchecked(row_offset + px as usize) };

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
                        let total_variance = (scaled_var_r + scaled_var_g + scaled_var_b) as f32
                            / (count_u64 * count_u64) as f32;

                        if total_variance < min_variance {
                            min_variance = total_variance;

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
        };

        new_pixels
            .par_chunks_mut(width as usize)
            .enumerate()
            .for_each(|(y, row)| process_row(y, row));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_kuwahara_basic() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // Fill framebuffer with a pattern
        for y in 0..5 {
            for x in 0..5 {
                let color = if x < 2 && y < 2 {
                    0xFF00_00FF // Blue in top left
                } else if x >= 2 && y < 2 {
                    0xFFFF_0000 // Red in top right
                } else {
                    0xFF00_FF00 // Green bottom
                };
                unsafe {
                    fb.set_pixel_unchecked(x, y, color);
                }
            }
        }

        apply_kuwahara(&mut fb, 1);

        // Spot check pixels after Kuwahara filter
        // We expect regions to be somewhat preserved or smoothed
        let pixel_tl = unsafe { fb.get_pixel_unchecked(0, 0) };
        assert_eq!(pixel_tl, 0xFF00_00FF, "Top-left should stay blue");

        let pixel_tr = unsafe { fb.get_pixel_unchecked(4, 0) };
        assert_eq!(pixel_tr, 0xFFFF_0000, "Top-right should stay red");
    }

    #[test]
    fn test_apply_kuwahara_zero_radius() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        unsafe {
            fb.set_pixel_unchecked(1, 1, 0xFF123456);
        }

        apply_kuwahara(&mut fb, 0);

        // Should be unmodified
        assert_eq!(unsafe { fb.get_pixel_unchecked(1, 1) }, 0xFF123456);
    }
}
