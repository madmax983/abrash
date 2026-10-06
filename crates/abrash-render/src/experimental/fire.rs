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

    if cooling_map.len() < width * height {
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
            // Row slices of exactly `width` elements let the interior loop
            // run without per-pixel bounds checks or edge clamping.
            let below = &src_pixels[(y + 1) * width..(y + 2) * width];
            let below2 = if y + 2 < height {
                &src_pixels[(y + 2) * width..(y + 3) * width]
            } else {
                below
            };
            let cool = &cooling_map[y * width..(y + 1) * width];

            let heat = |p: u32| (p >> 16) & 0xFF;
            // Shift by 2 is equivalent to divide by 4, but significantly faster
            let shade = |h_sum: u32, cooling: u8| {
                let new_heat = (h_sum >> 2).saturating_sub(u32::from(cooling));
                (new_heat << 16) | (new_heat << 8) | new_heat // greyscale for now
            };

            // Edge columns clamp the left/right neighbour to the row bounds.
            let edge = |x: usize| {
                let h = heat(below[x])
                    + heat(below[x.saturating_sub(1)])
                    + heat(below[(x + 1).min(width - 1)])
                    + heat(below2[x]);
                shade(h, cool[x])
            };
            row[0] = edge(0);
            if width > 1 {
                row[width - 1] = edge(width - 1);
            }
            for x in 1..width.saturating_sub(1) {
                let h = heat(below[x - 1]) + heat(below[x]) + heat(below[x + 1]) + heat(below2[x]);
                row[x] = shade(h, cool[x]);
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
        fb.set_pixel(0, 2, 0x00FF_0000);
        fb.set_pixel(1, 2, 0x00FF_0000);
        fb.set_pixel(2, 2, 0x00FF_0000);

        // Zero cooling map
        let cooling_map = vec![0; 9];

        // Apply fire
        apply_fire(&mut fb, &cooling_map);

        // Heat should have propagated up to row 1
        let pixel = fb.get_pixel(1, 1).unwrap();
        let red = (pixel >> 16) & 0xFF;
        assert!(red > 0, "Heat did not propagate upwards");
    }

    #[test]
    fn test_apply_fire_does_not_panic_with_invalid_cooling_map() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let cooling_map = vec![0; 5]; // smaller than 100
        apply_fire(&mut fb, &cooling_map); // Should return early, not panic
    }
}
