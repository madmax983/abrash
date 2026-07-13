use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static OIL_PAINT_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn apply_oil_paint(fb: &mut Framebuffer, radius: i32, intensity_levels: i32) {
    if radius <= 0 || intensity_levels <= 0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    OIL_PAINT_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = (width * height) as usize;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();
        let levels = intensity_levels as u32;

        #[cfg(feature = "parallel")]
        {
            dest_pixels
                .par_chunks_exact_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;
                    // Pre-allocate arrays to avoid inner loop allocations
                    let mut intensity_count = vec![0; intensity_levels as usize];
                    let mut sum_r = vec![0; intensity_levels as usize];
                    let mut sum_g = vec![0; intensity_levels as usize];
                    let mut sum_b = vec![0; intensity_levels as usize];

                    for (x_usize, pixel_out) in row.iter_mut().enumerate() {
                        let x = x_usize as i32;

                        intensity_count.fill(0);
                        sum_r.fill(0);
                        sum_g.fill(0);
                        sum_b.fill(0);

                        let py_start = (y - radius).max(0);
                        let py_end = (y + radius).min(height - 1);
                        let px_start = (x - radius).max(0);
                        let px_end = (x + radius).min(width - 1);

                        for py in py_start..=py_end {
                            let row_offset = (py * width) as usize;
                            for px in px_start..=px_end {
                                let pixel = src_fb[row_offset + px as usize];

                                let r = (pixel >> 16) & 0xFF;
                                let g = (pixel >> 8) & 0xFF;
                                let b = pixel & 0xFF;

                                let luma = pixel_luminance(pixel);
                                let intensity_bin =
                                    ((u32::from(luma) * levels) / 256).min(levels - 1) as usize;

                                intensity_count[intensity_bin] += 1;
                                sum_r[intensity_bin] += r;
                                sum_g[intensity_bin] += g;
                                sum_b[intensity_bin] += b;
                            }
                        }

                        let mut max_count = 0;
                        let mut max_index = 0;

                        for (i, &count) in intensity_count.iter().enumerate() {
                            if count > max_count {
                                max_count = count;
                                max_index = i;
                            }
                        }

                        if max_count > 0 {
                            let final_r = sum_r[max_index] / max_count;
                            let final_g = sum_g[max_index] / max_count;
                            let final_b = sum_b[max_index] / max_count;

                            *pixel_out = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
                        }
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            dest_pixels
                .chunks_exact_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;
                    let mut intensity_count = vec![0; intensity_levels as usize];
                    let mut sum_r = vec![0; intensity_levels as usize];
                    let mut sum_g = vec![0; intensity_levels as usize];
                    let mut sum_b = vec![0; intensity_levels as usize];

                    for (x_usize, pixel_out) in row.iter_mut().enumerate() {
                        let x = x_usize as i32;

                        intensity_count.fill(0);
                        sum_r.fill(0);
                        sum_g.fill(0);
                        sum_b.fill(0);

                        let py_start = (y - radius).max(0);
                        let py_end = (y + radius).min(height - 1);
                        let px_start = (x - radius).max(0);
                        let px_end = (x + radius).min(width - 1);

                        for py in py_start..=py_end {
                            let row_offset = (py * width) as usize;
                            for px in px_start..=px_end {
                                let pixel = src_fb[row_offset + px as usize];

                                let r = (pixel >> 16) & 0xFF;
                                let g = (pixel >> 8) & 0xFF;
                                let b = pixel & 0xFF;

                                let luma = pixel_luminance(pixel);
                                let intensity_bin =
                                    ((u32::from(luma) * levels) / 256).min(levels - 1) as usize;

                                intensity_count[intensity_bin] += 1;
                                sum_r[intensity_bin] += r;
                                sum_g[intensity_bin] += g;
                                sum_b[intensity_bin] += b;
                            }
                        }

                        let mut max_count = 0;
                        let mut max_index = 0;

                        for (i, &count) in intensity_count.iter().enumerate() {
                            if count > max_count {
                                max_count = count;
                                max_index = i;
                            }
                        }

                        if max_count > 0 {
                            let final_r = sum_r[max_index] / max_count;
                            let final_g = sum_g[max_index] / max_count;
                            let final_b = sum_b[max_index] / max_count;

                            *pixel_out = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
                        }
                    }
                });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oil_paint_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        apply_oil_paint(&mut fb, 2, 20);
    }
}
