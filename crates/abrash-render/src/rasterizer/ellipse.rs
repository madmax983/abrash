use crate::framebuffer::Framebuffer;

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
    let w = fb.width() as usize;
    let slice = fb.as_mut_slice();

    // Avoid multiple bounds checks by using manual 1D indexing.
    // Safe because this is only called from the fast-path which pre-validates boundaries.
    slice[((yc + y) as usize) * w + ((xc + x) as usize)] = color;
    if x != 0 {
        slice[((yc + y) as usize) * w + ((xc - x) as usize)] = color;
    }
    if y != 0 {
        slice[((yc - y) as usize) * w + ((xc + x) as usize)] = color;
    }
    if x != 0 && y != 0 {
        slice[((yc - y) as usize) * w + ((xc - x) as usize)] = color;
    }
}

pub fn draw_ellipse(fb: &mut Framebuffer, xc: i32, yc: i32, rx: i32, ry: i32, color: u32) {
    if rx <= 0 || ry <= 0 || rx > 16384 || ry > 16384 {
        return;
    }

    let rx_sq = i64::from(rx) * i64::from(rx);
    let ry_sq = i64::from(ry) * i64::from(ry);

    let mut x = 0;
    let mut y = ry;
    let mut px = 0_i64;
    let mut py = 2_i64 * rx_sq * i64::from(y);

    let min_x = i64::from(xc) - i64::from(rx);
    let max_x = i64::from(xc) + i64::from(rx);
    let min_y = i64::from(yc) - i64::from(ry);
    let max_y = i64::from(yc) + i64::from(ry);

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return;
    }

    if min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height()) {
        // Fast path: fully on screen
        draw_ellipse_points_unchecked(fb, xc, yc, x, y, color);

        // Region 1
        let mut p = ry_sq - (rx_sq * i64::from(ry)) + (rx_sq / 4);
        while px < py {
            x += 1;
            px += 2 * ry_sq;
            if p < 0 {
                p += ry_sq + px;
            } else {
                y -= 1;
                py -= 2 * rx_sq;
                p += ry_sq + px - py;
            }
            draw_ellipse_points_unchecked(fb, xc, yc, x, y, color);
        }

        // Region 2
        let mut p2 = ry_sq * i64::from(x) * i64::from(x)
            + ry_sq * i64::from(x)
            + rx_sq * i64::from(y - 1) * i64::from(y - 1)
            - rx_sq * ry_sq;

        while y > 0 {
            y -= 1;
            py -= 2 * rx_sq;
            if p2 > 0 {
                p2 += rx_sq - py;
            } else {
                x += 1;
                px += 2 * ry_sq;
                p2 += rx_sq - py + px;
            }
            draw_ellipse_points_unchecked(fb, xc, yc, x, y, color);
        }
    } else {
        // Safe path: bounds checking
        draw_ellipse_points(fb, xc, yc, x, y, color);

        // Region 1
        let mut p = ry_sq - (rx_sq * i64::from(ry)) + (rx_sq / 4);
        while px < py {
            x += 1;
            px += 2 * ry_sq;
            if p < 0 {
                p += ry_sq + px;
            } else {
                y -= 1;
                py -= 2 * rx_sq;
                p += ry_sq + px - py;
            }
            draw_ellipse_points(fb, xc, yc, x, y, color);
        }

        // Region 2
        let mut p2 = ry_sq * i64::from(x) * i64::from(x)
            + ry_sq * i64::from(x)
            + rx_sq * i64::from(y - 1) * i64::from(y - 1)
            - rx_sq * ry_sq;

        while y > 0 {
            y -= 1;
            py -= 2 * rx_sq;
            if p2 > 0 {
                p2 += rx_sq - py;
            } else {
                x += 1;
                px += 2 * ry_sq;
                p2 += rx_sq - py + px;
            }
            draw_ellipse_points(fb, xc, yc, x, y, color);
        }
    }
}

#[inline(always)]
fn draw_horizontal_line(fb: &mut Framebuffer, x1: i32, x2: i32, y: i32, color: u32) {
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

        fb.as_mut_slice()[start_idx..=end_idx].fill(color);
    }
}

#[inline(always)]
fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x1: i32, x2: i32, y: i32, color: u32) {
    let width = fb.width() as usize;
    let start_idx = (y as usize) * width + (x1 as usize);
    let end_idx = (y as usize) * width + (x2 as usize);
    fb.as_mut_slice()[start_idx..=end_idx].fill(color);
}

pub fn fill_ellipse(fb: &mut Framebuffer, xc: i32, yc: i32, rx: i32, ry: i32, color: u32) {
    if rx <= 0 || ry <= 0 || rx > 16384 || ry > 16384 {
        return;
    }

    let rx_sq = i64::from(rx) * i64::from(rx);
    let ry_sq = i64::from(ry) * i64::from(ry);

    let mut x = 0;
    let mut y = ry;
    let mut px = 0_i64;
    let mut py = 2_i64 * rx_sq * i64::from(y);

    let min_x = i64::from(xc) - i64::from(rx);
    let max_x = i64::from(xc) + i64::from(rx);
    let min_y = i64::from(yc) - i64::from(ry);
    let max_y = i64::from(yc) + i64::from(ry);

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return;
    }

    if min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height()) {
        // Fast path: fully on screen
        draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
        if y != 0 {
            draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
        }

        // Region 1
        let mut p = ry_sq - (rx_sq * i64::from(ry)) + (rx_sq / 4);
        while px < py {
            x += 1;
            px += 2 * ry_sq;
            if p < 0 {
                p += ry_sq + px;
            } else {
                y -= 1;
                py -= 2 * rx_sq;
                p += ry_sq + px - py;
            }
            draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
            }
        }

        // Region 2
        let mut p2 = ry_sq * i64::from(x) * i64::from(x)
            + ry_sq * i64::from(x)
            + rx_sq * i64::from(y - 1) * i64::from(y - 1)
            - rx_sq * ry_sq;

        while y > 0 {
            y -= 1;
            py -= 2 * rx_sq;
            if p2 > 0 {
                p2 += rx_sq - py;
            } else {
                x += 1;
                px += 2 * ry_sq;
                p2 += rx_sq - py + px;
            }
            draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line_unchecked(fb, xc - x, xc + x, yc - y, color);
            }
        }
    } else {
        // Safe path: bounds checking
        draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
        if y != 0 {
            draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
        }

        // Region 1
        let mut p = ry_sq - (rx_sq * i64::from(ry)) + (rx_sq / 4);
        while px < py {
            x += 1;
            px += 2 * ry_sq;
            if p < 0 {
                p += ry_sq + px;
            } else {
                y -= 1;
                py -= 2 * rx_sq;
                p += ry_sq + px - py;
            }
            draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
            }
        }

        // Region 2
        let mut p2 = ry_sq * i64::from(x) * i64::from(x)
            + ry_sq * i64::from(x)
            + rx_sq * i64::from(y - 1) * i64::from(y - 1)
            - rx_sq * ry_sq;

        while y > 0 {
            y -= 1;
            py -= 2 * rx_sq;
            if p2 > 0 {
                p2 += rx_sq - py;
            } else {
                x += 1;
                px += 2 * ry_sq;
                p2 += rx_sq - py + px;
            }
            draw_horizontal_line(fb, xc - x, xc + x, yc + y, color);
            if y != 0 {
                draw_horizontal_line(fb, xc - x, xc + x, yc - y, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_ellipse_pixels() {
        let mut fb = Framebuffer::new(30, 20).unwrap();
        fb.clear(0xFF00_0000);

        let color = 0xFFFF_FFFF;
        draw_ellipse(&mut fb, 15, 10, 10, 5, color);

        // Verify center is NOT filled
        assert_eq!(fb.get_pixel(15, 10), Some(0xFF00_0000));

        // Verify points on the circumference (top, bottom, left, right)
        assert_eq!(fb.get_pixel(15, 5), Some(color));
        assert_eq!(fb.get_pixel(15, 15), Some(color));
        assert_eq!(fb.get_pixel(5, 10), Some(color));
        assert_eq!(fb.get_pixel(25, 10), Some(color));
    }

    #[test]
    fn test_fill_ellipse_pixels() {
        let mut fb = Framebuffer::new(30, 20).unwrap();
        fb.clear(0xFF00_0000);

        let color = 0xFFFF_FFFF;
        fill_ellipse(&mut fb, 15, 10, 10, 5, color);

        // Verify center IS filled
        assert_eq!(fb.get_pixel(15, 10), Some(color));

        // Verify points on the circumference
        assert_eq!(fb.get_pixel(15, 5), Some(color));
        assert_eq!(fb.get_pixel(15, 15), Some(color));
        assert_eq!(fb.get_pixel(5, 10), Some(color));
        assert_eq!(fb.get_pixel(25, 10), Some(color));

        // Verify a point outside is NOT filled
        assert_eq!(fb.get_pixel(2, 2), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(15, 4), Some(0xFF00_0000)); // Just above the top
    }

    #[test]
    fn test_ellipse_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Should not panic or crash
        draw_ellipse(&mut fb, 5, 5, 20, 30, 0xFFFF_FFFF);
        fill_ellipse(&mut fb, 5, 5, 20, 30, 0xFFFF_FFFF);

        // Negative coordinates
        draw_ellipse(&mut fb, -5, -5, 10, 15, 0xFFFF_FFFF);
        fill_ellipse(&mut fb, -5, -5, 10, 15, 0xFFFF_FFFF);
    }
}
