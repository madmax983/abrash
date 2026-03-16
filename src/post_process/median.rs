use crate::framebuffer::Framebuffer;

/// Applies a 3x3 Median filter to the framebuffer to reduce salt-and-pepper noise.
/// Processes RGB channels independently.
use rayon::prelude::*;

pub fn apply_median_filter(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Bolt Performance Optimization:
    // We avoid allocations by using a thread-local buffer instead of `.to_vec()` on every call,
    // which eliminates a per-frame heap allocation.
    thread_local! {
        static BUFFER: std::cell::RefCell<Vec<u32>> = std::cell::RefCell::new(Vec::new());
    }

    BUFFER.with(|buf_cell| {
        let mut original = buf_cell.take(); // take from cell to drop the non-Send RefMut
        original.clear();
        original.extend_from_slice(pixels);

        // Bolt Performance Optimization:
        // By using `par_chunks_exact_mut` combined with `.enumerate()`, we process row-by-row
        // in parallel, entirely eliminating implicit array bounds checking on `pixels`
        // and improving multi-core utilization.
        pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .skip(1)
            .take(height.saturating_sub(2))
            .for_each(|(y, row)| {
                let prev_row_base = (y - 1) * width;
                let cur_row_base = y * width;
                let next_row_base = (y + 1) * width;

                // Bolt Performance Optimization:
                // We use `.iter_mut().enumerate()` on the row slice (skipping the edges)
                // to eliminate inner bounds checking on the destination.
                row.iter_mut()
                    .enumerate()
                    .skip(1)
                    .take(width.saturating_sub(2))
                    .for_each(|(x, dest_pixel)| {
                        // Gather neighborhood using direct known offsets
                        let mut r_vals = [0u8; 9];
                        let mut g_vals = [0u8; 9];
                        let mut b_vals = [0u8; 9];

                        let p0 = original[prev_row_base + x - 1];
                        let p1 = original[prev_row_base + x];
                        let p2 = original[prev_row_base + x + 1];
                        let p3 = original[cur_row_base + x - 1];
                        let p4 = original[cur_row_base + x];
                        let p5 = original[cur_row_base + x + 1];
                        let p6 = original[next_row_base + x - 1];
                        let p7 = original[next_row_base + x];
                        let p8 = original[next_row_base + x + 1];

                        let mut gather = |vals: &mut [u8; 9], shift: u32| {
                            vals[0] = ((p0 >> shift) & 0xFF) as u8;
                            vals[1] = ((p1 >> shift) & 0xFF) as u8;
                            vals[2] = ((p2 >> shift) & 0xFF) as u8;
                            vals[3] = ((p3 >> shift) & 0xFF) as u8;
                            vals[4] = ((p4 >> shift) & 0xFF) as u8;
                            vals[5] = ((p5 >> shift) & 0xFF) as u8;
                            vals[6] = ((p6 >> shift) & 0xFF) as u8;
                            vals[7] = ((p7 >> shift) & 0xFF) as u8;
                            vals[8] = ((p8 >> shift) & 0xFF) as u8;
                            vals.sort_unstable();
                        };

                        gather(&mut r_vals, 16);
                        gather(&mut g_vals, 8);
                        gather(&mut b_vals, 0);

                        *dest_pixel = 0xFF000000 | ((r_vals[4] as u32) << 16) | ((g_vals[4] as u32) << 8) | (b_vals[4] as u32);
                    });
            });

        buf_cell.replace(original); // put it back safely
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_median_filter() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // A single bright white pixel in a black image (salt noise)
        fb.clear(0xFF000000); // Black
        let pixels = fb.as_mut_slice();
        pixels[4] = 0xFFFFFFFF; // White center pixel

        apply_median_filter(&mut fb);

        // After median filter, the bright pixel should be removed (replaced by median of neighborhood, which is black)
        let pixels = fb.as_slice();
        assert_eq!(pixels[4], 0xFF000000);
    }
}
