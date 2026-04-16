//! Classic Demoscene Fire Effect
//!
//! Simulates a bottom-up fire using a cellular automaton approach with a cooling map.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a classic demoscene fire effect to the framebuffer.
///
/// This effect relies on an underlying palette to map heat values (0-255) to RGB colors.
/// For each pixel, it averages the pixels directly below it, subtracts a cooling value,
/// and moves the result up one pixel.
///
/// * `fb` - The framebuffer to apply the effect to. The framebuffer stores the heat values in the red channel.
/// * `cooling_map` - A 1D array representing the cooling values for each pixel. Should be `width * height` in length.
pub fn apply_fire(fb: &mut Framebuffer, cooling_map: &[u8]) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height < 2 {
        return;
    }

    thread_local! {
        static SRC_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    SRC_BUFFER.with(|src_buf| {
        let mut src_vec = src_buf.borrow_mut();
        let size = width * height;
        if src_vec.len() != size {
            src_vec.resize(size, 0);
        }
        src_vec.copy_from_slice(fb.as_slice());
        let src_pixels = src_vec.as_slice();

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let chunk_iter = dest_pixels[..width * (height - 1)]
            .par_chunks_exact_mut(width)
            .enumerate();
        #[cfg(not(feature = "parallel"))]
        let chunk_iter = dest_pixels[..width * (height - 1)]
            .chunks_exact_mut(width)
            .enumerate();

chunk_iter.for_each(|(y, row)| {
            let below_y = y + 1;
            let below_idx = below_y * width;
            let below2_idx = if below_y + 1 < height {
                (below_y + 1) * width
            } else {
                below_idx
            };

            let src_below = &src_pixels[below_idx..below_idx + width];
            let src_below2 = &src_pixels[below2_idx..below2_idx + width];
            let cooling_row = &cooling_map[y * width..y * width + width];

            // Edge cases
            if width > 0 {
                // Left edge
                let p_center = src_below[0];
                let p_left = src_below[0];
                let p_right = if width > 1 { src_below[1] } else { p_center };
                let p_below2 = src_below2[0];

                let h_center = (p_center >> 16) & 0xFF;
                let h_left = (p_left >> 16) & 0xFF;
                let h_right = (p_right >> 16) & 0xFF;
                let h_below2 = (p_below2 >> 16) & 0xFF;
                let avg_heat = (h_center + h_left + h_right + h_below2) >> 2;
                let cooling = cooling_row[0] as u32;
                let new_heat = avg_heat.saturating_sub(cooling);
                row[0] = (new_heat << 16) | (new_heat << 8) | new_heat;
            }

            // Middle
            if width > 2 {
                for x in 1..width - 1 {
                    let p_center = src_below[x];
                    let p_left = src_below[x - 1];
                    let p_right = src_below[x + 1];
                    let p_below2 = src_below2[x];

                    let h_center = (p_center >> 16) & 0xFF;
                    let h_left = (p_left >> 16) & 0xFF;
                    let h_right = (p_right >> 16) & 0xFF;
                    let h_below2 = (p_below2 >> 16) & 0xFF;
                    let avg_heat = (h_center + h_left + h_right + h_below2) >> 2;
                    let cooling = cooling_row[x] as u32;
                    let new_heat = avg_heat.saturating_sub(cooling);
                    row[x] = (new_heat << 16) | (new_heat << 8) | new_heat;
                }
            }

            // Right edge
            if width > 1 {
                let x = width - 1;
                let p_center = src_below[x];
                let p_left = src_below[x - 1];
                let p_right = src_below[x];
                let p_below2 = src_below2[x];

                let h_center = (p_center >> 16) & 0xFF;
                let h_left = (p_left >> 16) & 0xFF;
                let h_right = (p_right >> 16) & 0xFF;
                let h_below2 = (p_below2 >> 16) & 0xFF;
                let avg_heat = (h_center + h_left + h_right + h_below2) >> 2;
                let cooling = cooling_row[x] as u32;
                let new_heat = avg_heat.saturating_sub(cooling);
                row[x] = (new_heat << 16) | (new_heat << 8) | new_heat;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_fire_moves_heat_up() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Heat is stored in the R channel (0x00RRGGBB).
        // Let's set the bottom row to max heat.
        fb.set_pixel(0, 2, 0x00FF0000);
        fb.set_pixel(1, 2, 0x00FF0000);
        fb.set_pixel(2, 2, 0x00FF0000);

        // Zero cooling map
        let cooling_map = vec![0; 9];

        // Apply fire
        apply_fire(&mut fb, &cooling_map);

        // Heat should have propagated up to row 1
        let pixel = fb.get_pixel(1, 1).unwrap();
        let red = (pixel >> 16) & 0xFF;
        assert!(red > 0, "Heat did not propagate upwards");
    }
}
