//! Emboss Post-Processing Effect
//!
//! Simulates a 3D embossed look by replacing the image with a gray background and
//! highlighting edges using a directional convolution kernel.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies an emboss filter to the framebuffer.
///
/// Uses a 3x3 convolution kernel:
/// ```text
/// [ -1, -1,  0 ]
/// [ -1,  1,  1 ]
/// [  0,  1,  1 ]
/// ```
/// The result is biased by 128 (mid-gray) to visualize both positive and negative edges.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
pub fn apply_emboss(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width < 3 || height < 3 {
        return; // Too small for 3x3 kernel
    }

    let src = fb.as_slice().to_vec();
    let dest = fb.as_mut_slice();

    // Kernel:
    // -1, -1,  0
    // -1,  1,  1
    //  0,  1,  1

    // Extract channels
    let extract = |p: u32| ((p >> 16) & 0xFF, (p >> 8) & 0xFF, p & 0xFF);

    // Dest is width * height. We skip the first row.
    // .chunks_exact_mut(width) yields rows.
    // skipping 1 skips row 0. Then taking height - 2 yields rows 1..height-1.
    // The index from enumerate() will start at 0. So y = index + 1.
    #[cfg(feature = "parallel")]
    let row_iter = dest
        .par_chunks_exact_mut(width)
        .skip(1)
        .take(height - 2)
        .enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest
        .chunks_exact_mut(width)
        .skip(1)
        .take(height - 2)
        .enumerate();

    row_iter.for_each(|(y_offset, row)| {
        let y = y_offset + 1; // Since we skip 1, the index y_offset starts at 0 for row y=1
        let prev_row_offset = (y - 1) * width;
        let row_offset = y * width;
        let next_row_offset = (y + 1) * width;

        let prev_row = &src[prev_row_offset..prev_row_offset + width];
        let curr_row = &src[row_offset..row_offset + width];
        let next_row = &src[next_row_offset..next_row_offset + width];

        let dest_row = &mut row[1..width - 1];

        dest_row
            .iter_mut()
            .zip(prev_row.windows(3))
            .zip(curr_row.windows(3))
            .zip(next_row.windows(3))
            .for_each(|(((dest_pixel, prev_w), curr_w), next_w)| {
                // Read pixels
                let tl = prev_w[0];
                let t = prev_w[1];

                let l = curr_w[0];
                let c = curr_w[1];
                let r = curr_w[2];

                let b = next_w[1];
                let br = next_w[2];

                let (tl_r, tl_g, tl_b) = extract(tl);
                let (t_r, t_g, t_b) = extract(t);
                let (l_r, l_g, l_b) = extract(l);
                let (c_r, c_g, c_b) = extract(c);
                let (r_r, r_g, r_b) = extract(r);
                let (b_r, b_g, b_b) = extract(b);
                let (br_r, br_g, br_b) = extract(br);

                // Apply weights using saturating unsigned integer operations
                let pos_r = c_r + r_r + b_r + br_r;
                let neg_r = tl_r + t_r + l_r;

                let pos_g = c_g + r_g + b_g + br_g;
                let neg_g = tl_g + t_g + l_g;

                let pos_b = c_b + r_b + b_b + br_b;
                let neg_b = tl_b + t_b + l_b;

                // To map flat color areas to neutral grey properly, we compute
                // the average difference of the color channels and add a neutral bias.
                let diff_r = (pos_r as i32) - (neg_r as i32);
                let diff_g = (pos_g as i32) - (neg_g as i32);
                let diff_b = (pos_b as i32) - (neg_b as i32);

                let avg_diff = (diff_r + diff_g + diff_b) / 3;

                // Add bias (128) and clamp
                let out = (avg_diff + 128).clamp(0, 255) as u32;

                // Preserve alpha from center
                let a = c & 0xFF00_0000;

                *dest_pixel = a | (out << 16) | (out << 8) | out;
            });
    });
}
