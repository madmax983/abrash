//! Seam Carving / Content-Aware Scaling
//!
//! A post-processing effect that dynamically scales down an image by removing
//! the least noticeable "seams" (paths of pixels from top to bottom) based on an energy function.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Reduces the width of the given framebuffer by `pixels_to_remove` using Seam Carving.
///
/// The function iteratively calculates the energy of the image, finds the vertical seam
/// with the lowest total energy, and removes it.
///
/// Note: The `Framebuffer` struct itself retains its original capacity and `.width()` value
/// for memory allocation reasons. The visible content is shifted to the left, leaving
/// black pixels on the right edge. Callers should track the "effective width" manually
/// if they need to crop the final output buffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `pixels_to_remove` - The number of vertical seams (columns) to remove.
pub fn apply_seam_carving(fb: &mut Framebuffer, pixels_to_remove: u32) {
    let mut effective_width = fb.width() as usize;
    let height = fb.height() as usize;

    if effective_width == 0 || height == 0 || pixels_to_remove == 0 {
        return;
    }

    let pixels_to_remove = (pixels_to_remove as usize).min(effective_width.saturating_sub(1));

    // Allocate scratchpad buffers once to avoid per-iteration allocations.
    let mut energy = vec![0u32; effective_width * height];
    let mut cost = vec![0u32; effective_width * height];
    let mut path = vec![0i32; effective_width * height]; // Stores the x-offset (-1, 0, 1) to the previous pixel in the seam

    let original_width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    for _ in 0..pixels_to_remove {
        // 1. Calculate Energy (Dual-Gradient using Luminance)
        for y in 0..height {
            for x in 0..effective_width {
                let left_x = x.saturating_sub(1);
                let right_x = (x + 1).min(effective_width - 1);
                let up_y = y.saturating_sub(1);
                let down_y = (y + 1).min(height - 1);

                let px_left = pixels[y * original_width + left_x];
                let px_right = pixels[y * original_width + right_x];
                let px_up = pixels[up_y * original_width + x];
                let px_down = pixels[down_y * original_width + x];

                let lum_left = i32::from(pixel_luminance(px_left));
                let lum_right = i32::from(pixel_luminance(px_right));
                let lum_up = i32::from(pixel_luminance(px_up));
                let lum_down = i32::from(pixel_luminance(px_down));

                let dx = lum_right - lum_left;
                let dy = lum_down - lum_up;

                // Simple squared gradient energy
                energy[y * effective_width + x] = (dx * dx + dy * dy) as u32;
            }
        }

        // 2. Dynamic Programming: Find lowest energy path
        // Initialize first row costs
        for x in 0..effective_width {
            cost[x] = energy[x];
        }

        for y in 1..height {
            for x in 0..effective_width {
                let e = energy[y * effective_width + x];
                let prev_y = y - 1;

                // Check top-left, top, top-right
                let cost_mid = cost[prev_y * effective_width + x];

                let cost_left = if x > 0 {
                    cost[prev_y * effective_width + (x - 1)]
                } else {
                    u32::MAX
                };

                let cost_right = if x + 1 < effective_width {
                    cost[prev_y * effective_width + (x + 1)]
                } else {
                    u32::MAX
                };

                // Find the minimum cost and record the path
                let min_cost;
                let offset;

                if cost_left <= cost_mid && cost_left <= cost_right {
                    min_cost = cost_left;
                    offset = -1;
                } else if cost_mid <= cost_left && cost_mid <= cost_right {
                    min_cost = cost_mid;
                    offset = 0;
                } else {
                    min_cost = cost_right;
                    offset = 1;
                }

                cost[y * effective_width + x] = e.saturating_add(min_cost);
                path[y * effective_width + x] = offset;
            }
        }

        // 3. Find the end of the minimum seam at the bottom row
        let mut min_seam_cost = u32::MAX;
        let mut min_seam_x = 0;

        let bottom_y = height - 1;
        for x in 0..effective_width {
            let c = cost[bottom_y * effective_width + x];
            if c < min_seam_cost {
                min_seam_cost = c;
                min_seam_x = x;
            }
        }

        // 4. Backtrack to find the seam and remove it
        let mut current_x = min_seam_x as i32;

        for y in (0..height).rev() {
            let seam_x = current_x as usize;

            // Shift pixels left to overwrite the seam
            for x in seam_x..(effective_width - 1) {
                pixels[y * original_width + x] = pixels[y * original_width + x + 1];
            }

            // Fill the newly exposed rightmost pixel of the effective bounds with black
            // (or transparency depending on use case, we use black to clear the data)
            pixels[y * original_width + (effective_width - 1)] = 0xFF_00_00_00;

            if y > 0 {
                let offset = path[y * effective_width + seam_x];
                current_x += offset;
                // Safety clamp, shouldn't happen with correct pathing
                current_x = current_x.clamp(0, (effective_width - 1) as i32);
            }
        }

        // 5. Decrement effective width
        effective_width -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_seam_carving_reduces_effective_width() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // Create a 5x5 image.
        // We'll make a vertical stripe of high energy (white) in the middle,
        // and low energy (black) on the sides.
        // The seam carver should remove the black paths first.
        fb.clear(0xFF_00_00_00);
        for y in 0..5 {
            fb.set_pixel(2, y, 0xFF_FF_FF_FF); // White stripe at x=2
        }

        apply_seam_carving(&mut fb, 2);

        // The framebuffer still reports 5x5 capacity, but effective width is 3.
        // The rightmost 2 columns should be black due to shifting.
        // The white stripe should have moved. Let's check the contents.
        let pixels = fb.as_slice();
        let mut found_white = false;

        for y in 0..5 {
            // Check that the rightmost 2 columns are cleared
            assert_eq!(pixels[y * 5 + 3], 0xFF_00_00_00, "Column 3 should be empty");
            assert_eq!(pixels[y * 5 + 4], 0xFF_00_00_00, "Column 4 should be empty");

            // Look for the white stripe in the remaining 3 columns
            for x in 0..3 {
                if pixels[y * 5 + x] == 0xFF_FF_FF_FF {
                    found_white = true;
                }
            }
        }

        assert!(
            found_white,
            "The high-energy white stripe should be preserved"
        );
    }

    #[test]
    fn test_apply_seam_carving_all_white() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFF_FF_FF_FF);

        apply_seam_carving(&mut fb, 1);

        // One column removed, so rightmost column should be black
        for y in 0..4 {
            assert_eq!(fb.get_pixel(3, y as i32).unwrap(), 0xFF_00_00_00);
        }
    }
}
