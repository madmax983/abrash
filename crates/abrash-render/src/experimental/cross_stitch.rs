//! Cross Stitch Filter Module
//!
//! A retro-style post-processing filter that converts an image into a
//! digital cross-stitch pattern, rendering small colored 'X' shapes on a dark canvas.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Cross Stitch filter.
#[derive(Debug, Clone, Copy)]
pub struct CrossStitchConfig {
    /// Size of the stitch block (e.g., 8 pixels).
    pub block_size: usize,
    /// Thickness of the thread stroke.
    pub stroke_thickness: usize,
    /// Background color of the canvas fabric.
    pub canvas_color: u32,
}

impl Default for CrossStitchConfig {
    fn default() -> Self {
        Self {
            block_size: 10,
            stroke_thickness: 1,
            canvas_color: 0xFF_111111,
        }
    }
}

/// Applies a cross stitch effect to the framebuffer.
///
/// Divides the framebuffer into blocks. For each block, samples the center pixel,
/// fills the block with a canvas color, and draws an 'X' using the sampled color.
pub fn apply_cross_stitch(fb: &mut Framebuffer, config: &CrossStitchConfig) {
    if config.block_size <= 1 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let block_size = config.block_size;
    let stroke_thickness = config.stroke_thickness as i32;
    let canvas_color = config.canvas_color;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width * block_size).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width * block_size).enumerate();

    row_iter.for_each(|(_by, block_rows)| {
        for bx in 0..(width / block_size) {
            let x_start = bx * block_size;

            // Sample upper left pixel as color
            let center_x = x_start; // just take top-left corner
            let sample_color = block_rows[center_x]; // top row is row 0 in block_rows

            // Draw the block
            for iy in 0..block_size {
                let row_offset = iy * width;
                for ix in 0..block_size {
                    // Check if we are on the diagonals of the 'X'
                    // We can check if |ix - iy| <= thickness or |ix + iy - block_size + 1| <= thickness

                    let dx1 = (ix as i32 - iy as i32).abs();
                    let dx2 = (ix as i32 + iy as i32 - block_size as i32 + 1).abs();

                    let pixel_idx = row_offset + x_start + ix;

                    if dx1 <= stroke_thickness || dx2 <= stroke_thickness {
                        block_rows[pixel_idx] = sample_color;
                    } else {
                        block_rows[pixel_idx] = canvas_color;
                    }
                }
            }
        }
    });

    // Handle remaining edges if height is not divisible by block_size
    let remainder_y = height % block_size;
    if remainder_y > 0 {
        let start_y = height - remainder_y;
        let start_idx = start_y * width;
        let rest_pixels = &mut fb.as_mut_slice()[start_idx..];

        for bx in 0..(width / block_size) {
            let x_start = bx * block_size;
            let sample_color = rest_pixels[x_start]; // top row

            for iy in 0..remainder_y {
                let row_offset = iy * width;
                for ix in 0..block_size {
                    let dx1 = (ix as i32 - iy as i32).abs();
                    let dx2 = (ix as i32 + iy as i32 - block_size as i32 + 1).abs();

                    let pixel_idx = row_offset + x_start + ix;

                    if dx1 <= stroke_thickness || dx2 <= stroke_thickness {
                        rest_pixels[pixel_idx] = sample_color;
                    } else {
                        rest_pixels[pixel_idx] = canvas_color;
                    }
                }
            }
        }
    }

    // Handle remaining edges if width is not divisible by block_size
    let remainder_x = width % block_size;
    if remainder_x > 0 {
        let start_x = width - remainder_x;
        // Need to process full height
        let rest_pixels = fb.as_mut_slice();

        // Process full blocks on right edge
        for by in 0..(height / block_size) {
            let y_start = by * block_size;
            let sample_color = rest_pixels[y_start * width + start_x];

            for iy in 0..block_size {
                let row_offset = (y_start + iy) * width;
                for ix in 0..remainder_x {
                    let dx1 = (ix as i32 - iy as i32).abs();
                    let dx2 = (ix as i32 + iy as i32 - block_size as i32 + 1).abs();

                    let pixel_idx = row_offset + start_x + ix;

                    if dx1 <= stroke_thickness || dx2 <= stroke_thickness {
                        rest_pixels[pixel_idx] = sample_color;
                    } else {
                        rest_pixels[pixel_idx] = canvas_color;
                    }
                }
            }
        }

        // Process corner block
        if remainder_y > 0 {
            let start_y = height - remainder_y;
            let sample_color = rest_pixels[start_y * width + start_x];

            for iy in 0..remainder_y {
                let row_offset = (start_y + iy) * width;
                for ix in 0..remainder_x {
                    let dx1 = (ix as i32 - iy as i32).abs();
                    let dx2 = (ix as i32 + iy as i32 - block_size as i32 + 1).abs();

                    let pixel_idx = row_offset + start_x + ix;

                    if dx1 <= stroke_thickness || dx2 <= stroke_thickness {
                        rest_pixels[pixel_idx] = sample_color;
                    } else {
                        rest_pixels[pixel_idx] = canvas_color;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_cross_stitch() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFF_FFFFFF); // White background

        let config = CrossStitchConfig {
            block_size: 10,
            stroke_thickness: 1,
            canvas_color: 0xFF_000000,
        };

        apply_cross_stitch(&mut fb, &config);

        let top_left = fb.get_pixel(0, 0).unwrap();
        // Since block_size=10, thickness=1:
        // (0,0) is dx1 = 0 <= 1 => stroke
        // (0,1) is dx1 = 1 <= 1 => stroke
        // (0,2) is dx1 = 2 > 1. And dx2 = (0+2-10+1).abs() = 7 > 1 => canvas
        let middle_canvas = fb.get_pixel(0, 3).unwrap();

        assert_eq!(top_left, 0xFF_FFFFFF); // The 'X' stroke
        assert_eq!(middle_canvas, 0xFF_000000); // The canvas color
    }
}
