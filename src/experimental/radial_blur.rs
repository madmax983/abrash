//! Radial Blur Post-Processing Filter
//!
//! A retro-style filter that blurs the image outwardly from a center point,
//! simulating a zooming or speed effect.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static RADIAL_BLUR_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a radial blur effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `cx` - The X coordinate of the blur center.
/// * `cy` - The Y coordinate of the blur center.
/// * `strength` - The intensity of the blur. 0.0 means no blur.
/// * `samples` - The number of samples to take along the blur vector. 0 or 1 means no blur.
/// Bolt Performance Optimization:
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
/// Bolt Performance Optimization:
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
pub fn apply_radial_blur(
    fb: &mut Framebuffer,
    cx: usize,
    cy: usize,
    strength: f32,
    samples: usize,
) {
    if strength == 0.0 || samples <= 1 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // We must copy the buffer to read from the original state while writing to the new state.
    // This avoids artifacts from reading already-blurred pixels.
    // Use a thread-local buffer to prevent a massive heap allocation on every frame.
    RADIAL_BLUR_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        let cx_f32 = cx as f32;
        let cy_f32 = cy as f32;
        let sf_fixed = -(strength / (samples - 1) as f32) * 65536.0;
        let inv_samples = samples as u32;

        let w_m1 = width as i32 - 1;
        let h_m1 = height as i32 - 1;

        // If samples <= 256, we can accumulate R and B channels in a single u32 register (SWAR)
        // without the B channel (bits 0-7) overflowing into the R channel (bits 16-23)
        // because the gap (bits 8-15) can hold exactly 256 accumulations of the max 8-bit value (255).
        let can_swar = samples <= 256;

        let process_pixel = |x: usize, y: usize| -> u32 {
            let dx = x as f32 - cx_f32;
            let dy = y as f32 - cy_f32;

            let step_x = (dx * sf_fixed) as i32;
            let step_y = (dy * sf_fixed) as i32;

            let mut cur_x = (x as i32) << 16;
            let mut cur_y = (y as i32) << 16;

            if can_swar {
                let mut rb_acc = 0;
                let mut g_acc = 0;

                for _ in 0..samples {
                    let x_idx = (cur_x >> 16).max(0).min(w_m1) as usize;
                    let y_idx = (cur_y >> 16).max(0).min(h_m1) as usize;

                    let color = src_fb[y_idx * width + x_idx];

                    // Accumulate R and B channels simultaneously. The 0x00FF00FF mask isolates R and B.
                    rb_acc += color & 0x00FF_00FF;
                    // Accumulate G channel separately.
                    g_acc += (color >> 8) & 0x0000_00FF;

                    cur_x += step_x;
                    cur_y += step_y;
                }

                let r = (rb_acc >> 16) / inv_samples;
                let g = g_acc / inv_samples;
                let b = (rb_acc & 0xFFFF) / inv_samples;

                (r << 16) | (g << 8) | b
            } else {
                let mut r_acc = 0;
                let mut g_acc = 0;
                let mut b_acc = 0;

                for _ in 0..samples {
                    let x_idx = (cur_x >> 16).max(0).min(w_m1) as usize;
                    let y_idx = (cur_y >> 16).max(0).min(h_m1) as usize;

                    let color = src_fb[y_idx * width + x_idx];
                    r_acc += (color >> 16) & 0xFF;
                    g_acc += (color >> 8) & 0xFF;
                    b_acc += color & 0xFF;

                    cur_x += step_x;
                    cur_y += step_y;
                }

                let r = (r_acc / inv_samples) & 0xFF;
                let g = (g_acc / inv_samples) & 0xFF;
                let b = (b_acc / inv_samples) & 0xFF;

                (r << 16) | (g << 8) | b
            }
        };

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            dest_pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        *pixel = process_pixel(x, y);
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                let row_start = y * width;
                for x in 0..width {
                    dest_pixels[row_start + x] = process_pixel(x, y);
                }
            }
        }
    });
}
