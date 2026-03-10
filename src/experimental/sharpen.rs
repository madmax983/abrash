//! Sharpen Post-Processing Filter
//!
//! A filter that enhances the edges of an image using a convolution kernel.

use crate::framebuffer::Framebuffer;

/// Applies a sharpen effect to the framebuffer.
///
/// Uses a 3x3 convolution kernel to enhance edges.
/// The 1-pixel border of the image is left unmodified to avoid boundary issues.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `amount` - The intensity of the sharpen effect (0.0 to 1.0).
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
    let center_weight = 1.0 + 4.0 * amount;
    let side_weight = -amount;

    let src = fb.as_slice().to_vec(); // create a copy of the framebuffer to read from
    let dst = fb.as_mut_slice();

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
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y_idx, row)| {
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

                    let mut r_sum = 0.0;
                    let mut g_sum = 0.0;
                    let mut b_sum = 0.0;

                    // Extract and accumulate
                    let c_r = ((center_c >> 16) & 0xFF) as f32;
                    let c_g = ((center_c >> 8) & 0xFF) as f32;
                    let c_b = (center_c & 0xFF) as f32;

                    r_sum += c_r * center_weight;
                    g_sum += c_g * center_weight;
                    b_sum += c_b * center_weight;

                    // Add sides
                    for &side_c in &[top_c, bottom_c, left_c, right_c] {
                        let s_r = ((side_c >> 16) & 0xFF) as f32;
                        let s_g = ((side_c >> 8) & 0xFF) as f32;
                        let s_b = (side_c & 0xFF) as f32;

                        r_sum += s_r * side_weight;
                        g_sum += s_g * side_weight;
                        b_sum += s_b * side_weight;
                    }

                    let r = r_sum.clamp(0.0, 255.0) as u32;
                    let g = g_sum.clamp(0.0, 255.0) as u32;
                    let b = b_sum.clamp(0.0, 255.0) as u32;

                    // preserve alpha
                    let a = center_c & 0xFF000000;

                    row[x] = a | (r << 16) | (g << 8) | b;
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 1..(height - 1) {
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

                let mut r_sum = 0.0;
                let mut g_sum = 0.0;
                let mut b_sum = 0.0;

                // Extract and accumulate
                let c_r = ((center_c >> 16) & 0xFF) as f32;
                let c_g = ((center_c >> 8) & 0xFF) as f32;
                let c_b = (center_c & 0xFF) as f32;

                r_sum += c_r * center_weight;
                g_sum += c_g * center_weight;
                b_sum += c_b * center_weight;

                // Add sides
                for &side_c in &[top_c, bottom_c, left_c, right_c] {
                    let s_r = ((side_c >> 16) & 0xFF) as f32;
                    let s_g = ((side_c >> 8) & 0xFF) as f32;
                    let s_b = (side_c & 0xFF) as f32;

                    r_sum += s_r * side_weight;
                    g_sum += s_g * side_weight;
                    b_sum += s_b * side_weight;
                }

                let r = r_sum.clamp(0.0, 255.0) as u32;
                let g = g_sum.clamp(0.0, 255.0) as u32;
                let b = b_sum.clamp(0.0, 255.0) as u32;

                // preserve alpha
                let a = center_c & 0xFF000000;

                dst[center_idx] = a | (r << 16) | (g << 8) | b;
            }
        }
    }
}
