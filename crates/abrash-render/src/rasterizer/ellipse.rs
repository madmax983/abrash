use abrash_core::framebuffer::Framebuffer;

/// Draw an empty ellipse (outline) using the Midpoint Ellipse algorithm.
pub fn draw_ellipse(fb: &mut Framebuffer, xc: i32, yc: i32, rx: i32, ry: i32, color: u32) {
    if rx <= 0 || ry <= 0 {
        return;
    }

    let mut dx: i64;
    let mut dy: i64;
    let mut d1: i64;
    let mut d2: i64;
    let mut x: i32 = 0;
    let mut y: i32 = ry;

    let rx2: i64 = i64::from(rx) * i64::from(rx);
    let ry2: i64 = i64::from(ry) * i64::from(ry);

    let min_x = i64::from(xc) - i64::from(rx);
    let max_x = i64::from(xc) + i64::from(rx);
    let min_y = i64::from(yc) - i64::from(ry);
    let max_y = i64::from(yc) + i64::from(ry);

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return;
    }

    let is_fully_on_screen =
        min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height());

    d1 = ry2 - (rx2 * i64::from(ry)) + (rx2 / 4);
    dx = 2 * ry2 * i64::from(x);
    dy = 2 * rx2 * i64::from(y);

    if is_fully_on_screen {
        while dx < dy {
            draw_ellipse_points_unchecked(fb, xc, yc, x, y, color);

            if d1 < 0 {
                x += 1;
                dx += 2 * ry2;
                d1 += dx + ry2;
            } else {
                x += 1;
                y -= 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d1 += dx - dy + ry2;
            }
        }

        d2 = (ry2 * i64::from(x) * i64::from(x) + ry2 * i64::from(x))
            + (rx2 * i64::from(y - 1) * i64::from(y - 1))
            - rx2 * ry2;

        while y >= 0 {
            draw_ellipse_points_unchecked(fb, xc, yc, x, y, color);

            if d2 > 0 {
                y -= 1;
                dy -= 2 * rx2;
                d2 += rx2 - dy;
            } else {
                y -= 1;
                x += 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d2 += dx - dy + rx2;
            }
        }
    } else {
        while dx < dy {
            draw_ellipse_points(fb, xc, yc, x, y, color);

            if d1 < 0 {
                x += 1;
                dx += 2 * ry2;
                d1 += dx + ry2;
            } else {
                x += 1;
                y -= 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d1 += dx - dy + ry2;
            }
        }

        d2 = (ry2 * i64::from(x) * i64::from(x) + ry2 * i64::from(x))
            + (rx2 * i64::from(y - 1) * i64::from(y - 1))
            - rx2 * ry2;

        while y >= 0 {
            draw_ellipse_points(fb, xc, yc, x, y, color);

            if d2 > 0 {
                y -= 1;
                dy -= 2 * rx2;
                d2 += rx2 - dy;
            } else {
                y -= 1;
                x += 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d2 += dx - dy + rx2;
            }
        }
    }
}

#[inline(always)]
fn draw_ellipse_points(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    fb.set_pixel(xc + x, yc + y, color);
    if x != 0 {
        fb.set_pixel(xc - x, yc + y, color);
    }
    if y != 0 {
        fb.set_pixel(xc + x, yc - y, color);
    }
    if x != 0 && y != 0 {
        fb.set_pixel(xc - x, yc - y, color);
    }
}

#[inline(always)]
fn draw_ellipse_points_unchecked(
    fb: &mut Framebuffer,
    xc: i32,
    yc: i32,
    x: i32,
    y: i32,
    color: u32,
) {
    // ⚡ Bolt Performance Optimization:
    // Elide standard 2D array bounds checks by calculating flat 1D memory indices
    // and using unchecked assignments. Only called when the bounds are guaranteed on-screen.
    let w = fb.width() as i32;
    unsafe {
        *fb.as_mut_slice()
            .get_unchecked_mut(((yc + y) * w + (xc + x)) as usize) = color;
        if x != 0 {
            *fb.as_mut_slice()
                .get_unchecked_mut(((yc + y) * w + (xc - x)) as usize) = color;
        }
        if y != 0 {
            *fb.as_mut_slice()
                .get_unchecked_mut(((yc - y) * w + (xc + x)) as usize) = color;
        }
        if x != 0 && y != 0 {
            *fb.as_mut_slice()
                .get_unchecked_mut(((yc - y) * w + (xc - x)) as usize) = color;
        }
    }
}

/// Draw a solid, filled ellipse using the Midpoint Ellipse algorithm.
pub fn fill_ellipse(fb: &mut Framebuffer, xc: i32, yc: i32, rx: i32, ry: i32, color: u32) {
    if rx <= 0 || ry <= 0 {
        return;
    }

    let mut dx: i64;
    let mut dy: i64;
    let mut d1: i64;
    let mut d2: i64;
    let mut x: i32 = 0;
    let mut y: i32 = ry;

    let rx2: i64 = i64::from(rx) * i64::from(rx);
    let ry2: i64 = i64::from(ry) * i64::from(ry);

    let min_x = i64::from(xc) - i64::from(rx);
    let max_x = i64::from(xc) + i64::from(rx);
    let min_y = i64::from(yc) - i64::from(ry);
    let max_y = i64::from(yc) + i64::from(ry);

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return;
    }

    let is_fully_on_screen =
        min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height());

    d1 = ry2 - (rx2 * i64::from(ry)) + (rx2 / 4);
    dx = 2 * ry2 * i64::from(x);
    dy = 2 * rx2 * i64::from(y);

    if is_fully_on_screen {
        while dx < dy {
            draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
            }

            if d1 < 0 {
                x += 1;
                dx += 2 * ry2;
                d1 += dx + ry2;
            } else {
                x += 1;
                y -= 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d1 += dx - dy + ry2;
            }
        }

        d2 = (ry2 * i64::from(x) * i64::from(x) + ry2 * i64::from(x))
            + (rx2 * i64::from(y - 1) * i64::from(y - 1))
            - rx2 * ry2;

        while y >= 0 {
            draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
            }

            if d2 > 0 {
                y -= 1;
                dy -= 2 * rx2;
                d2 += rx2 - dy;
            } else {
                y -= 1;
                x += 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d2 += dx - dy + rx2;
            }
        }
    } else {
        while dx < dy {
            draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
            }

            if d1 < 0 {
                x += 1;
                dx += 2 * ry2;
                d1 += dx + ry2;
            } else {
                x += 1;
                y -= 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d1 += dx - dy + ry2;
            }
        }

        d2 = (ry2 * i64::from(x) * i64::from(x) + ry2 * i64::from(x))
            + (rx2 * i64::from(y - 1) * i64::from(y - 1))
            - rx2 * ry2;

        while y >= 0 {
            draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
            }

            if d2 > 0 {
                y -= 1;
                dy -= 2 * rx2;
                d2 += rx2 - dy;
            } else {
                y -= 1;
                x += 1;
                dx += 2 * ry2;
                dy -= 2 * rx2;
                d2 += dx - dy + rx2;
            }
        }
    }
}

#[inline(always)]
fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x1: i32, x2: i32, y: i32, color: u32) {
    let width = fb.width() as usize;
    let start_idx = (y as usize) * width + (x1 as usize);
    let end_idx = (y as usize) * width + (x2 as usize);
    // ⚡ Bolt Performance Optimization:
    // Elide bounds check with get_unchecked_mut in Fill
    unsafe {
        fb.as_mut_slice()
            .get_unchecked_mut(start_idx..=end_idx)
            .fill(color);
    }
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
        let width = fb.width() as usize;
        let y_offset = (y as usize) * width;
        let start_idx = y_offset + (min_x as usize);
        let end_idx = y_offset + (max_x as usize);

        // ⚡ Bolt Performance Optimization:
        // Elide bounds check with get_unchecked_mut in Fill
        unsafe {
            fb.as_mut_slice()
                .get_unchecked_mut(start_idx..=end_idx)
                .fill(color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_ellipse_pixels() {
        let mut fb = Framebuffer::new(30, 30).unwrap();
        fb.clear(0xFF00_0000);

        let color = 0xFFFF_FFFF;
        draw_ellipse(&mut fb, 15, 15, 10, 5, color);

        // Center should not be filled
        assert_eq!(fb.get_pixel(15, 15), Some(0xFF00_0000));

        // Extremities should be filled
        assert_eq!(fb.get_pixel(25, 15), Some(color)); // Right
        assert_eq!(fb.get_pixel(5, 15), Some(color)); // Left
        assert_eq!(fb.get_pixel(15, 20), Some(color)); // Bottom
        assert_eq!(fb.get_pixel(15, 10), Some(color)); // Top
    }

    #[test]
    fn test_fill_ellipse_pixels() {
        let mut fb = Framebuffer::new(30, 30).unwrap();
        fb.clear(0xFF00_0000);

        let color = 0xFFFF_FFFF;
        fill_ellipse(&mut fb, 15, 15, 10, 5, color);

        // Center should be filled
        assert_eq!(fb.get_pixel(15, 15), Some(color));

        // Extremities should be filled
        assert_eq!(fb.get_pixel(25, 15), Some(color)); // Right
        assert_eq!(fb.get_pixel(5, 15), Some(color)); // Left
        assert_eq!(fb.get_pixel(15, 20), Some(color)); // Bottom
        assert_eq!(fb.get_pixel(15, 10), Some(color)); // Top

        // Outside should be black
        assert_eq!(fb.get_pixel(26, 15), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(15, 21), Some(0xFF00_0000));
    }

    #[test]
    fn test_ellipse_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Should not panic
        draw_ellipse(&mut fb, 5, 5, 20, 20, 0xFFFF_FFFF);
        fill_ellipse(&mut fb, 5, 5, 20, 20, 0xFFFF_FFFF);
        draw_ellipse(&mut fb, -5, -5, 10, 10, 0xFFFF_FFFF);
        fill_ellipse(&mut fb, -5, -5, 10, 10, 0xFFFF_FFFF);
    }
}
