//! Content-Aware Seam Carving
//!
//! An experimental feature that reduces the width of a framebuffer
//! by finding and removing the lowest-energy vertical seams.

use abrash_core::framebuffer::Framebuffer;

/// Calculates a simple gradient energy for a pixel.
fn pixel_energy(fb: &Framebuffer, x: usize, y: usize) -> u32 {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    let x_left = if x > 0 { x - 1 } else { x };
    let x_right = if x + 1 < width { x + 1 } else { x };
    let y_up = if y > 0 { y - 1 } else { y };
    let y_down = if y + 1 < height { y + 1 } else { y };

    let get_rgb = |x, y| -> (i32, i32, i32) {
        let c = fb.get_pixel(x as i32, y as i32).unwrap_or(0);
        ((c >> 16 & 0xFF) as i32, (c >> 8 & 0xFF) as i32, (c & 0xFF) as i32)
    };

    let left = get_rgb(x_left, y);
    let right = get_rgb(x_right, y);
    let up = get_rgb(x, y_up);
    let down = get_rgb(x, y_down);

    let rx = left.0 - right.0;
    let gx = left.1 - right.1;
    let bx = left.2 - right.2;

    let ry = up.0 - down.0;
    let gy = up.1 - down.1;
    let by = up.2 - down.2;

    ((rx * rx + gx * gx + bx * bx) + (ry * ry + gy * gy + by * by)) as u32
}

/// Computes the cumulative energy map for vertical seam carving.
fn compute_energy_map(fb: &Framebuffer, current_width: usize) -> Vec<u32> {
    let height = fb.height() as usize;
    let mut energy = vec![0; current_width * height];

    // Compute energy for top row
    for x in 0..current_width {
        energy[x] = pixel_energy(fb, x, 0);
    }

    // Dynamic programming to compute cumulative energy
    for y in 1..height {
        for x in 0..current_width {
            let e = pixel_energy(fb, x, y);

            let mut min_prev = energy[(y - 1) * current_width + x];
            if x > 0 {
                min_prev = min_prev.min(energy[(y - 1) * current_width + x - 1]);
            }
            if x + 1 < current_width {
                min_prev = min_prev.min(energy[(y - 1) * current_width + x + 1]);
            }

            energy[y * current_width + x] = e + min_prev;
        }
    }

    energy
}

/// Finds the lowest energy vertical seam.
fn find_seam(energy_map: &[u32], width: usize, height: usize) -> Vec<usize> {
    let mut seam = vec![0; height];

    // Find the minimum energy pixel in the bottom row
    let mut min_x = 0;
    let mut min_energy = u32::MAX;
    let y = height - 1;
    for x in 0..width {
        let e = energy_map[y * width + x];
        if e < min_energy {
            min_energy = e;
            min_x = x;
        }
    }
    seam[height - 1] = min_x;

    // Backtrack to find the full seam
    for y in (0..height - 1).rev() {
        let x = seam[y + 1];
        let mut best_x = x;
        let mut min_e = energy_map[y * width + x];

        if x > 0 && energy_map[y * width + x - 1] < min_e {
            min_e = energy_map[y * width + x - 1];
            best_x = x - 1;
        }
        if x + 1 < width && energy_map[y * width + x + 1] < min_e {
            best_x = x + 1;
        }

        seam[y] = best_x;
    }

    seam
}

/// Reduces the width of the framebuffer by removing `num_seams` lowest energy vertical seams.
///
/// # Errors
///
/// Returns an error if attempting to remove more seams than the width of the image.
pub fn carve_vertical_seams(fb: &mut Framebuffer, num_seams: usize) -> Result<(), &'static str> {
    let mut width = fb.width() as usize;
    let height = fb.height() as usize;

    if width <= num_seams {
        return Err("Cannot remove more seams than the width of the image");
    }

    for _ in 0..num_seams {
        let energy_map = compute_energy_map(fb, width);
        let seam = find_seam(&energy_map, width, height);

        // Remove the seam by shifting pixels left
        let fb_width = fb.width() as usize;
        let pixels = fb.as_mut_slice();
        for y in 0..height {
            let sx = seam[y];
            let row_start = y * fb_width;

            for x in sx..width - 1 {
                pixels[row_start + x] = pixels[row_start + x + 1];
            }
        }
        width -= 1;
    }

    // Create a new framebuffer with the reduced width
    let mut new_fb = Framebuffer::new(width as u32, height as u32)?;
    let old_pixels = fb.as_slice();
    let new_pixels = new_fb.as_mut_slice();

    let old_stride = fb.width() as usize;
    for y in 0..height {
        for x in 0..width {
            new_pixels[y * width + x] = old_pixels[y * old_stride + x];
        }
    }

    *fb = new_fb;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_seam_carving() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Make the image uniform except for a high-energy vertical line
        for y in 0..4 {
            for x in 0..4 {
                if x == 1 {
                    fb.set_pixel(x, y, 0xFF_FFFFFF);
                } else {
                    fb.set_pixel(x, y, 0xFF_000000);
                }
            }
        }

        carve_vertical_seams(&mut fb, 1).unwrap();

        // Width should be reduced
        assert_eq!(fb.width(), 3);
        assert_eq!(fb.height(), 4);
    }

    #[test]
    fn test_seam_carving_too_many() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        assert!(carve_vertical_seams(&mut fb, 4).is_err());
    }
}
