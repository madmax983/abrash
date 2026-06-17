//! Content-Aware Image Resizing (Seam Carving).
//!
//! This module implements seam carving to resize an image by removing
//! the lowest-energy vertical seams.

/// Configuration for the seam carving algorithm.
#[derive(Debug, Clone, Copy)]
pub struct SeamCarvingConfig {
    /// Number of vertical seams to remove (reducing the width).
    pub seams_to_remove: u32,
    /// Width of the original image buffer.
    pub original_width: u32,
    /// The physical stride (pixels per row) of the buffer.
    pub stride: u32,
    /// Height of the image buffer.
    pub height: u32,
}

/// Applies seam carving to the given pixel buffer.
///
/// Modifies the buffer in-place by removing vertical seams. The effective
/// width of the image is reduced by `config.seams_to_remove`. The buffer
/// is not tightly repacked; gaps will exist at the end of each row.
///
/// # Arguments
///
/// * `buffer` - The image buffer (0xAARRGGBB format).
/// * `config` - Configuration specifying dimensions and seams to remove.
pub fn apply_seam_carving(buffer: &mut [u32], config: SeamCarvingConfig) {
    if config.seams_to_remove == 0 || config.original_width == 0 || config.height == 0 {
        return;
    }

    let mut effective_width = config.original_width as usize;
    let height = config.height as usize;
    let stride = config.stride as usize;

    let mut energy_map = vec![0u32; (config.stride * config.height) as usize];
    let mut cost_map = vec![0u32; (config.stride * config.height) as usize];
    let mut path_map = vec![0i32; (config.stride * config.height) as usize];

    for _ in 0..config.seams_to_remove {
        if effective_width <= 1 {
            break; // Cannot carve further
        }

        compute_energy_map(buffer, &mut energy_map, effective_width, height, stride);

        // Dynamic Programming: Calculate cumulative minimum energy paths
        // First row cost is just the energy
        for x in 0..effective_width {
            cost_map[x] = energy_map[x];
        }

        for y in 1..height {
            for x in 0..effective_width {
                let mut min_cost = cost_map[(y - 1) * stride + x];
                let mut min_path = 0; // 0: straight up, -1: up-left, 1: up-right

                if x > 0 {
                    let cost_left = cost_map[(y - 1) * stride + x - 1];
                    if cost_left < min_cost {
                        min_cost = cost_left;
                        min_path = -1;
                    }
                }

                if x < effective_width - 1 {
                    let cost_right = cost_map[(y - 1) * stride + x + 1];
                    if cost_right < min_cost {
                        min_cost = cost_right;
                        min_path = 1;
                    }
                }

                cost_map[y * stride + x] = energy_map[y * stride + x] + min_cost;
                path_map[y * stride + x] = min_path;
            }
        }

        // Find the end of the minimum energy seam at the bottom row
        let mut min_cost = u32::MAX;
        let mut min_x = 0;
        let last_row_idx = (height - 1) * stride;

        for x in 0..effective_width {
            let cost = cost_map[last_row_idx + x];
            if cost < min_cost {
                min_cost = cost;
                min_x = x;
            }
        }

        // Trace back and remove the seam
        let mut curr_x = min_x as i32;
        for y in (0..height).rev() {
            let row_start = y * stride;
            let cx = curr_x as usize;

            // Shift pixels left to overwrite the seam
            if cx < effective_width - 1 {
                for x in cx..(effective_width - 1) {
                    buffer[row_start + x] = buffer[row_start + x + 1];
                }
            }

            if y > 0 {
                curr_x += path_map[row_start + cx];
            }
        }

        // Decrease the effective width for the next pass
        effective_width -= 1;
    }
}

/// Computes the energy map for the image using a simplified Sobel-like filter.
fn compute_energy_map(
    buffer: &[u32],
    energy_map: &mut [u32],
    effective_width: usize,
    height: usize,
    stride: usize,
) {
    for y in 0..height {
        for x in 0..effective_width {
            let left_x = x.saturating_sub(1);
            let right_x = (x + 1).min(effective_width - 1);
            let top_y = y.saturating_sub(1);
            let bottom_y = (y + 1).min(height - 1);

            let px_left = buffer[y * stride + left_x];
            let px_right = buffer[y * stride + right_x];
            let px_top = buffer[top_y * stride + x];
            let px_bottom = buffer[bottom_y * stride + x];

            let dx = color_diff_sq(px_left, px_right);
            let dy = color_diff_sq(px_top, px_bottom);

            energy_map[y * stride + x] = dx + dy;
        }
    }
}

/// Calculates squared difference between two colors (AARRGGBB) ignoring alpha.
#[inline(always)]
fn color_diff_sq(c1: u32, c2: u32) -> u32 {
    let r1 = ((c1 >> 16) & 0xFF) as i32;
    let g1 = ((c1 >> 8) & 0xFF) as i32;
    let b1 = (c1 & 0xFF) as i32;

    let r2 = ((c2 >> 16) & 0xFF) as i32;
    let g2 = ((c2 >> 8) & 0xFF) as i32;
    let b2 = (c2 & 0xFF) as i32;

    let dr = r1 - r2;
    let dg = g1 - g2;
    let db = b1 - b2;

    (dr * dr + dg * dg + db * db) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seam_carving_reduces_width() {
        let mut buffer = vec![0xFFFFFFFF; 10 * 10]; // 10x10 white image
        let config = SeamCarvingConfig {
            seams_to_remove: 3,
            original_width: 10,
            stride: 10,
            height: 10,
        };

        // Draw a black vertical line at x=5
        for y in 0..10 {
            buffer[y * 10 + 5] = 0xFF000000;
        }

        apply_seam_carving(&mut buffer, config);

        // After removing 3 seams from a 10-wide image, effective width is 7.
        // The black line was at x=5, so 3 white columns to the left should be removed
        // because they all have the same low energy, moving the line to x=2.
        for y in 0..10 {
            assert_eq!(buffer[y * 10 + 2], 0xFF000000, "Black line should have shifted to x=2");
        }
    }

    #[test]
    fn test_seam_carving_preserves_features() {
         // Create a 5x5 image
         let mut buffer = vec![
             0xFF000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFF000000,
             0xFF000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFF000000,
             0xFF000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFF000000,
             0xFF000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFF000000,
             0xFF000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFF000000,
         ];

         let config = SeamCarvingConfig {
            seams_to_remove: 1,
            original_width: 5,
            stride: 5,
            height: 5,
        };

        apply_seam_carving(&mut buffer, config);

        // The lowest energy seam should be one of the white columns in the middle.
        // We expect the black columns at x=0 and x=4 to be preserved, meaning the black
        // column at x=4 shifts to x=3.
        for y in 0..5 {
            assert_eq!(buffer[y * 5], 0xFF000000); // Original left edge
            assert_eq!(buffer[y * 5 + 3], 0xFF000000); // Shifted right edge
        }
    }
}
