//! Bresenham's circle drawing algorithms.
//!
//! Implements integer-based circle rasterization.

use crate::framebuffer::Framebuffer;

/// Draw an empty circle (outline) using Bresenham's algorithm.
///
/// This uses Bresenham's midpoint circle algorithm, which determines the pixels needed
/// to rasterize a circle without requiring floating-point trigonometry. It calculates
/// the first octant and mirrors it to the other seven octants.
///
/// # Details
///
/// - The function safely handles off-screen coordinates by clipping points to the [`Framebuffer`] boundaries.
/// - The circle is drawn with a 1-pixel thickness.
/// - If the circle is entirely visible on screen, a fast-path is used that skips per-pixel bounds checks.
/// - If `radius` is less than or equal to 0, nothing is drawn.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::rasterizer::draw_circle;
///
/// // Create a 100x100 framebuffer
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// fb.clear(0xFF00_0000); // Black background
///
/// // Draw a red circle outline at center (50, 50) with radius 20
/// let color_red = 0xFFFF_0000;
/// draw_circle(&mut fb, 50, 50, 20, color_red);
///
/// // The center remains uncolored (black)
/// assert_eq!(fb.get_pixel(50, 50), Some(0xFF00_0000));
/// // The top edge of the circle is red
/// assert_eq!(fb.get_pixel(50, 30), Some(color_red));
/// ```
///
/// # Arguments
///
/// * `fb` - Target framebuffer.
/// * `xc`, `yc` - Center coordinates of the circle.
/// * `radius` - Radius of the circle.
/// * `color` - 0xAARRGGBB color value.
pub fn draw_circle(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, color: u32) {
    if radius <= 0 || radius > 16384 {
        return;
    }

    let mut x = 0;
    let mut y = radius;
    let mut d = 3 - 2 * radius;

    // Fast path: fully on screen
    let min_x = i64::from(xc) - i64::from(radius);
    let max_x = i64::from(xc) + i64::from(radius);
    let min_y = i64::from(yc) - i64::from(radius);
    let max_y = i64::from(yc) + i64::from(radius);

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return;
    }

    if min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height()) {
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
    } else {
        // Safe path: clip against screen bounds
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

#[inline(always)]
fn draw_circle_points_unchecked(
    fb: &mut Framebuffer,
    xc: i32,
    yc: i32,
    x: i32,
    y: i32,
    color: u32,
) {
    unsafe {
        fb.set_pixel_unchecked((xc + x) as usize, (yc + y) as usize, color);
        fb.set_pixel_unchecked((xc - x) as usize, (yc + y) as usize, color);
        fb.set_pixel_unchecked((xc + x) as usize, (yc - y) as usize, color);
        fb.set_pixel_unchecked((xc - x) as usize, (yc - y) as usize, color);
        fb.set_pixel_unchecked((xc + y) as usize, (yc + x) as usize, color);
        fb.set_pixel_unchecked((xc - y) as usize, (yc + x) as usize, color);
        fb.set_pixel_unchecked((xc + y) as usize, (yc - x) as usize, color);
        fb.set_pixel_unchecked((xc - y) as usize, (yc - x) as usize, color);
    }
}

#[inline(always)]
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

/// Draw a solid, filled circle using Bresenham's algorithm.
///
/// Unlike [`draw_circle`], which only plots individual pixels on the perimeter, this function
/// draws solid horizontal lines connecting the left and right edges of the circle for each vertical scanline.
/// This approach is much more efficient than drawing multiple smaller circles to fill the interior.
///
/// # Details
///
/// - The function safely handles off-screen coordinates by clipping scanlines to the [`Framebuffer`] boundaries.
/// - If the circle is entirely visible on screen, a fast-path is used that skips per-scanline bounds checks.
/// - If `radius` is less than or equal to 0, nothing is drawn.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::rasterizer::fill_circle;
///
/// // Create a 100x100 framebuffer
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// fb.clear(0xFF00_0000); // Black background
///
/// // Draw a solid blue circle at center (50, 50) with radius 10
/// let color_blue = 0xFF00_00FF;
/// fill_circle(&mut fb, 50, 50, 10, color_blue);
///
/// // The center is colored blue
/// assert_eq!(fb.get_pixel(50, 50), Some(color_blue));
/// // The edges are also blue
/// assert_eq!(fb.get_pixel(50, 40), Some(color_blue));
/// ```
///
/// # Arguments
///
/// * `fb` - Target framebuffer.
/// * `xc`, `yc` - Center coordinates of the circle.
/// * `radius` - Radius of the circle.
/// * `color` - 0xAARRGGBB color value.
pub fn fill_circle(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, color: u32) {
    if radius <= 0 || radius > 16384 {
        return;
    }

    let mut x = 0;
    let mut y = radius;
    let mut d = 3 - 2 * radius;

    // Fast path: fully on screen
    let min_x = i64::from(xc) - i64::from(radius);
    let max_x = i64::from(xc) + i64::from(radius);
    let min_y = i64::from(yc) - i64::from(radius);
    let max_y = i64::from(yc) + i64::from(radius);

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return;
    }

    if min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height()) {
        draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
        draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
        draw_horizontal_line_unchecked(fb, xc - y, xc + y, yc + x, color);
        // yc - x is identical to yc + x when x = 0

        while y >= x {
            x += 1;

            // The rows at yc+x and yc-x are always new scanlines when x increments
            draw_horizontal_line_unchecked(fb, xc - y, xc + y, yc + x, color);
            draw_horizontal_line_unchecked(fb, xc - y, xc + y, yc - x, color);

            if d > 0 {
                // If y is changing, the PREVIOUS y has reached its maximum x width.
                // We draw the scanlines for yc+y and yc-y with the maximum x reached (x-1).
                draw_horizontal_line_unchecked(fb, xc - (x - 1), xc + (x - 1), yc + y, color);
                draw_horizontal_line_unchecked(fb, xc - (x - 1), xc + (x - 1), yc - y, color);
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
        }
    } else {
        // Safe path: clip against screen bounds
        draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
        draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
        draw_horizontal_line(fb, xc - y, xc + y, yc + x, color);
        // yc - x is identical to yc + x when x = 0

        while y >= x {
            x += 1;

            draw_horizontal_line(fb, xc - y, xc + y, yc + x, color);
            draw_horizontal_line(fb, xc - y, xc + y, yc - x, color);

            if d > 0 {
                let max_x_for_y = x - 1;
                draw_horizontal_line(fb, xc - max_x_for_y, xc + max_x_for_y, yc + y, color);
                draw_horizontal_line(fb, xc - max_x_for_y, xc + max_x_for_y, yc - y, color);
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
        }
    }
}

#[inline(always)]
fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x1: i32, x2: i32, y: i32, color: u32) {
    let width = fb.width() as usize;
    let start_idx = (y as usize) * width + (x1 as usize);
    let end_idx = (y as usize) * width + (x2 as usize);
    fb.as_mut_slice()[start_idx..=end_idx].fill(color);
}

#[inline(always)]
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
    fn test_circle_overflow() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // i32::MAX overflow test. Should not crash or use unsafe unchecked set.
        draw_circle(&mut fb, i32::MAX - 5, 50, 10, 0xFFFFFFFF);
        fill_circle(&mut fb, i32::MAX - 5, 50, 10, 0xFFFFFFFF);
    }

    #[test]
    fn test_circle_radius_too_large() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Radius greater than 16384 should exit early without panicking or overflowing
        draw_circle(&mut fb, 50, 50, 16385, 0xFFFFFFFF);
        fill_circle(&mut fb, 50, 50, 16385, 0xFFFFFFFF);
        // Test with massive values
        draw_circle(&mut fb, 50, 50, i32::MAX / 2 + 2, 0xFFFFFFFF);
        fill_circle(&mut fb, 50, 50, i32::MAX / 2 + 2, 0xFFFFFFFF);
        draw_circle(&mut fb, 50, 50, i32::MAX, 0xFFFFFFFF);
        fill_circle(&mut fb, 50, 50, i32::MAX, 0xFFFFFFFF);
    }
}
