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

    #[cfg(feature = "parallel")]
    let row_iter = dest
        .par_chunks_exact_mut(width)
        .enumerate()
        .skip(1)
        .take(height - 2);
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_exact_mut(width).enumerate().skip(1).take(height - 2);

    row_iter.for_each(|(y, row)| {
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

                // Add bias (128) and clamp using unsigned math
                let out_r = (pos_r + 128).saturating_sub(neg_r).min(255);
                let out_g = (pos_g + 128).saturating_sub(neg_g).min(255);
                let out_b = (pos_b + 128).saturating_sub(neg_b).min(255);

                // Preserve alpha from center
                let a = c & 0xFF00_0000;

                *dest_pixel = a | (out_r << 16) | (out_g << 8) | out_b;
            });
    });
}
