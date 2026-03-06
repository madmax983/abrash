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

        // Precalculate scales
        let mut scales = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = i as f32 / (samples - 1) as f32;
            scales.push(1.0 - (strength * t));
        }

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            dest_pixels
                .par_chunks_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx as f32;
                        let dy = y as f32 - cy as f32;

                        let mut r_acc = 0;
                        let mut g_acc = 0;
                        let mut b_acc = 0;

                        for &scale in &scales {
                            // Using standard clamp optimization: bounds are static per render pass.
                            let sample_x = (cx as f32 + dx * scale) as i32;
                            let sample_y = (cy as f32 + dy * scale) as i32;

                            // Using direct min/max instead of clamp is usually faster when inline
                            let x_idx = sample_x.max(0).min(width as i32 - 1) as usize;
                            let y_idx = sample_y.max(0).min(height as i32 - 1) as usize;

                            let color = src_fb[y_idx * width + x_idx];
                            r_acc += (color >> 16) & 0xFF;
                            g_acc += (color >> 8) & 0xFF;
                            b_acc += color & 0xFF;
                        }

                        let inv_samples = samples as u32;
                        let r = (r_acc / inv_samples) & 0xFF;
                        let g = (g_acc / inv_samples) & 0xFF;
                        let b = (b_acc / inv_samples) & 0xFF;

                        *pixel = (r << 16) | (g << 8) | b;
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                let row_start = y * width;
                for x in 0..width {
                    let dx = x as f32 - cx as f32;
                    let dy = y as f32 - cy as f32;

                    let mut r_acc = 0;
                    let mut g_acc = 0;
                    let mut b_acc = 0;

                    for &scale in &scales {
                        let sample_x = (cx as f32 + dx * scale) as i32;
                        let sample_y = (cy as f32 + dy * scale) as i32;

                        let x_idx = sample_x.max(0).min(width as i32 - 1) as usize;
                        let y_idx = sample_y.max(0).min(height as i32 - 1) as usize;

                        let color = src_fb[y_idx * width + x_idx];
                        r_acc += (color >> 16) & 0xFF;
                        g_acc += (color >> 8) & 0xFF;
                        b_acc += color & 0xFF;
                    }

                    let inv_samples = samples as u32;
                    let r = (r_acc / inv_samples) & 0xFF;
                    let g = (g_acc / inv_samples) & 0xFF;
                    let b = (b_acc / inv_samples) & 0xFF;

                    dest_pixels[row_start + x] = (r << 16) | (g << 8) | b;
                }
            }
        }
    });
}
