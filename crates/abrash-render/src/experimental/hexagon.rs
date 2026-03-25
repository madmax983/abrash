//! Hexagon Pixelation Filter
//!
//! A retro post-processing effect that creates a geometric, honeycomb-like pixelation.
//! It divides the screen into a hexagonal grid based on a given radius,
//! maps each pixel to the nearest hexagon center, and fills it with that center's color.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static HEX_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a hexagonal pixelation effect to the framebuffer.
///
/// Divides the screen into a honeycomb grid where each hexagon has the specified `radius`.
/// Each pixel in the framebuffer is replaced by the color of the pixel at the center of its enclosing hexagon.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `radius` - The radius (circumradius) of the hexagons in pixels.
pub fn apply_hexagon(fb: &mut Framebuffer, radius: f32) {
    if radius <= 1.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Since pixels sample from other coordinates (the center of their hexagon),
    // we must read from a cloned source buffer and write to the destination.
    HEX_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        // Hexagon math constants
        // R = radius
        // width = sqrt(3) * R
        // horizontal spacing = width
        // vertical spacing = 1.5 * R
        let hex_width = 3.0_f32.sqrt() * radius;
        let hex_height = 2.0 * radius;

        let half_width = hex_width / 2.0;
        let three_quarters_height = 1.5 * radius;

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let y_f32 = y as f32;

            for (x, pixel) in row.iter_mut().enumerate() {
                let x_f32 = x as f32;

                // Find the approximate grid coordinates
                let grid_y = (y_f32 / three_quarters_height).floor() as i32;

                // Offset X every other row
                let offset_x = if grid_y % 2 != 0 { half_width } else { 0.0 };

                let grid_x = ((x_f32 - offset_x) / hex_width).floor() as i32;

                // Calculate the exact centers of the four potentially closest hexagons
                let mut best_dist_sq = f32::MAX;
                let mut best_cx = 0;
                let mut best_cy = 0;

                // Check 2x2 local grid
                for dy in 0..=1 {
                    for dx in 0..=1 {
                        let ty = grid_y + dy;
                        let tx = grid_x + dx;

                        let cx_f32 =
                            (tx as f32 * hex_width) + if ty % 2 != 0 { half_width } else { 0.0 };
                        let cy_f32 = ty as f32 * three_quarters_height;

                        let dist_x = x_f32 - cx_f32;
                        let dist_y = y_f32 - cy_f32;
                        let dist_sq = dist_x * dist_x + dist_y * dist_y;

                        if dist_sq < best_dist_sq {
                            best_dist_sq = dist_sq;
                            best_cx = cx_f32.round() as i32;
                            best_cy = cy_f32.round() as i32;
                        }
                    }
                }

                // Clamp center coordinates to framebuffer bounds
                let clamped_cx = best_cx.clamp(0, width as i32 - 1) as usize;
                let clamped_cy = best_cy.clamp(0, height as i32 - 1) as usize;

                // Sample the color from the source buffer at the hexagon's center
                let center_idx = clamped_cy * width + clamped_cx;
                *pixel = src_pixels[center_idx];
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_hexagon_bounds() {
        let sizes = [(1, 1), (10, 10), (100, 1), (1, 100), (33, 47)];

        for (w, h) in sizes {
            let mut fb = Framebuffer::new(w, h).unwrap();
            fb.clear(0xFFFFFFFF);

            for radius in [1.5, 5.0, 10.0, 50.0] {
                apply_hexagon(&mut fb, radius);
            }
        }
    }

    #[test]
    fn test_apply_hexagon_white_image() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FF_FF_FF);

        apply_hexagon(&mut fb, 5.0);

        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFFFF_FFFF);
        }
    }
}
