//! Cross-Stitch Filter
//!
//! A retro-style post-processing effect that converts the framebuffer into a pattern resembling a cross-stitch embroidery canvas.
//! Groups pixels into blocks, samples the color, and draws an 'X' using absolute differences, backed by a canvas color.

use crate::framebuffer::Framebuffer;

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
                    let start = (by * width) + cx;
                    paint_stitch_row(
                        &mut row_buffer[start..start + block_width],
                        by,
                        b_size,
                        sample_color,
                        config.canvas_color,
                    );
                }
            }

            block_rows.copy_from_slice(&row_buffer);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        // One sample per cell is read before the band is filled with canvas, then
        // only the stitch runs of each cell row are painted over the fill.
        let mut samples: Vec<u32> = Vec::with_capacity(width.div_ceil(b_size));
        for y in (0..height).step_by(b_size) {
            let block_height = std::cmp::min(b_size, height - y);
            let row_start = y * width;

            samples.clear();
            samples.extend(pixels[row_start..row_start + width].iter().step_by(b_size));

            pixels[row_start..row_start + block_height * width].fill(config.canvas_color);

            for (cell, &sample_color) in samples.iter().enumerate() {
                let x = cell * b_size;
                let block_width = std::cmp::min(b_size, width - x);

                for by in 0..block_height {
                    let start = row_start + (by * width) + x;
                    paint_stitch_runs(
                        &mut pixels[start..start + block_width],
                        by,
                        b_size,
                        sample_color,
                    );
                }
            }
        }
    }
}

/// Paints one row (`by`) of an 'X' cross-stitch cell into `row`.
///
/// A pixel at column `bx` is stitched when `|bx - by| <= 1` or
/// `|bx - (b_size - 1 - by)| <= 1`; those are two runs of at most three pixels,
/// so the row is filled with the canvas color and the runs are filled with the
/// sample color instead of evaluating the predicate per pixel.
#[inline]
fn paint_stitch_row(row: &mut [u32], by: usize, b_size: usize, sample: u32, canvas: u32) {
    row.fill(canvas);
    paint_stitch_runs(row, by, b_size, sample);
}

/// Paints only the stitch runs of row `by` (see [`paint_stitch_row`]) into `row`.
#[inline]
fn paint_stitch_runs(row: &mut [u32], by: usize, b_size: usize, sample: u32) {
    let mirror = b_size - 1 - by;
    for center in [by, mirror] {
        // Columns center-1 ..= center+1, clipped to the row.
        for col in [center.wrapping_sub(1), center, center + 1] {
            if let Some(p) = row.get_mut(col) {
                *p = sample;
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

    #[test]
    fn test_cross_stitch_matches_reference_on_partial_cells() {
        for &(w, h, cell) in &[(23u32, 17u32, 5u32), (10, 10, 3), (31, 9, 8), (7, 40, 10)] {
            let mut fb = Framebuffer::new(w, h).unwrap();
            for y in 0..h {
                for x in 0..w {
                    fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | (y * 977 + x * 131 + 1));
                }
            }
            let src = fb.as_slice().to_vec();
            let canvas = 0xFF_DDDDDD;
            apply_cross_stitch(
                &mut fb,
                &CrossStitchConfig {
                    cell_size: cell,
                    canvas_color: canvas,
                },
            );

            let (w, h, b) = (w as usize, h as usize, cell as usize);
            for y in 0..h {
                for x in 0..w {
                    let (bx, by) = (x % b, y % b);
                    let sample = src[(y - by) * w + (x - bx)];
                    let df = (bx as isize - by as isize).abs();
                    let db = (bx as isize - (b as isize - 1 - by as isize)).abs();
                    let want = if df <= 1 || db <= 1 { sample } else { canvas };
                    assert_eq!(fb.as_slice()[y * w + x], want, "({x},{y}) {w}x{h} cell {b}");
                }
            }
        }
    }
}
