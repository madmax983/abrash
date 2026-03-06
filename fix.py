import re

with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

# Replace rayon include and thread local
code = code.replace(
    "use crate::framebuffer::Framebuffer;\nuse rayon::prelude::*;",
    "use crate::framebuffer::Framebuffer;\n#[cfg(feature = \"parallel\")]\nuse rayon::prelude::*;\nuse std::cell::RefCell;\n\nthread_local! {\n    static KUWAHARA_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };\n}"
)

# Replace start of function
code = code.replace(
    """    let pixels = fb.as_slice();

    // We must read from the original pixels and write to a new buffer
    // since the filter requires unmodified neighboring pixels.
    let mut new_pixels = vec![0u32; (width * height) as usize];

    new_pixels
        .par_chunks_mut(width as usize)
        .enumerate()
        .for_each(|(y_usize, row)| {""",
    """    // We must read from the original pixels and write to a new buffer
    // since the filter requires unmodified neighboring pixels.
    KUWAHARA_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = (width * height) as usize;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            dest_pixels
                .par_chunks_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {"""
)

# Fix pixels.get_unchecked to src_fb.get_unchecked
code = code.replace(
    "let pixel = unsafe { *pixels.get_unchecked(row_offset + px as usize) };",
    "let pixel = unsafe { *src_fb.get_unchecked(row_offset + px as usize) };"
)

# Add scalar fallback
code = code.replace(
    """                row[x as usize] = best_color;
            }
        });

    // Copy the filtered pixels back to the framebuffer
    fb.as_mut_slice().copy_from_slice(&new_pixels);
}""",
    """                    row[x as usize] = best_color;
                }
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            dest_pixels
                .chunks_mut(width as usize)
                .enumerate()
                .for_each(|(y_usize, row)| {
                    let y = y_usize as i32;

                    for x in 0..width {
                        let mut min_variance = f32::MAX;
                        let mut best_color = 0u32;

                        let regions = [
                            (-radius, 0, -radius, 0),
                            (0, radius, -radius, 0),
                            (-radius, 0, 0, radius),
                            (0, radius, 0, radius),
                        ];

                        for &(dx_start, dx_end, dy_start, dy_end) in &regions {
                            let mut sum_r = 0;
                            let mut sum_g = 0;
                            let mut sum_b = 0;
                            let mut sum_r2 = 0;
                            let mut sum_g2 = 0;
                            let mut sum_b2 = 0;
                            let mut count = 0;

                            let py_start = (y + dy_start).max(0).min(height - 1);
                            let py_end = (y + dy_end).max(0).min(height - 1);
                            let px_start = (x + dx_start).max(0).min(width - 1);
                            let px_end = (x + dx_end).max(0).min(width - 1);

                            for py in py_start..=py_end {
                                let row_offset = (py * width) as usize;
                                for px in px_start..=px_end {
                                    let pixel = unsafe { *src_fb.get_unchecked(row_offset + px as usize) };

                                    let r = (pixel >> 16) & 0xFF;
                                    let g = (pixel >> 8) & 0xFF;
                                    let b = pixel & 0xFF;

                                    sum_r += r;
                                    sum_g += g;
                                    sum_b += b;

                                    sum_r2 += r * r;
                                    sum_g2 += g * g;
                                    sum_b2 += b * b;

                                    count += 1;
                                }
                            }

                            if count > 0 {
                                let count_u64 = u64::from(count);
                                let sum_r_sq = u64::from(sum_r) * u64::from(sum_r);
                                let sum_g_sq = u64::from(sum_g) * u64::from(sum_g);
                                let sum_b_sq = u64::from(sum_b) * u64::from(sum_b);

                                let scaled_var_r = count_u64 * u64::from(sum_r2) - sum_r_sq;
                                let scaled_var_g = count_u64 * u64::from(sum_g2) - sum_g_sq;
                                let scaled_var_b = count_u64 * u64::from(sum_b2) - sum_b_sq;

                                let total_variance = (scaled_var_r + scaled_var_g + scaled_var_b) as f32
                                    / (count_u64 * count_u64) as f32;

                                if total_variance < min_variance {
                                    min_variance = total_variance;

                                    let out_r = sum_r / count;
                                    let out_g = sum_g / count;
                                    let out_b = sum_b / count;

                                    best_color = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                                }
                            }
                        }

                        row[x as usize] = best_color;
                    }
                });
        }
    });
}"""
)

with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
