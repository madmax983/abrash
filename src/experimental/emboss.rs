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
    let extract = |p: u32| {
        (
            ((p >> 16) & 0xFF) as i32,
            ((p >> 8) & 0xFF) as i32,
            (p & 0xFF) as i32,
        )
    };

    #[cfg(feature = "parallel")]
    let row_iter = dest
        .par_chunks_mut(width)
        .enumerate()
        .skip(1)
        .take(height - 2);
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_mut(width).enumerate().skip(1).take(height - 2);

    row_iter.for_each(|(y, row)| {
        let prev_row_offset = (y - 1) * width;
        let row_offset = y * width;
        let next_row_offset = (y + 1) * width;

        let prev_row = &src[prev_row_offset..prev_row_offset + width];
        let curr_row = &src[row_offset..row_offset + width];
        let next_row = &src[next_row_offset..next_row_offset + width];

        for x in 1..width - 1 {
            // Read pixels
            let tl = prev_row[x - 1];
            let t = prev_row[x];

            let l = curr_row[x - 1];
            let c = curr_row[x];
            let r = curr_row[x + 1];

            let b = next_row[x];
            let br = next_row[x + 1];

            let (tl_r, tl_g, tl_b) = extract(tl);
            let (t_r, t_g, t_b) = extract(t);
            let (l_r, l_g, l_b) = extract(l);
            let (c_r, c_g, c_b) = extract(c);
            let (r_r, r_g, r_b) = extract(r);
            let (b_r, b_g, b_b) = extract(b);
            let (br_r, br_g, br_b) = extract(br);

            // Apply weights
            let sum_r = -tl_r - t_r - l_r + c_r + r_r + b_r + br_r;
            let sum_g = -tl_g - t_g - l_g + c_g + r_g + b_g + br_g;
            let sum_b = -tl_b - t_b - l_b + c_b + r_b + b_b + br_b;

            // Add bias and clamp
            let out_r = (sum_r + 128).clamp(0, 255) as u32;
            let out_g = (sum_g + 128).clamp(0, 255) as u32;
            let out_b = (sum_b + 128).clamp(0, 255) as u32;

            // Preserve alpha from center
            let a = c & 0xFF00_0000;

            row[x] = a | (out_r << 16) | (out_g << 8) | out_b;
        }
    });
}
