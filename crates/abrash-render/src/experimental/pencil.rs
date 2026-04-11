//! Pencil Sketch Post-Processing Effect
//!
//! Simulates a pencil sketch by combining grayscale conversion, inversion,
//! Gaussian blurring (approximated), and color dodging.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static PENCIL_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a pencil sketch filter to the framebuffer.
pub fn apply_pencil_sketch(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width < 3 || height < 3 {
        return;
    }

    let mut scratch = PENCIL_BUFFER.with(std::cell::RefCell::take);
    let fb_slice = fb.as_slice();
    if scratch.len() != fb_slice.len() {
        scratch.resize(fb_slice.len(), 0);
    }

    // Step 1: Grayscale and invert into scratch buffer
    scratch.iter_mut().zip(fb_slice.iter()).for_each(|(out, &p)| {
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        // Luminance
        let lum = (r * 77 + g * 150 + b * 29) >> 8;

        // Invert
        let inv_lum = 255 - lum;

        *out = (p & 0xFF00_0000) | (inv_lum << 16) | (inv_lum << 8) | inv_lum;
    });

    // Step 2: Apply a small blur to the inverted image (simulated with a simple box blur for now)
    // In a real implementation this might be a multi-pass Gaussian blur.
    let dest = fb.as_mut_slice();

    let src: &[u32] = &scratch;

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
                // Sum the 3x3 neighborhood of the inverted luminance
                let sum = (prev_w[0] & 0xFF) + (prev_w[1] & 0xFF) + (prev_w[2] & 0xFF) +
                          (curr_w[0] & 0xFF) + (curr_w[1] & 0xFF) + (curr_w[2] & 0xFF) +
                          (next_w[0] & 0xFF) + (next_w[1] & 0xFF) + (next_w[2] & 0xFF);

                let blurred_inv = sum / 9;

                // Color Dodge blend: base / (1.0 - blend)
                let p = *dest_pixel;
                let r = (p >> 16) & 0xFF;
                let g = (p >> 8) & 0xFF;
                let b = p & 0xFF;
                let base = (r * 77 + g * 150 + b * 29) >> 8;

                // Color Dodge math:
                let result = if blurred_inv == 255 {
                    255
                } else {
                    let dodge = (base << 8) / (255 - blurred_inv);
                    dodge.min(255)
                };

                let a = p & 0xFF00_0000;
                *dest_pixel = a | (result << 16) | (result << 8) | result;
            });
    });

    PENCIL_BUFFER.with(|cell| cell.replace(scratch));
}
