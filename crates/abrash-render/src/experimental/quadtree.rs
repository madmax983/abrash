#![cfg(feature = "nova")]

//! Quadtree stylization filter.
//!
//! Recursively subdivides the image into quadrants based on color variance.

use abrash_core::framebuffer::Framebuffer;

fn process_quad(
    fb: &mut Framebuffer,
    x: u32,
    y: u32,
    size: u32,
    threshold: u32,
    min_size: u32,
    draw_borders: bool,
    border_color: u32,
) {
    if size <= min_size {
        return;
    }

    let mut sum_r: u64 = 0;
    let mut sum_g: u64 = 0;
    let mut sum_b: u64 = 0;
    let mut max_r: u32 = 0;
    let mut max_g: u32 = 0;
    let mut max_b: u32 = 0;
    let mut min_r: u32 = 255;
    let mut min_g: u32 = 255;
    let mut min_b: u32 = 255;

    let width = fb.width();
    let height = fb.height();

    let end_x = (x + size).min(width);
    let end_y = (y + size).min(height);

    if x >= width || y >= height {
        return;
    }

    let actual_width = end_x - x;
    let actual_height = end_y - y;
    let pixel_count = u64::from(actual_width * actual_height);

    if pixel_count == 0 {
        return;
    }

    for py in y..end_y {
        for px in x..end_x {
            if let Some(color) = fb.get_pixel(px as i32, py as i32) {
                let r = (color >> 16) & 0xFF;
                let g = (color >> 8) & 0xFF;
                let b = color & 0xFF;

                sum_r += u64::from(r);
                sum_g += u64::from(g);
                sum_b += u64::from(b);

                max_r = max_r.max(r);
                max_g = max_g.max(g);
                max_b = max_b.max(b);

                min_r = min_r.min(r);
                min_g = min_g.min(g);
                min_b = min_b.min(b);
            }
        }
    }

    let diff_r = max_r - min_r;
    let diff_g = max_g - min_g;
    let diff_b = max_b - min_b;
    let max_diff = diff_r.max(diff_g).max(diff_b);

    if max_diff > threshold {
        let half = size / 2;
        process_quad(
            fb,
            x,
            y,
            half,
            threshold,
            min_size,
            draw_borders,
            border_color,
        );
        process_quad(
            fb,
            x + half,
            y,
            half,
            threshold,
            min_size,
            draw_borders,
            border_color,
        );
        process_quad(
            fb,
            x,
            y + half,
            half,
            threshold,
            min_size,
            draw_borders,
            border_color,
        );
        process_quad(
            fb,
            x + half,
            y + half,
            half,
            threshold,
            min_size,
            draw_borders,
            border_color,
        );
    } else {
        let avg_r = (sum_r / pixel_count) as u32;
        let avg_g = (sum_g / pixel_count) as u32;
        let avg_b = (sum_b / pixel_count) as u32;
        let avg_color = 0xFF00_0000 | (avg_r << 16) | (avg_g << 8) | avg_b;

        for py in y..end_y {
            for px in x..end_x {
                if draw_borders && (px == x || px == end_x - 1 || py == y || py == end_y - 1) {
                    fb.set_pixel(px as i32, py as i32, border_color);
                } else {
                    fb.set_pixel(px as i32, py as i32, avg_color);
                }
            }
        }
    }
}

/// Applies a quadtree compression/stylization filter to the framebuffer.
///
/// Recursively subdivides the image based on the maximum color difference within a region.
/// If the difference is below the threshold, the region is filled with its average color.
///
/// * `fb` - The framebuffer to modify
/// * `threshold` - The maximum color difference allowed before subdivision (0-255)
/// * `min_size` - The smallest allowed quad size in pixels
/// * `draw_borders` - Whether to draw borders around the quads
/// * `border_color` - The ARGB color of the borders
pub fn apply_quadtree(
    fb: &mut Framebuffer,
    threshold: u32,
    min_size: u32,
    draw_borders: bool,
    border_color: u32,
) {
    let width = fb.width();
    let height = fb.height();
    let size = width.max(height);

    let mut po2_size = 1;
    while po2_size < size {
        po2_size *= 2;
    }

    process_quad(
        fb,
        0,
        0,
        po2_size,
        threshold,
        min_size,
        draw_borders,
        border_color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_quadtree_subdivision() {
        let mut fb = Framebuffer::new(8, 8).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                if x < 4 {
                    fb.set_pixel(x, y, 0xFF00_0000);
                } else {
                    fb.set_pixel(x, y, 0xFFFF_FFFF);
                }
            }
        }

        apply_quadtree(&mut fb, 10, 2, false, 0);

        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(7, 0), Some(0xFFFF_FFFF));
    }
}
