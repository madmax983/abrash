//! Sharpen Post-Processing Filter
//!
//! A filter that enhances the edges of an image using a convolution kernel.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a sharpen effect to the framebuffer.
///
/// Uses a 3x3 convolution kernel to enhance edges.
/// The 1-pixel border of the image is left unmodified to avoid boundary issues.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `amount` - The intensity of the sharpen effect (0.0 to 1.0).
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_sharpen(fb: &mut Framebuffer, amount: f32) {
    if amount <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width < 3 || height < 3 {
        return;
    }

    let amount = amount.clamp(0.0, 1.0);

    // Original weight is scaled by amount
    // Center pixel gets (1 + 4 * amount), surrounding gets (-amount)
    let center_weight_fixed = ((1.0 + 4.0 * amount) * 256.0) as i32;
    let side_weight_fixed = (-amount * 256.0) as i32;

    SOURCE_PIXELS.with(|buf| {
        let mut src_vec = buf.borrow_mut();
        src_vec.clear();
        src_vec.extend_from_slice(fb.as_slice());
        let src = src_vec.as_slice();
        let dst = fb.as_mut_slice();

        let process_row = |(y_idx, row): (usize, &mut [u32])| {
            let y = y_idx + 1; // actual y in the full image
            let row_start = y * width;
            let prev_row = (y - 1) * width;
            let next_row = (y + 1) * width;

            for x in 1..(width - 1) {
                let center_idx = row_start + x;

                // Read 5 pixels (center + 4 neighbors)
                let center_c = src[center_idx];
                let top_c = src[prev_row + x];
                let bottom_c = src[next_row + x];
                let left_c = src[row_start + x - 1];
                let right_c = src[row_start + x + 1];

                // Extract center
                let c_r = ((center_c >> 16) & 0xFF) as i32;
                let c_g = ((center_c >> 8) & 0xFF) as i32;
                let c_b = (center_c & 0xFF) as i32;

                let mut r_sum = c_r * center_weight_fixed;
                let mut g_sum = c_g * center_weight_fixed;
                let mut b_sum = c_b * center_weight_fixed;

                // Add sides inline
                for &side_c in &[top_c, bottom_c, left_c, right_c] {
                    let s_r = ((side_c >> 16) & 0xFF) as i32;
                    let s_g = ((side_c >> 8) & 0xFF) as i32;
                    let s_b = (side_c & 0xFF) as i32;

                    r_sum += s_r * side_weight_fixed;
                    g_sum += s_g * side_weight_fixed;
                    b_sum += s_b * side_weight_fixed;
                }

                let r = (r_sum >> 8).clamp(0, 255) as u32;
                let g = (g_sum >> 8).clamp(0, 255) as u32;
                let b = (b_sum >> 8).clamp(0, 255) as u32;

                // preserve alpha
                let a = center_c & 0xFF00_0000;

                row[x] = a | (r << 16) | (g << 8) | b;
            }
        };

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            // We process from y=1 to y=height-2. We can split `dst` into chunks of rows,
            // but it's simpler to just iterate over rows using par_chunks_mut, keeping
            // the y offset in mind.
            // The first row of `dst` is at y=0, which we don't modify.
            // We'll skip the first row, then process `height - 2` rows.

            // We slice dst from the second row up to the second-to-last row.
            let dst_body = &mut dst[width..width * (height - 1)];

            dst_body
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(process_row);
        }

        #[cfg(not(feature = "parallel"))]
        {
            let dst_body = &mut dst[width..width * (height - 1)];
            dst_body
                .chunks_exact_mut(width)
                .enumerate()
                .for_each(process_row);
        }
    });
}
