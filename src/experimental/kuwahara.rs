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
    let pixels = fb.as_slice();

    // We must read from the original pixels and write to a new buffer
    // since the filter requires unmodified neighboring pixels.
    let mut new_pixels = vec![0u32; (width * height) as usize];

    new_pixels
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
                        let count_f = count as f32;
                        let mean_r = sum_r as f32 / count_f;
                        let mean_g = sum_g as f32 / count_f;
                        let mean_b = sum_b as f32 / count_f;

                        let var_r = (sum_r2 as f32 / count_f) - (mean_r * mean_r);
                        let var_g = (sum_g2 as f32 / count_f) - (mean_g * mean_g);
                        let var_b = (sum_b2 as f32 / count_f) - (mean_b * mean_b);

                        // Total variance (luminance could also be used here, but sum of channel variances is simple)
                        let total_variance = var_r + var_g + var_b;

                        if total_variance < min_variance {
                            min_variance = total_variance;

                            let out_r = mean_r.clamp(0.0, 255.0) as u32;
                            let out_g = mean_g.clamp(0.0, 255.0) as u32;
                            let out_b = mean_b.clamp(0.0, 255.0) as u32;

                            best_color = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                        }
                    }
                }

                row[x as usize] = best_color;
            }
        });

    // Copy the filtered pixels back to the framebuffer
    fb.as_mut_slice().copy_from_slice(&new_pixels);
}
