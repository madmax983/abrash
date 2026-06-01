//! Cross-Stitch Filter
//!
//! A retro-style post-processing effect that converts the framebuffer into a pattern resembling a cross-stitch embroidery canvas.
//! Groups pixels into blocks, samples the color, and draws an 'X' using absolute differences, backed by a canvas color.

use abrash_core::framebuffer::Framebuffer;

/// Configuration for the Cross-Stitch filter.
#[derive(Debug, Clone, Copy)]
pub struct CrossStitchConfig {
    /// Size of each cross-stitch cell in pixels. Minimum is 3.
    pub cell_size: u32,
    /// The background color to act as the canvas underneath the stitches.
    pub canvas_color: u32,
}

impl Default for CrossStitchConfig {
    fn default() -> Self {
        Self {
            cell_size: 8,
            canvas_color: 0xFF_DDDDDD,
        }
    }
}

/// Applies a cross-stitch effect to the framebuffer.
///
/// Divides the framebuffer into cells. For each cell, the top-left pixel color is sampled,
/// the block is cleared to `canvas_color`, and an 'X' is drawn in the block using the sampled color.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Settings for the effect.
pub fn apply_cross_stitch(fb: &mut Framebuffer, config: &CrossStitchConfig) {
    if config.cell_size < 3 {
        return;
    }

    let width = fb.width() as usize;
    if width == 0 {
        return;
    }

    let height = fb.height() as usize;
    if height == 0 {
        return;
    }

    let b_size = config.cell_size as usize;
    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        let chunk_size = width * b_size;

        pixels.par_chunks_mut(chunk_size).for_each(|block_rows| {
            let block_height = block_rows.len() / width;
            if block_height == 0 {
                return;
            }

            // Temporary buffer to hold the rendered cell row
            // We allocate a buffer for an entire row of cells (width * block_height)
            let mut row_buffer = vec![config.canvas_color; width * block_height];

            for cx in (0..width).step_by(b_size) {
                let block_width = std::cmp::min(b_size, width - cx);
                // Sample color from upper-left pixel
                let sample_color = block_rows[cx];

                // Render the cross stitch into the row_buffer
                for by in 0..block_height {
                    for bx in 0..block_width {
                        let diff_forward = (bx as isize - by as isize).abs();
                        let diff_backward =
                            (bx as isize - (b_size as isize - 1 - by as isize)).abs();

                        // Simple 'X' shape
                        if diff_forward <= 1 || diff_backward <= 1 {
                            let target_idx = (by * width) + cx + bx;
                            row_buffer[target_idx] = sample_color;
                        }
                    }
                }
            }

            block_rows.copy_from_slice(&row_buffer);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in (0..height).step_by(b_size) {
            let block_height = std::cmp::min(b_size, height - y);
            let row_start = y * width;

            for x in (0..width).step_by(b_size) {
                let block_width = std::cmp::min(b_size, width - x);
                let sample_color = pixels[row_start + x];

                for by in 0..block_height {
                    for bx in 0..block_width {
                        let target_idx = row_start + (by * width) + x + bx;
                        let diff_forward = (bx as isize - by as isize).abs();
                        let diff_backward =
                            (bx as isize - (b_size as isize - 1 - by as isize)).abs();

                        if diff_forward <= 1 || diff_backward <= 1 {
                            pixels[target_idx] = sample_color;
                        } else {
                            pixels[target_idx] = config.canvas_color;
                        }
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
    fn test_cross_stitch_filter() {
        let mut fb = Framebuffer::new(8, 8).unwrap();
        fb.clear(0xFF_FF0000); // Red

        let config = CrossStitchConfig {
            cell_size: 8,
            canvas_color: 0xFF_FFFFFF,
        };
        apply_cross_stitch(&mut fb, &config);

        // Check top-left (part of X)
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF_FF0000));
        // Check middle empty space (should be canvas)
        assert_eq!(fb.get_pixel(0, 3), Some(0xFF_FFFFFF));
        // Check center (part of X)
        assert_eq!(fb.get_pixel(4, 4), Some(0xFF_FF0000));
    }
}
