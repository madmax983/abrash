use abrash_core::framebuffer::Framebuffer;

/// Carves vertical seams from the given framebuffer, reducing its width by `num_seams`.
/// The image height remains unchanged.
///
/// # Panics
/// Panics if `num_seams` is greater than or equal to the framebuffer's width.
#[must_use]
pub fn carve_seams(fb: &Framebuffer, num_seams: u32) -> Framebuffer {
    let original_width = fb.width() as usize;
    let mut width = original_width;
    let height = fb.height() as usize;
    assert!(
        num_seams < width as u32,
        "Cannot carve more seams than width"
    );

    // We copy the pixels into a mutable buffer where we can shift pixels left
    // We will use original_width as the row stride when accessing this buffer
    let mut pixels = fb.as_slice().to_vec();
    let mut energy = vec![0.0f32; width * height];
    let mut cumulative = vec![0.0f32; width * height];
    let mut path = vec![0usize; width * height];

    for _ in 0..num_seams {
        // 1. Calculate energy for current dimensions
        for y in 0..height {
            for x in 0..width {
                let x_l = x.saturating_sub(1);
                let x_r = (x + 1).min(width - 1);
                let y_u = y.saturating_sub(1);
                let y_d = (y + 1).min(height - 1);

                // Read pixels using original_width as stride
                let c = pixels[y * original_width + x];
                let cx_l = pixels[y * original_width + x_l];
                let cx_r = pixels[y * original_width + x_r];
                let cy_u = pixels[y_u * original_width + x];
                let cy_d = pixels[y_d * original_width + x];

                let c_r = ((c >> 16) & 0xFF) as f32;
                let c_g = ((c >> 8) & 0xFF) as f32;
                let c_b = (c & 0xFF) as f32;

                let l_r = ((cx_l >> 16) & 0xFF) as f32;
                let l_g = ((cx_l >> 8) & 0xFF) as f32;
                let l_b = (cx_l & 0xFF) as f32;

                let r_r = ((cx_r >> 16) & 0xFF) as f32;
                let r_g = ((cx_r >> 8) & 0xFF) as f32;
                let r_b = (cx_r & 0xFF) as f32;

                let u_r = ((cy_u >> 16) & 0xFF) as f32;
                let u_g = ((cy_u >> 8) & 0xFF) as f32;
                let u_b = (cy_u & 0xFF) as f32;

                let d_r = ((cy_d >> 16) & 0xFF) as f32;
                let d_g = ((cy_d >> 8) & 0xFF) as f32;
                let d_b = (cy_d & 0xFF) as f32;

                let ex = (l_r - c_r).powi(2)
                    + (l_g - c_g).powi(2)
                    + (l_b - c_b).powi(2)
                    + (r_r - c_r).powi(2)
                    + (r_g - c_g).powi(2)
                    + (r_b - c_b).powi(2);
                let ey = (u_r - c_r).powi(2)
                    + (u_g - c_g).powi(2)
                    + (u_b - c_b).powi(2)
                    + (d_r - c_r).powi(2)
                    + (d_g - c_g).powi(2)
                    + (d_b - c_b).powi(2);

                // Write to internal arrays using the current width as stride
                energy[y * width + x] = ex + ey;
            }
        }

        // 2. Cumulative energy map
        for x in 0..width {
            cumulative[x] = energy[x];
        }

        for y in 1..height {
            for x in 0..width {
                let mut min_e = cumulative[(y - 1) * width + x];

                let mut best_x = if x > 0 && cumulative[(y - 1) * width + x - 1] < min_e {
                    min_e = cumulative[(y - 1) * width + x - 1];
                    x - 1
                } else {
                    x
                };

                best_x = if x < width - 1 && cumulative[(y - 1) * width + x + 1] < min_e {
                    min_e = cumulative[(y - 1) * width + x + 1];
                    x + 1
                } else {
                    best_x
                };

                cumulative[y * width + x] = energy[y * width + x] + min_e;
                path[y * width + x] = best_x;
            }
        }

        // 3. Find minimum seam at the bottom row
        let mut min_val = f32::MAX;
        let mut min_idx = 0;
        for x in 0..width {
            if cumulative[(height - 1) * width + x] < min_val {
                min_val = cumulative[(height - 1) * width + x];
                min_idx = x;
            }
        }

        // 4. Backtrack and remove seam by shifting pixels left
        let mut current_x = min_idx;
        for y in (0..height).rev() {
            let row_start = y * original_width; // Shift within the fixed-stride buffer
            if current_x < width - 1 {
                pixels.copy_within(
                    (row_start + current_x + 1)..(row_start + width),
                    row_start + current_x,
                );
            }
            if y > 0 {
                current_x = path[(y - 1) * width + current_x]; // the path buffer is size height*width
            }
        }
        width -= 1;
    }

    // 5. Pack the result into a new framebuffer
    let mut result = Framebuffer::new(width as u32, height as u32).unwrap();
    let res_pixels = result.as_mut_slice();
    for y in 0..height {
        let src_start = y * original_width;
        let dst_start = y * width;
        res_pixels[dst_start..(dst_start + width)]
            .copy_from_slice(&pixels[src_start..(src_start + width)]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_seam_carving() {
        // Create a 5x5 image where the middle column (x=2) is a white line and everything else is black.
        // We expect seam carving to remove the black columns, leaving the white line intact.
        let mut fb = Framebuffer::new(5, 5).unwrap();
        fb.clear(0xFF00_0000u32);
        for y in 0..5 {
            fb.set_pixel(2, y, 0xFFFF_FFFFu32);
        }

        // Carve 2 seams
        let carved = carve_seams(&fb, 2);

        assert_eq!(carved.width(), 3);
        assert_eq!(carved.height(), 5);

        // Since we carved 2 lowest energy seams (black columns), the high energy white line should still exist
        // It might have shifted index, but there must be exactly 1 white pixel per row.
        for y in 0..5 {
            let mut white_count = 0;
            for x in 0..3 {
                if carved.get_pixel(x, y).unwrap() == 0xFFFF_FFFFu32 {
                    white_count += 1;
                }
            }
            assert_eq!(
                white_count, 1,
                "Row {} should have exactly one white pixel",
                y
            );
        }
    }
}
