//! Bresenham's circle drawing algorithms.
//!
//! Implements integer-based circle rasterization.

use crate::framebuffer::Framebuffer;

/// Draw an empty circle using Bresenham's algorithm.
///
/// # Arguments
///
/// * `fb` - Target framebuffer.
/// * `xc`, `yc` - Center coordinates of the circle.
/// * `radius` - Radius of the circle.
/// * `color` - 0xAARRGGBB color value.
pub fn draw_circle(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, color: u32) {
    if radius < 0 {
        return;
    }

    let mut x = 0;
    let mut y = radius;
    let mut d = 3 - 2 * radius;

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // Fast path: check if the entire bounding box of the circle is within the framebuffer
    let left = xc.saturating_sub(radius);
    let right = xc.saturating_add(radius);
    let top = yc.saturating_sub(radius);
    let bottom = yc.saturating_add(radius);

    let is_fully_on_screen = left >= 0 && right < width && top >= 0 && bottom < height;

    if is_fully_on_screen {
        // SAFETY: We just verified the entire circle's bounding box is well within the
        // valid range of [0, width) and [0, height). `xc ± x` and `yc ± y` will always
        // be within [left, right] and [top, bottom].
        unsafe {
            draw_circle_points_unchecked(fb, xc, yc, x, y, color);

            while y >= x {
                x += 1;
                if d > 0 {
                    y -= 1;
                    d = d + 4 * (x - y) + 10;
                } else {
                    d = d + 4 * x + 6;
                }
                draw_circle_points_unchecked(fb, xc, yc, x, y, color);
            }
        }
    } else {
        // Slow path: circle is partially or entirely off-screen
        draw_circle_points(fb, xc, yc, x, y, color);

        while y >= x {
            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
            draw_circle_points(fb, xc, yc, x, y, color);
        }
    }
}

/// Draws circle points without bounds checking.
///
/// # Safety
/// Caller must guarantee that all points (xc ± x, yc ± y) and (xc ± y, yc ± x)
/// are within the framebuffer bounds.
unsafe fn draw_circle_points_unchecked(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    fb.set_pixel_unchecked((xc + x) as usize, (yc + y) as usize, color);
    fb.set_pixel_unchecked((xc - x) as usize, (yc + y) as usize, color);
    fb.set_pixel_unchecked((xc + x) as usize, (yc - y) as usize, color);
    fb.set_pixel_unchecked((xc - x) as usize, (yc - y) as usize, color);
    fb.set_pixel_unchecked((xc + y) as usize, (yc + x) as usize, color);
    fb.set_pixel_unchecked((xc - y) as usize, (yc + x) as usize, color);
    fb.set_pixel_unchecked((xc + y) as usize, (yc - x) as usize, color);
    fb.set_pixel_unchecked((xc - y) as usize, (yc - x) as usize, color);
}

fn draw_circle_points(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    fb.set_pixel(xc + x, yc + y, color);
    fb.set_pixel(xc - x, yc + y, color);
    fb.set_pixel(xc + x, yc - y, color);
    fb.set_pixel(xc - x, yc - y, color);
    fb.set_pixel(xc + y, yc + x, color);
    fb.set_pixel(xc - y, yc + x, color);
    fb.set_pixel(xc + y, yc - x, color);
    fb.set_pixel(xc - y, yc - x, color);
}

/// Draw a filled circle using Bresenham's algorithm.
///
/// # Arguments
///
/// * `fb` - Target framebuffer.
/// * `xc`, `yc` - Center coordinates of the circle.
/// * `radius` - Radius of the circle.
/// * `color` - 0xAARRGGBB color value.
pub fn fill_circle(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, color: u32) {
    if radius < 0 {
        return;
    }

    let mut x = 0;
    let mut y = radius;
    let mut d = 3 - 2 * radius;

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // Fast path: check if the entire bounding box of the circle is within the framebuffer
    let left = xc.saturating_sub(radius);
    let right = xc.saturating_add(radius);
    let top = yc.saturating_sub(radius);
    let bottom = yc.saturating_add(radius);

    let is_fully_on_screen = left >= 0 && right < width && top >= 0 && bottom < height;

    if is_fully_on_screen {
        // SAFETY: We just verified the entire circle's bounding box is well within the
        // valid range of [0, width) and [0, height). The horizontal lines will
        // always be within [left, right] and their y-coordinates within [top, bottom].
        unsafe {
            fill_circle_lines_unchecked(fb, xc, yc, x, y, color);

            while y >= x {
                x += 1;
                if d > 0 {
                    y -= 1;
                    d = d + 4 * (x - y) + 10;
                } else {
                    d = d + 4 * x + 6;
                }
                fill_circle_lines_unchecked(fb, xc, yc, x, y, color);
            }
        }
    } else {
        // Slow path: circle is partially or entirely off-screen
        fill_circle_lines(fb, xc, yc, x, y, color);

        while y >= x {
            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
            fill_circle_lines(fb, xc, yc, x, y, color);
        }
    }
}

/// Fills circle lines without bounds checking.
///
/// # Safety
/// Caller must guarantee that all points (xc ± x, yc ± y) and (xc ± y, yc ± x)
/// are within the framebuffer bounds.
unsafe fn fill_circle_lines_unchecked(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
    draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
    draw_horizontal_line_unchecked(fb, xc - y, xc + y, yc + x, color);
    draw_horizontal_line_unchecked(fb, xc - y, xc + y, yc - x, color);
}

/// Draws a horizontal line without bounds checking.
///
/// # Safety
/// Caller must guarantee that `x1` and `x2` are within `[0, fb.width())` and `y` is within `[0, fb.height())`.
unsafe fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x1: i32, x2: i32, y: i32, color: u32) {
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);

    let width = fb.width() as usize;
    let y_offset = (y as usize) * width;
    let start_idx = y_offset + (min_x as usize);
    let end_idx = y_offset + (max_x as usize);

    fb.as_mut_slice()[start_idx..=end_idx].fill(color);
}

fn fill_circle_lines(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    // For a filled circle, we draw horizontal lines connecting the left and right points
    // for each pair of symmetrical y-coordinates.
    draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
    draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
    draw_horizontal_line(fb, xc - y, xc + y, yc + x, color);
    draw_horizontal_line(fb, xc - y, xc + y, yc - x, color);
}

fn draw_horizontal_line(fb: &mut Framebuffer, x1: i32, x2: i32, y: i32, color: u32) {
    // Quick bounds check for y
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let min_x = x1.max(0);
    let max_x = x2.min(fb.width() as i32 - 1);

    if min_x <= max_x {
        // We could use `set_pixel`, but for a horizontal line, direct slice access
        // is much faster if we can safely calculate the offsets.
        // It avoids repeated bounds checking.
        let width = fb.width() as usize;
        let y_offset = (y as usize) * width;
        let start_idx = y_offset + (min_x as usize);
        let end_idx = y_offset + (max_x as usize);

        // This is safe because we clamped min_x, max_x, and y to valid ranges.
        // And we know end_idx >= start_idx.
        // We also know end_idx < fb.width() * fb.height().
        fb.as_mut_slice()[start_idx..=end_idx].fill(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_circle_pixels() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        // Clear to black
        fb.clear(0xFF00_0000);

        let color = 0xFFFF_FFFF;
        draw_circle(&mut fb, 10, 10, 5, color);

        // Verify center is NOT filled (it's an outline)
        assert_eq!(fb.get_pixel(10, 10), Some(0xFF00_0000));

        // Verify points on the circumference (top, bottom, left, right)
        assert_eq!(fb.get_pixel(10, 5), Some(color));
        assert_eq!(fb.get_pixel(10, 15), Some(color));
        assert_eq!(fb.get_pixel(5, 10), Some(color));
        assert_eq!(fb.get_pixel(15, 10), Some(color));
    }

    #[test]
    fn test_fill_circle_pixels() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        // Clear to black
        fb.clear(0xFF00_0000);

        let color = 0xFFFF_FFFF;
        fill_circle(&mut fb, 10, 10, 5, color);

        // Verify center IS filled
        assert_eq!(fb.get_pixel(10, 10), Some(color));

        // Verify points on the circumference (top, bottom, left, right)
        assert_eq!(fb.get_pixel(10, 5), Some(color));
        assert_eq!(fb.get_pixel(10, 15), Some(color));
        assert_eq!(fb.get_pixel(5, 10), Some(color));
        assert_eq!(fb.get_pixel(15, 10), Some(color));

        // Verify a point outside the circle is NOT filled
        assert_eq!(fb.get_pixel(2, 2), Some(0xFF00_0000));
    }

    #[test]
    fn test_circle_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Should not panic or crash
        draw_circle(&mut fb, 5, 5, 20, 0xFFFF_FFFF);
        fill_circle(&mut fb, 5, 5, 20, 0xFFFF_FFFF);

        // Negative coordinates
        draw_circle(&mut fb, -5, -5, 10, 0xFFFF_FFFF);
        fill_circle(&mut fb, -5, -5, 10, 0xFFFF_FFFF);
    }

    #[test]
    fn test_draw_circle_partial_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF00_0000);
        let color = 0xFFFF_FFFF;

        // Draw a circle centered near the top-left edge
        draw_circle(&mut fb, 2, 2, 5, color);

        // Some points should be rendered
        assert_eq!(fb.get_pixel(2, 7), Some(color)); // bottom
        assert_eq!(fb.get_pixel(7, 2), Some(color)); // right

        // Off-screen points should be ignored without panic
        // e.g., (2, -3) and (-3, 2)
    }

    #[test]
    fn test_draw_circle_fully_on_screen() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFF00_0000);
        let color = 0xFFFF_FFFF;

        // Fits perfectly in bounds, should hit fast path
        draw_circle(&mut fb, 10, 10, 5, color);

        assert_eq!(fb.get_pixel(10, 5), Some(color));
        assert_eq!(fb.get_pixel(10, 15), Some(color));
        assert_eq!(fb.get_pixel(5, 10), Some(color));
        assert_eq!(fb.get_pixel(15, 10), Some(color));
    }
}
