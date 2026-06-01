//! CRT Monitor Post-Processing Filter
//!
//! Simulates the barrel distortion of a classic Cathode Ray Tube monitor.

use abrash_core::framebuffer::Framebuffer;
use std::cell::RefCell;

/// Applies a CRT monitor barrel distortion effect to the framebuffer.
///
/// Pixels are mapped using a radial distortion function to curve the image
/// away from the center, mimicking the curvature of a physical CRT screen.
///
/// # Performance
///
/// Optimization: Uses a `thread_local!` buffer instead of allocating a new
/// `Vec` every frame. This eliminates a costly dynamic heap allocation
/// per frame, significantly improving performance on the hot path while
/// preventing in-place overwrite artifacts.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `distortion` - The strength of the barrel distortion (e.g., 0.1 to 0.3).
pub fn apply_crt(fb: &mut Framebuffer, distortion: f32) {
    if distortion <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    thread_local! {
        static CRT_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        static NX_CACHE: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    }

    NX_CACHE.with(|nx_buf| {
        let mut nx_vec = nx_buf.borrow_mut();
        if nx_vec.len() != width {
            nx_vec.resize(width, 0.0);
            for x in 0..width {
                nx_vec[x] = (x as f32 - cx) / cx;
            }
        }
        let nx_cache = &nx_vec[..width];

        // To prevent in-place overwrite issues, we need to read from the original
        // and write to a copy, then copy back.
        CRT_BUFFER.with(|buf| {
            let mut new_pixels_vec = buf.borrow_mut();
            let size = width * height;
            if new_pixels_vec.len() < size {
                new_pixels_vec.resize(size, 0xFF00_0000);
            }
            // Ensure background is black if parts aren't overwritten
            let new_pixels = &mut new_pixels_vec[..size];
            new_pixels.fill(0xFF00_0000);

            let pixels = fb.as_slice();

            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                new_pixels
                    .par_chunks_exact_mut(width)
                    .enumerate()
                    .for_each(|(y, row)| {
                        let ny = (y as f32 - cy) / cy;
                        let ny2 = ny * ny;

                        for (x, pixel) in row.iter_mut().enumerate().take(width) {
                            // Read normalized x coordinate from cache
                            let nx = nx_cache[x];

                            // Calculate radial distance squared
                            let r2 = nx * nx + ny2;

                            // Apply barrel distortion mapping: r' = r * (1 + k * r^2)
                            let f = 1.0 + distortion * r2;
                            let sx = nx * f;
                            let sy = ny * f;

                            // Map back to screen space coordinates
                            // Use fast cast instead of round()
                            let src_x = (sx * cx + cx) as i32;
                            let src_y = (sy * cy + cy) as i32;

                            if src_x >= 0
                                && src_x < width as i32
                                && src_y >= 0
                                && src_y < height as i32
                            {
                                let src_idx = (src_y as usize) * width + (src_x as usize);
                                *pixel = pixels[src_idx];
                            }
                        }
                    });
            }

            #[cfg(not(feature = "parallel"))]
            {
                for y in 0..height {
                    let ny = (y as f32 - cy) / cy;
                    let ny2 = ny * ny;

                    for x in 0..width {
                        // Read normalized x coordinate from cache
                        let nx = nx_cache[x];

                        // Calculate radial distance squared
                        let r2 = nx * nx + ny2;

                        // Apply barrel distortion mapping: r' = r * (1 + k * r^2)
                        let f = 1.0 + distortion * r2;
                        let sx = nx * f;
                        let sy = ny * f;

                        // Map back to screen space coordinates
                        let src_x = (sx * cx + cx) as i32;
                        let src_y = (sy * cy + cy) as i32;

                        let dest_idx = y * width + x;

                        if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32
                        {
                            let src_idx = (src_y as usize) * width + (src_x as usize);
                            new_pixels[dest_idx] = pixels[src_idx];
                        }
                    }
                }
            }

            // Copy the distorted image back into the framebuffer
            fb.as_mut_slice().copy_from_slice(&new_pixels);
        });
    });
}
