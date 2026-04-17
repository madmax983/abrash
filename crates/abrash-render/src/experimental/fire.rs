//! Classic Demoscene Fire Effect
//!
//! Simulates a bottom-up fire using a cellular automaton approach with a cooling map.

use crate::framebuffer::Framebuffer;

/// Applies a classic demoscene fire effect to the framebuffer.
///
/// This effect relies on an underlying palette to map heat values (0-255) to RGB colors.
/// For each pixel, it averages the pixels directly below it, subtracts a cooling value,
/// and moves the result up one pixel.
///
/// * `fb` - The framebuffer to apply the effect to. The framebuffer stores the heat values in the red channel.
/// * `cooling_map` - A 1D array representing the cooling values for each pixel. Should be `width * height` in length.
/// ⚡ Bolt: Removed O(N) full buffer copy and unsafe parallelization by processing sequentially top-to-bottom.
/// Since row `y` reads from row `y+1` and `y+2`, iterating top-to-bottom sequentially avoids race conditions
/// and aliasing issues without needing double buffering.
pub fn apply_fire(fb: &mut Framebuffer, cooling_map: &[u8]) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height < 2 {
        return;
    }

    let pixels = fb.as_mut_slice();

    for y in 0..height - 1 {
        for x in 0..width {
            let left_x = x.saturating_sub(1);
            let right_x = (x + 1).min(width - 1);
            let below_y = y + 1;

            let p_center = pixels[below_y * width + x];
            let p_left = pixels[below_y * width + left_x];
            let p_right = pixels[below_y * width + right_x];
            let p_below2 = if below_y + 1 < height {
                pixels[(below_y + 1) * width + x]
            } else {
                p_center
            };

            let h_center = (p_center >> 16) & 0xFF;
            let h_left = (p_left >> 16) & 0xFF;
            let h_right = (p_right >> 16) & 0xFF;
            let h_below2 = (p_below2 >> 16) & 0xFF;

            // Shift by 2 is equivalent to divide by 4, but significantly faster
            let avg_heat = (h_center + h_left + h_right + h_below2) >> 2;

            let cooling = cooling_map[y * width + x] as u32;
            let new_heat = avg_heat.saturating_sub(cooling);

            pixels[y * width + x] = (new_heat << 16) | (new_heat << 8) | new_heat; // greyscale for now
        }
    }
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
