//! Retro Sun Generator
//!
//! Renders a synthwave/outrun style sunset sun with horizontal cutouts and a gradient.

use crate::framebuffer::Framebuffer;

pub fn render_retro_sun(
    fb: &mut Framebuffer,
    center_x: i32,
    center_y: i32,
    radius: i32,
    top_color: u32,
    bottom_color: u32,
) {
    if radius <= 0 || fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let r_sq = radius * radius;
    let diameter = radius * 2;
    let min_y = (center_y - radius).max(0);
    let max_y = (center_y + radius).min(fb.height() as i32 - 1);
    let min_x = (center_x - radius).max(0);
    let max_x = (center_x + radius).min(fb.width() as i32 - 1);

    // Extract colors for interpolation
    let top_r = ((top_color >> 16) & 0xFF) as f32;
    let top_g = ((top_color >> 8) & 0xFF) as f32;
    let top_b = (top_color & 0xFF) as f32;

    let bot_r = ((bottom_color >> 16) & 0xFF) as f32;
    let bot_g = ((bottom_color >> 8) & 0xFF) as f32;
    let bot_b = (bottom_color & 0xFF) as f32;

    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    for y in min_y..=max_y {
        let dy = y - center_y;
        let dy_sq = dy * dy;

        // Calculate gradient interpolation factor
        // Normalized Y from 0.0 (top) to 1.0 (bottom)
        let t = ((dy + radius) as f32) / (diameter as f32);

        // Synthwave sun horizontal cutouts
        // The cutouts should start from the middle/bottom and get progressively thicker
        // We can simulate this using a sine wave or mod over the normalized Y

        let mut draw = true;
        if t > 0.4 {
            // Normalize from 0.0 to 1.0 in the bottom 60%
            let cutout_t = (t - 0.4) / 0.6;

            // Generate stripes using modulo.
            // Spacing gets wider as we go down
            let spacing = 8.0 + cutout_t * 15.0;
            let phase = ((y as f32) % spacing) / spacing;

            // Thickness gets larger as we go down
            let thickness = 0.2 + cutout_t * 0.4;

            if phase < thickness {
                draw = false;
            }
        }

        if !draw {
            continue;
        }

        // Calculate interpolated color
        let r = (top_r + (bot_r - top_r) * t) as u32;
        let g = (top_g + (bot_g - top_g) * t) as u32;
        let b = (top_b + (bot_b - top_b) * t) as u32;
        let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;

        let row_start = (y as usize) * width;

        for x in min_x..=max_x {
            let dx = x - center_x;
            if dx * dx + dy_sq <= r_sq {
                pixels[row_start + x as usize] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_render_retro_sun() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);

        render_retro_sun(&mut fb, 50, 50, 40, 0xFF_FF_DD_00, 0xFF_FF_00_44);

        let mut modified = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF_00_00_00 {
                modified = true;
                break;
            }
        }

        assert!(modified, "retro sun should have modified the framebuffer");
    }
}
