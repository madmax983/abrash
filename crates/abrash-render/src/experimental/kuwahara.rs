//! Kuwahara Filter Module
//!
//! A non-photorealistic post-processing filter that gives images a painterly,
//! oil-painting-like aesthetic while preserving hard edges.
//!
//! The filter works by calculating the mean and variance of colors in four
//! overlapping rectangular regions around each pixel. The pixel is then assigned
//! the mean color of the region with the lowest variance.
//!
//! Each region's sum/sum-of-squares is computed in O(1) via summed-area
//! tables (integral images) built once per call, instead of re-summing every
//! pixel in the (up to `radius+1`-per-side) region from scratch for every
//! output pixel. See `build_summed_area_tables`.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static KUWAHARA_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static KUWAHARA_SAT: RefCell<SatTables> = RefCell::new(SatTables::new());
}

/// Summed-area tables (integral images) for R, G, B and their squares.
///
/// Each table has shape `(height + 1) x (width + 1)` (row-major, `stride =
/// width + 1`), with a zeroed leading row/column so that an inclusive
/// rectangle sum `[x0..=x1] x [y0..=y1]` is a plain 4-point lookup:
/// `table[(y1+1)*stride + (x1+1)] - table[y0*stride + (x1+1)]
///     - table[(y1+1)*stride + x0] + table[y0*stride + x0]`.
struct SatTables {
    sum_r: Vec<u64>,
    sum_g: Vec<u64>,
    sum_b: Vec<u64>,
    sum_r2: Vec<u64>,
    sum_g2: Vec<u64>,
    sum_b2: Vec<u64>,
}

impl SatTables {
    const fn new() -> Self {
        Self {
            sum_r: Vec::new(),
            sum_g: Vec::new(),
            sum_b: Vec::new(),
            sum_r2: Vec::new(),
            sum_g2: Vec::new(),
            sum_b2: Vec::new(),
        }
    }

    const fn tables_mut(&mut self) -> [&mut Vec<u64>; 6] {
        [
            &mut self.sum_r,
            &mut self.sum_g,
            &mut self.sum_b,
            &mut self.sum_r2,
            &mut self.sum_g2,
            &mut self.sum_b2,
        ]
    }

    /// Ensures every table is at least `len` elements long, and zeroes the guard
    /// row (index `0..stride`) and guard column (index `y * stride` for each
    /// row) so stale data from a previous, differently-shaped call can never
    /// leak into a rectangle-sum lookup.
    fn reset(&mut self, width: usize, height: usize) {
        let stride = width + 1;
        let len = stride * (height + 1);
        for table in self.tables_mut() {
            if table.len() < len {
                table.resize(len, 0);
            }
            table[..stride].fill(0);
            for y in 0..=height {
                table[y * stride] = 0;
            }
        }
    }
}

/// Builds all 6 summed-area tables from `src` (a `width x height` pixel
/// buffer in `0xAARRGGBB` order) in a single pass.
fn build_summed_area_tables(src: &[u32], width: usize, height: usize, sat: &mut SatTables) {
    sat.reset(width, height);
    let stride = width + 1;

    for y in 0..height {
        let mut running_r = 0u64;
        let mut running_g = 0u64;
        let mut running_b = 0u64;
        let mut running_r2 = 0u64;
        let mut running_g2 = 0u64;
        let mut running_b2 = 0u64;

        let src_row = y * width;
        let row_above = y * stride;
        let row_cur = (y + 1) * stride;

        for x in 0..width {
            let pixel = src[src_row + x];
            let r = u64::from((pixel >> 16) & 0xFF);
            let g = u64::from((pixel >> 8) & 0xFF);
            let b = u64::from(pixel & 0xFF);

            running_r += r;
            running_g += g;
            running_b += b;
            running_r2 += r * r;
            running_g2 += g * g;
            running_b2 += b * b;

            let idx = row_cur + x + 1;
            let idx_above = row_above + x + 1;
            sat.sum_r[idx] = sat.sum_r[idx_above] + running_r;
            sat.sum_g[idx] = sat.sum_g[idx_above] + running_g;
            sat.sum_b[idx] = sat.sum_b[idx_above] + running_b;
            sat.sum_r2[idx] = sat.sum_r2[idx_above] + running_r2;
            sat.sum_g2[idx] = sat.sum_g2[idx_above] + running_g2;
            sat.sum_b2[idx] = sat.sum_b2[idx_above] + running_b2;
        }
    }
}

/// Inclusive rectangle sum `[x0..=x1] x [y0..=y1]` from a summed-area table.
#[inline]
fn rect_sum(table: &[u64], stride: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> u64 {
    // Grouped as (bottom-right + top-left) - (top-right + bottom-left) rather
    // than the textbook left-to-right `A - B - C + D`: both groups are pure
    // additions, so the final subtraction is the only one performed, and it
    // can never underflow (the true rectangle sum is always >= 0), unlike an
    // intermediate `A - B - C` which can dip negative before `+ D` corrects it.
    (table[(y1 + 1) * stride + (x1 + 1)] + table[y0 * stride + x0])
        - (table[y0 * stride + (x1 + 1)] + table[(y1 + 1) * stride + x0])
}

#[allow(clippy::too_many_arguments)]
fn process_kuwahara_region(
    sat: &SatTables,
    stride: usize,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    dx_start: i32,
    dx_end: i32,
    dy_start: i32,
    dy_end: i32,
    best_num: &mut u64,
    best_den: &mut u64,
    best_color: &mut u32,
) {
    // Same clamping as a bounds-checked rectangle: for interior pixels (the
    // common case) this is a no-op, since the unclamped range is already
    // within `[0, width) x [0, height)`, exactly matching the original
    // interior fast-path's unclamped range.
    let px_start = (x + dx_start).clamp(0, width - 1) as usize;
    let px_end = (x + dx_end).clamp(0, width - 1) as usize;
    let py_start = (y + dy_start).clamp(0, height - 1) as usize;
    let py_end = (y + dy_end).clamp(0, height - 1) as usize;

    let count = ((px_end - px_start + 1) * (py_end - py_start + 1)) as u32;

    if count > 0 {
        let sum_r = rect_sum(&sat.sum_r, stride, px_start, py_start, px_end, py_end) as u32;
        let sum_g = rect_sum(&sat.sum_g, stride, px_start, py_start, px_end, py_end) as u32;
        let sum_b = rect_sum(&sat.sum_b, stride, px_start, py_start, px_end, py_end) as u32;
        let sum_r2 = rect_sum(&sat.sum_r2, stride, px_start, py_start, px_end, py_end) as u32;
        let sum_g2 = rect_sum(&sat.sum_g2, stride, px_start, py_start, px_end, py_end) as u32;
        let sum_b2 = rect_sum(&sat.sum_b2, stride, px_start, py_start, px_end, py_end) as u32;

        let count_u64 = u64::from(count);
        let sum_r_sq = u64::from(sum_r) * u64::from(sum_r);
        let sum_g_sq = u64::from(sum_g) * u64::from(sum_g);
        let sum_b_sq = u64::from(sum_b) * u64::from(sum_b);

        let scaled_var_r = count_u64 * u64::from(sum_r2) - sum_r_sq;
        let scaled_var_g = count_u64 * u64::from(sum_g2) - sum_g_sq;
        let scaled_var_b = count_u64 * u64::from(sum_b2) - sum_b_sq;

        let total_variance_num = scaled_var_r + scaled_var_g + scaled_var_b;
        let total_variance_den = count_u64 * count_u64;

        if u128::from(total_variance_num) * u128::from(*best_den)
            < u128::from(*best_num) * u128::from(total_variance_den)
        {
            *best_num = total_variance_num;
            *best_den = total_variance_den;

            let out_r = sum_r / count;
            let out_g = sum_g / count;
            let out_b = sum_b / count;

            *best_color = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
        }
    }
}

pub fn apply_kuwahara(fb: &mut Framebuffer, radius: i32) {
    if radius <= 0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // We must read from the original pixels and write to a new buffer
    // since the filter requires unmodified neighboring pixels.

    KUWAHARA_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = (width * height) as usize;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        KUWAHARA_SAT.with(|sat_cell| {
            let mut sat = sat_cell.borrow_mut();
            build_summed_area_tables(src_fb, width as usize, height as usize, &mut sat);
            // Rebind as a plain shared reference (rather than `RefMut`) so it can
            // be safely captured by the `parallel` feature's `rayon` closure below.
            let sat: &SatTables = &sat;
            let stride = width as usize + 1;

            let dest_pixels = fb.as_mut_slice();

            #[cfg(feature = "parallel")]
            {
                dest_pixels
                    .par_chunks_exact_mut(width as usize)
                    .enumerate()
                    .for_each(|(y_usize, row)| {
                        let y = y_usize as i32;

                        for (x_usize, pixel_out) in row.iter_mut().enumerate() {
                            let x = x_usize as i32;

                            // The four regions around the center pixel (x, y):
                            // 0: Top-Left
                            // 1: Top-Right
                            // 2: Bottom-Left
                            // 3: Bottom-Right
                            let mut best_num = u64::MAX;
                            let mut best_den = 1u64;
                            let mut best_color = 0u32;

                            let regions = [
                                (-radius, 0, -radius, 0),
                                (0, radius, -radius, 0),
                                (-radius, 0, 0, radius),
                                (0, radius, 0, radius),
                            ];

                            for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                                process_kuwahara_region(
                                    &sat,
                                    stride,
                                    width,
                                    height,
                                    x,
                                    y,
                                    dx_start,
                                    dx_end,
                                    dy_start,
                                    dy_end,
                                    &mut best_num,
                                    &mut best_den,
                                    &mut best_color,
                                );
                            }

                            *pixel_out = best_color;
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

                        for (x_usize, pixel_out) in row.iter_mut().enumerate() {
                            let x = x_usize as i32;

                            // The four regions around the center pixel (x, y):
                            // 0: Top-Left
                            // 1: Top-Right
                            // 2: Bottom-Left
                            // 3: Bottom-Right
                            let mut best_num = u64::MAX;
                            let mut best_den = 1u64;
                            let mut best_color = 0u32;

                            let regions = [
                                (-radius, 0, -radius, 0),
                                (0, radius, -radius, 0),
                                (-radius, 0, 0, radius),
                                (0, radius, 0, radius),
                            ];

                            for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                                process_kuwahara_region(
                                    &sat,
                                    stride,
                                    width,
                                    height,
                                    x,
                                    y,
                                    dx_start,
                                    dx_end,
                                    dy_start,
                                    dy_end,
                                    &mut best_num,
                                    &mut best_den,
                                    &mut best_color,
                                );
                            }

                            *pixel_out = best_color;
                        }
                    });
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_kuwahara_bounds() {
        // Test with different framebuffer sizes to ensure no out-of-bounds panics
        let sizes = [(1, 1), (10, 10), (100, 1), (1, 100), (33, 47)];

        for (w, h) in sizes {
            let mut fb = Framebuffer::new(w, h).unwrap();
            fb.clear(0xFFFF_FFFF);

            // Should not panic with varying radii
            for radius in [1, 2, 5, 10] {
                apply_kuwahara(&mut fb, radius);
            }
        }
    }
}
