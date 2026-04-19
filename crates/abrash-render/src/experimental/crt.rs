//! CRT Monitor Post-Processing Filter
//!
//! Simulates the barrel distortion, scanlines, chromatic aberration, and vignette of a classic Cathode Ray Tube monitor.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

/// Applies a comprehensive CRT monitor effect to the framebuffer.
///
/// Features:
/// - Barrel distortion to simulate screen curvature.
/// - Scanlines that scale with vertical position.
/// - Chromatic aberration (RGB shift) at the screen edges.
/// - Vignette for darkened corners.
///
/// # Performance
///
/// Optimization: Uses a `thread_local!` buffer to avoid allocating a new
/// `Vec` every frame.
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

        CRT_BUFFER.with(|buf| {
            let mut new_pixels_vec = buf.borrow_mut();
            let size = width * height;
            if new_pixels_vec.len() < size {
                new_pixels_vec.resize(size, 0xFF00_0000);
            }
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
                        process_row(y, row, width, height, cx, cy, nx_cache, pixels, distortion);
                    });
            }

            #[cfg(not(feature = "parallel"))]
            {
                for (y, row) in new_pixels.chunks_exact_mut(width).enumerate() {
                    process_row(y, row, width, height, cx, cy, nx_cache, pixels, distortion);
                }
            }

            fb.as_mut_slice().copy_from_slice(&new_pixels);
        });
    });
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn process_row(
    y: usize,
    row: &mut [u32],
    width: usize,
    height: usize,
    cx: f32,
    cy: f32,
    nx_cache: &[f32],
    pixels: &[u32],
    distortion: f32,
) {
    let ny = (y as f32 - cy) / cy;
    let ny2 = ny * ny;

    // Scanline intensity based on y coordinate
    let scanline_mult = if y % 3 == 0 {
        0.8
    } else if y % 3 == 1 {
        0.9
    } else {
        1.0
    };

    for (x, pixel) in row.iter_mut().enumerate().take(width) {
        let nx = nx_cache[x];
        let r2 = nx * nx + ny2;

        // Barrel distortion mapping
        let f = 1.0 + distortion * r2;
        let sx = nx * f;
        let sy = ny * f;

        let vignette = (1.0 - r2 * 0.5).clamp(0.0, 1.0);

        // Map back to screen space coordinates
        let src_x = (sx * cx + cx) as i32;
        let src_y = (sy * cy + cy) as i32;

        if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
            // Chromatic aberration shift based on radial distance
            let shift_x = (nx * 3.0 * f) as i32;
            let shift_y = (ny * 3.0 * f) as i32;

            let c_x = src_x as usize;
            let c_y = src_y as usize;


            // Read R channel with shift
            let rx = (src_x - shift_x).clamp(0, width as i32 - 1) as usize;
            let ry = (src_y - shift_y).clamp(0, height as i32 - 1) as usize;
            let r_val = (pixels[ry * width + rx] >> 16) & 0xFF;

            // Read G channel (no shift)
            let g_val = (pixels[c_y * width + c_x] >> 8) & 0xFF;

            // Read B channel with opposite shift
            let bx = (src_x + shift_x).clamp(0, width as i32 - 1) as usize;
            let by = (src_y + shift_y).clamp(0, height as i32 - 1) as usize;
            let b_val = pixels[by * width + bx] & 0xFF;

            // Apply vignette and scanlines
            let multiplier = scanline_mult * vignette;
            let r_out = ((r_val as f32 * multiplier) as u32).min(255);
            let g_out = ((g_val as f32 * multiplier) as u32).min(255);
            let b_out = ((b_val as f32 * multiplier) as u32).min(255);

            *pixel = 0xFF00_0000 | (r_out << 16) | (g_out << 8) | b_out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_crt_filter_modifies_buffer() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFF_FFFF);

        apply_crt(&mut fb, 0.2);

        // Corners should be black due to barrel distortion
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(99, 99), Some(0xFF00_0000));

        // Center should remain somewhat white, but darkened by vignette/scanline
        // It won't be exactly 0xFFFF_FFFF
        let center = fb.get_pixel(50, 50).unwrap();
        let center_r = (center >> 16) & 0xFF;
        assert!(center_r > 100 && center_r <= 255);
    }
}
