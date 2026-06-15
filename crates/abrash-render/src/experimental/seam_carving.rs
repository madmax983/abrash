//! Seam Carving / Content-Aware Scaling
//!
//! A post-processing effect that removes vertical seams of lowest energy
//! to dynamically resize or distort an image while preserving important features.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies the seam carving algorithm to the given framebuffer to remove
/// `num_seams` vertical seams.
///
/// The algorithm calculates the dual-gradient energy of each pixel based on its
/// neighbors. It then uses dynamic programming to find the contiguous path from
/// top to bottom with the lowest total energy. The pixels along this path are
/// "carved" out by shifting the remaining pixels on that row to the left. The
/// rightmost column of the row is filled with the specified `fill_color`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `num_seams` - The number of seams to remove.
/// * `fill_color` - The ARGB color to use to fill the gap on the right.
pub fn apply_seam_carving(fb: &mut Framebuffer, num_seams: usize, fill_color: u32) {
    if num_seams == 0 {
        return;
    }

    let original_width = fb.width() as usize;
    let height = fb.height() as usize;

    if original_width < 2 || height < 2 || num_seams >= original_width {
        return;
    }

    let mut energy_map = vec![0.0; original_width * height];
    let mut cost_map = vec![0.0; original_width * height];
    let mut dir_map = vec![0i8; original_width * height]; // -1 left, 0 straight, 1 right

    let pixels = fb.as_mut_slice();

    for iteration in 0..num_seams {
        let current_width = original_width - iteration;

        // 1. Calculate energy map (Dual-Gradient)
        // We use a simple luminance gradient for performance.
        for y in 0..height {
            for x in 0..current_width {
                let left_x = x.saturating_sub(1);
                let right_x = (x + 1).min(current_width - 1);
                let top_y = y.saturating_sub(1);
                let bottom_y = (y + 1).min(height - 1);

                let pt = pixels[top_y * original_width + x];
                let pb = pixels[bottom_y * original_width + x];
                let pl = pixels[y * original_width + left_x];
                let pr = pixels[y * original_width + right_x];

                // Calculate luminance differences
                let lum_l = f32::from(pixel_luminance(pl));
                let lum_r = f32::from(pixel_luminance(pr));
                let lum_t = f32::from(pixel_luminance(pt));
                let lum_b = f32::from(pixel_luminance(pb));

                let dx = lum_r - lum_l;
                let dy = lum_b - lum_t;

                #[allow(clippy::imprecise_flops)]
                let energy = (dx * dx + dy * dy).sqrt();
                energy_map[y * original_width + x] = energy;
            }
        }

        // 2. Compute minimum cost path (Dynamic Programming)
        // Initialize the first row of costs with their energy
        for x in 0..current_width {
            cost_map[x] = energy_map[x];
        }

        for y in 1..height {
            for x in 0..current_width {
                let curr_idx = y * original_width + x;
                let prev_row_idx = (y - 1) * original_width;

                let cost_mid = cost_map[prev_row_idx + x];
                let mut min_cost = cost_mid;
                let mut dir = 0; // 0 = straight up

                if x > 0 {
                    let cost_left = cost_map[prev_row_idx + x - 1];
                    if cost_left < min_cost {
                        min_cost = cost_left;
                        dir = -1; // -1 = up-left
                    }
                }

                if x < current_width - 1 {
                    let cost_right = cost_map[prev_row_idx + x + 1];
                    if cost_right < min_cost {
                        min_cost = cost_right;
                        dir = 1; // 1 = up-right
                    }
                }

                cost_map[curr_idx] = energy_map[curr_idx] + min_cost;
                dir_map[curr_idx] = dir;
            }
        }

        // 3. Find the lowest cost seam at the bottom row
        let mut min_col = 0;
        let mut min_total_cost = f32::MAX;
        let bottom_row_idx = (height - 1) * original_width;

        for x in 0..current_width {
            let cost = cost_map[bottom_row_idx + x];
            if cost < min_total_cost {
                min_total_cost = cost;
                min_col = x;
            }
        }

        // 4. Trace back and carve the seam
        let mut current_col = min_col as i32;

        for y in (0..height).rev() {
            let cx = current_col as usize;

            // Shift pixels to the left to overwrite the seam
            let row_start = y * original_width;
            for x in cx..(current_width - 1) {
                pixels[row_start + x] = pixels[row_start + x + 1];
            }

            // Fill the newly created gap on the right
            pixels[row_start + current_width - 1] = fill_color;

            // Move up following the direction map
            if y > 0 {
                let dir = dir_map[y * original_width + cx];
                current_col += i32::from(dir);
                current_col = current_col.clamp(0, (current_width - 1) as i32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_seam_carving() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // Fill fb with a gradient
        for y in 0..5 {
            for x in 0..5 {
                let lum = (x * 50) as u32;
                fb.set_pixel(
                    x as i32,
                    y as i32,
                    0xFF000000 | (lum << 16) | (lum << 8) | lum,
                );
            }
        }

        let original_pixel = fb.get_pixel(4, 0).unwrap();

        apply_seam_carving(&mut fb, 2, 0xFFFF0000); // Fill with red

        // Rightmost 2 columns should now be filled with red
        for y in 0..5 {
            assert_eq!(fb.get_pixel(4, y).unwrap(), 0xFFFF0000);
            assert_eq!(fb.get_pixel(3, y).unwrap(), 0xFFFF0000);
        }

        // Ensure some pixel was shifted
        let mut shifted = false;
        for x in 0..3 {
            if fb.get_pixel(x, 0).unwrap() == original_pixel {
                shifted = true;
                break;
            }
        }
        assert!(
            shifted,
            "Pixels should be shifted left to overwrite the seam"
        );
    }
}
