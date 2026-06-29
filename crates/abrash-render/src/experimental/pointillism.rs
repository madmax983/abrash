#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// Applies a pointillism painting effect to the framebuffer.
pub fn apply_pointillism(fb: &mut Framebuffer, dot_size: usize) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 || dot_size == 0 {
        return;
    }

    let original = fb.as_slice().to_vec();
    fb.clear(0xFF00_0000);
    let dest_pixels = fb.as_mut_slice();

    let step = dot_size;
    let radius = dot_size as i32 / 2;
    let radius_sq = radius * radius;

    let mut offsets = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius_sq {
                offsets.push((dx, dy));
            }
        }
    }

    let mut rng = XorShift32::new(1337);
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let jitter_x = (rng.next_u32() % (step as u32)) as i32 - radius;
            let jitter_y = (rng.next_u32() % (step as u32)) as i32 - radius;
            let cx = (x as i32 + jitter_x).clamp(0, width as i32 - 1);
            let cy = (y as i32 + jitter_y).clamp(0, height as i32 - 1);

            let color = original[cy as usize * width + cx as usize];
            let a = (color >> 24) & 0xFF;
            let mut r = (color >> 16) & 0xFF;
            let mut g = (color >> 8) & 0xFF;
            let mut b = color & 0xFF;

            // Apply slight color jitter (e.g. +/- 15)
            let color_jitter = (rng.next_u32() % 31) as i32 - 15;
            r = (r as i32 + color_jitter).clamp(0, 255) as u32;
            g = (g as i32 + color_jitter).clamp(0, 255) as u32;
            b = (b as i32 + color_jitter).clamp(0, 255) as u32;

            let jittered_color = (a << 24) | (r << 16) | (g << 8) | b;

            if cx >= radius
                && cx < (width as i32 - radius)
                && cy >= radius
                && cy < (height as i32 - radius)
            {
                for &(dx, dy) in &offsets {
                    let px = cx + dx;
                    let py = cy + dy;
                    dest_pixels[py as usize * width + px as usize] = jittered_color;
                }
            } else {
                for &(dx, dy) in &offsets {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        dest_pixels[py as usize * width + px as usize] = jittered_color;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointillism_changes_framebuffer() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFFFFFF);
        let original = fb.as_slice().to_vec();
        apply_pointillism(&mut fb, 5);
        assert_ne!(
            fb.as_slice(),
            original.as_slice(),
            "Framebuffer should be modified by pointillism"
        );
    }
}
