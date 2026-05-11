//! Kuwahara Filter Post-Processing Effect
//!
//! Applies a non-linear Kuwahara filter to the framebuffer. This filter reduces
//! image noise and detail while preserving strong edges, resulting in a painterly,
//! watercolor, or oil-painting aesthetic.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Kuwahara effect.
#[derive(Debug, Clone, Copy)]
pub struct KuwaharaConfig {
    /// The radius of the filter. Larger values create a more pronounced
    /// painterly effect but are significantly more computationally expensive.
    /// Recommended values are between 2 and 6.
    pub radius: u32,
}

impl Default for KuwaharaConfig {
    fn default() -> Self {
        Self { radius: 3 }
    }
}

/// Applies a Kuwahara filter to the framebuffer.
///
/// This filter divides the area around each pixel into four overlapping regions.
/// It calculates the mean and variance of each region, and assigns the pixel
/// the mean color of the region with the lowest variance. This smooths out
/// textures while keeping sharp edges intact.
pub fn apply_kuwahara(fb: &mut Framebuffer, config: &KuwaharaConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let radius = config.radius as i32;

    if width == 0 || height == 0 || radius <= 0 {
        return;
    }

    SOURCE_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }
        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let y_i32 = y as i32;
            let width_i32 = width as i32;
            let height_i32 = height as i32;

            for (x, pixel) in row.iter_mut().enumerate() {
                let x_i32 = x as i32;

                // Define the bounds of the four regions (quadrants)
                // (x0, y0, x1, y1)
                let regions = [
                    (x_i32 - radius, y_i32 - radius, x_i32, y_i32), // Top-Left
                    (x_i32, y_i32 - radius, x_i32 + radius, y_i32), // Top-Right
                    (x_i32 - radius, y_i32, x_i32, y_i32 + radius), // Bottom-Left
                    (x_i32, y_i32, x_i32 + radius, y_i32 + radius), // Bottom-Right
                ];

                let mut min_variance = f32::MAX;
                let mut best_mean = (0.0, 0.0, 0.0);

                for (x0, y0, x1, y1) in &regions {
                    // Clamp regions to screen bounds
                    let start_x = (*x0).clamp(0, width_i32 - 1);
                    let start_y = (*y0).clamp(0, height_i32 - 1);
                    let end_x = (*x1).clamp(0, width_i32 - 1);
                    let end_y = (*y1).clamp(0, height_i32 - 1);

                    let num_pixels = ((end_x - start_x + 1) * (end_y - start_y + 1)) as f32;

                    if num_pixels == 0.0 {
                        continue;
                    }

                    // Calculate Mean
                    let mut sum_r = 0.0;
                    let mut sum_g = 0.0;
                    let mut sum_b = 0.0;

                    for ry in start_y..=end_y {
                        for rx in start_x..=end_x {
                            let p = src_pixels[(ry * width_i32 + rx) as usize];
                            sum_r += ((p >> 16) & 0xFF) as f32;
                            sum_g += ((p >> 8) & 0xFF) as f32;
                            sum_b += (p & 0xFF) as f32;
                        }
                    }

                    let mean_r = sum_r / num_pixels;
                    let mean_g = sum_g / num_pixels;
                    let mean_b = sum_b / num_pixels;

                    // Calculate Variance (we use a simple luminance-based variance for speed)
                    let mut variance = 0.0;

                    for ry in start_y..=end_y {
                        for rx in start_x..=end_x {
                            let p = src_pixels[(ry * width_i32 + rx) as usize];
                            let pr = ((p >> 16) & 0xFF) as f32;
                            let pg = ((p >> 8) & 0xFF) as f32;
                            let pb = (p & 0xFF) as f32;

                            // To keep it simple and relatively fast, we compute the variance
                            // as the sum of squared differences from the mean for each channel.
                            let dr = pr - mean_r;
                            let dg = pg - mean_g;
                            let db = pb - mean_b;

                            variance += dr * dr + dg * dg + db * db;
                        }
                    }

                    // No need to divide variance by num_pixels for comparison purposes,
                    // since regions at the edges might have different num_pixels, we should divide it.
                    variance /= num_pixels;

                    if variance < min_variance {
                        min_variance = variance;
                        best_mean = (mean_r, mean_g, mean_b);
                    }
                }

                // Apply the mean of the region with the lowest variance
                let final_r = best_mean.0.clamp(0.0, 255.0) as u32;
                let final_g = best_mean.1.clamp(0.0, 255.0) as u32;
                let final_b = best_mean.2.clamp(0.0, 255.0) as u32;

                *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_kuwahara() {
        let width = 5;
        let height = 5;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Create a noisy pattern
        for y in 0..height {
            for x in 0..width {
                // Alternating black and white pixels (high variance)
                let c = if (x + y) % 2 == 0 {
                    0xFF_FF_FF_FF
                } else {
                    0xFF_00_00_00
                };
                fb.set_pixel(x as i32, y as i32, c);
            }
        }

        // Make the top-left quadrant solid white (low variance)
        fb.set_pixel(0, 0, 0xFF_FF_FF_FF);
        fb.set_pixel(1, 0, 0xFF_FF_FF_FF);
        fb.set_pixel(0, 1, 0xFF_FF_FF_FF);
        fb.set_pixel(1, 1, 0xFF_FF_FF_FF);

        let config = KuwaharaConfig { radius: 1 };
        apply_kuwahara(&mut fb, &config);

        // Pixel at (1,1) should pick the top-left quadrant which is pure white and has 0 variance.
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF_FF_FF_FF);
    }
}
