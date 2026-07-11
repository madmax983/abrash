//! Seam Carving Filter Module
//!
//! A post-processing feature for content-aware image resizing. It removes the path
//! of lowest energy (a "seam") from top to bottom, minimizing distortion of important details.

use abrash_core::framebuffer::Framebuffer;

/// Calculates the energy of a pixel using a simple dual-gradient energy function.
fn calculate_energy(fb: &Framebuffer, x: i32, y: i32, width: i32, height: i32) -> u32 {
    let left = fb.get_pixel((x - 1).max(0), y).unwrap_or(0);
    let right = fb.get_pixel((x + 1).min(width - 1), y).unwrap_or(0);
    let up = fb.get_pixel(x, (y - 1).max(0)).unwrap_or(0);
    let down = fb.get_pixel(x, (y + 1).min(height - 1)).unwrap_or(0);

    let rx = ((left >> 16) & 0xFF) as i32 - ((right >> 16) & 0xFF) as i32;
    let gx = ((left >> 8) & 0xFF) as i32 - ((right >> 8) & 0xFF) as i32;
    let bx = (left & 0xFF) as i32 - (right & 0xFF) as i32;

    let ry = ((up >> 16) & 0xFF) as i32 - ((down >> 16) & 0xFF) as i32;
    let gy = ((up >> 8) & 0xFF) as i32 - ((down >> 8) & 0xFF) as i32;
    let by = (up & 0xFF) as i32 - (down & 0xFF) as i32;

    ((rx * rx + gx * gx + bx * bx) + (ry * ry + gy * gy + by * by)) as u32
}

/// Finds and removes one vertical seam of lowest energy, reducing width by 1.
/// Returns a new Framebuffer.
///
/// # Panics
///
/// Panics if a new framebuffer of size `(width - 1, height)` fails to allocate,
/// which is highly unlikely given it's smaller than the input buffer.
#[must_use]
pub fn remove_vertical_seam(fb: &Framebuffer) -> Option<Framebuffer> {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    if width <= 1 {
        return None;
    }

    let mut energy_map = vec![0u32; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            energy_map[(y * width + x) as usize] = calculate_energy(fb, x, y, width, height);
        }
    }

    let mut cumulative_energy = energy_map.clone();
    for y in 1..height {
        for x in 0..width {
            let left = if x > 0 {
                cumulative_energy[((y - 1) * width + x - 1) as usize]
            } else {
                u32::MAX
            };
            let mid = cumulative_energy[((y - 1) * width + x) as usize];
            let right = if x < width - 1 {
                cumulative_energy[((y - 1) * width + x + 1) as usize]
            } else {
                u32::MAX
            };

            cumulative_energy[(y * width + x) as usize] += left.min(mid).min(right);
        }
    }

    let mut min_energy = u32::MAX;
    let mut min_x = 0;
    for x in 0..width {
        let energy = cumulative_energy[((height - 1) * width + x) as usize];
        if energy < min_energy {
            min_energy = energy;
            min_x = x;
        }
    }

    let mut seam = vec![0; height as usize];
    seam[(height - 1) as usize] = min_x;
    let mut current_x = min_x;

    for y in (0..height - 1).rev() {
        let left = if current_x > 0 {
            cumulative_energy[(y * width + current_x - 1) as usize]
        } else {
            u32::MAX
        };
        let mid = cumulative_energy[(y * width + current_x) as usize];
        let right = if current_x < width - 1 {
            cumulative_energy[(y * width + current_x + 1) as usize]
        } else {
            u32::MAX
        };

        if left <= mid && left <= right {
            current_x -= 1;
        } else if right <= mid && right <= left {
            current_x += 1;
        }
        seam[y as usize] = current_x;
    }

    let mut new_fb = Framebuffer::new((width - 1) as u32, height as u32).unwrap();
    for y in 0..height {
        let seam_x = seam[y as usize];
        let mut dest_x = 0;
        for x in 0..width {
            if x != seam_x {
                if let Some(pixel) = fb.get_pixel(x, y) {
                    new_fb.set_pixel(dest_x, y, pixel);
                }
                dest_x += 1;
            }
        }
    }

    Some(new_fb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_remove_vertical_seam() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_FFFF);

        // Draw a black vertical line down the middle
        for y in 0..10 {
            fb.set_pixel(5, y, 0xFF00_0000);
        }

        let new_fb = remove_vertical_seam(&fb).unwrap();
        assert_eq!(new_fb.width(), 9);
        assert_eq!(new_fb.height(), 10);
    }

    #[test]
    fn test_remove_seam_too_small() {
        let fb = Framebuffer::new(1, 10).unwrap();
        assert!(remove_vertical_seam(&fb).is_none());
    }
}
