//! Emboss Post-Processing Effect
//!
//! Simulates a 3D embossed look by replacing the image with a gray background and
//! highlighting edges using a directional convolution kernel.

use crate::framebuffer::Framebuffer;

use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

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
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_emboss(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width < 3 || height < 3 {
        return; // Too small for 3x3 kernel
    }

    let mut source_pixels = SOURCE_PIXELS.with(std::cell::RefCell::take);

    let fb_slice = fb.as_slice();
    if source_pixels.len() != fb_slice.len() {
        source_pixels.resize(fb_slice.len(), 0);
    }
    source_pixels.copy_from_slice(fb_slice);

    let dest = fb.as_mut_slice();

    // We bind it to a local variable to safely borrow it across threads
    let src: &[u32] = &source_pixels;

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
    let row_iter = dest
        .chunks_exact_mut(width)
        .enumerate()
        .skip(1)
        .take(height - 2);

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

                // ⚡ Bolt Performance Optimization:
                // SIMD Within A Register (SWAR)
                // Process Red and Blue channels simultaneously.
                // 0x00FF_00FF leaves 16 bits of headroom per channel. Max sum is 4*255=1020, which easily fits.
                let tl_rb = tl & 0x00FF_00FF;
                let tl_g = tl & 0x0000_FF00;

                let t_rb = t & 0x00FF_00FF;
                let t_g = t & 0x0000_FF00;

                let l_rb = l & 0x00FF_00FF;
                let l_g = l & 0x0000_FF00;

                let c_rb = c & 0x00FF_00FF;
                let c_g = c & 0x0000_FF00;

                let r_rb = r & 0x00FF_00FF;
                let r_g = r & 0x0000_FF00;

                let b_rb = b & 0x00FF_00FF;
                let b_g = b & 0x0000_FF00;

                let br_rb = br & 0x00FF_00FF;
                let br_g = br & 0x0000_FF00;

                // Apply weights using SWAR arithmetic
                let pos_rb = c_rb + r_rb + b_rb + br_rb;
                let neg_rb = tl_rb + t_rb + l_rb;

                let pos_g = c_g + r_g + b_g + br_g;
                let neg_g = tl_g + t_g + l_g;

                // We must unpack before saturating_sub to avoid channel overflow/underflow interactions
                let pos_r = (pos_rb >> 16) & 0xFFFF;
                let pos_b = pos_rb & 0xFFFF;
                let neg_r = (neg_rb >> 16) & 0xFFFF;
                let neg_b = neg_rb & 0xFFFF;

                let pos_g = (pos_g >> 8) & 0xFFFF;
                let neg_g = (neg_g >> 8) & 0xFFFF;

                // Add bias (128) and clamp using unsigned math
                let out_r = (pos_r + 128).saturating_sub(neg_r).min(255);
                let out_g = (pos_g + 128).saturating_sub(neg_g).min(255);
                let out_b = (pos_b + 128).saturating_sub(neg_b).min(255);

                // Preserve alpha from center
                let a = c & 0xFF00_0000;

                *dest_pixel = a | (out_r << 16) | (out_g << 8) | out_b;
            });
    });

    SOURCE_PIXELS.with(|source_pixels_cell| source_pixels_cell.replace(source_pixels));
}
