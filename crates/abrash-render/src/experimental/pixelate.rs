//! Pixelate Post-Processing Filter
//!
//! A retro-style filter that reduces the perceived resolution of the framebuffer
//! by grouping pixels into blocks of a specified size.

use crate::framebuffer::Framebuffer;

/// Applies a pixelate effect to the framebuffer.
///
/// Divides the framebuffer into blocks of `block_size` x `block_size`.
/// Each block is filled entirely with the color of its upper-left pixel.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `block_size` - The size of the pixelation blocks. A size of 0 or 1 has no effect.
pub fn apply_pixelate(fb: &mut Framebuffer, block_size: u32) {
    if block_size <= 1 {
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

    let b_size = block_size as usize;
    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;

        let chunk_size = width * b_size;

        pixels
            .par_chunks_exact_mut(chunk_size)
            .for_each(|block_rows| {
                let block_height = block_rows.len() / width;
                if block_height == 0 {
                    return;
                }

                // Process the first row
                for x in (0..width).step_by(b_size) {
                    let block_width = std::cmp::min(b_size, width - x);
                    let color = block_rows[x];
                    block_rows[x..x + block_width].fill(color);
                }

                // Copy the first row to the rest of the block rows
                if block_height > 1 {
                    let (first_row_region, rest) = block_rows.split_at_mut(width);
                    for by in 1..block_height {
                        let dest_start = (by - 1) * width;
                        rest[dest_start..dest_start + width].copy_from_slice(first_row_region);
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in (0..height).step_by(b_size) {
            let block_height = std::cmp::min(b_size, height - y);
            let row_start = y * width;

            // Process each block in the row
            for x in (0..width).step_by(b_size) {
                let block_width = std::cmp::min(b_size, width - x);

                // The upper-left pixel of this block
                let color = pixels[row_start + x];

                // Fill the first row of this block
                let first_row_start = row_start + x;
                pixels[first_row_start..first_row_start + block_width].fill(color);
            }

            // Now that the first row is fully populated with block colors,
            // we can efficiently copy this entire row to the rest of the block rows.
            if block_height > 1 {
                let (first_row_region, rest) = pixels[row_start..].split_at_mut(width);

                for by in 1..block_height {
                    let dest_start = (by - 1) * width;
                    rest[dest_start..dest_start + width].copy_from_slice(first_row_region);
                }
            }
        }
    }
}
