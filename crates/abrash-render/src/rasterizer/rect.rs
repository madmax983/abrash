use abrash_core::framebuffer::Framebuffer;
use abrash_core::ivec::IVec2;

/// ⚡ Bolt: Fast horizontal line fill that avoids per-pixel bounds checks in the inner loop.
#[inline(always)]
fn draw_horizontal_line(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let x_start = x0.max(0);
    let x_end = x1.min(fb.width() as i32 - 1);

    if x_start > x_end {
        return;
    }

    let w = fb.width() as usize;
    let start_idx = y as usize * w + x_start as usize;
    let end_idx = y as usize * w + x_end as usize;

    // Explicitly avoids per-pixel bounds checks inside the slice
    fb.as_mut_slice()[start_idx..=end_idx].fill(color);
}

/// ⚡ Bolt: Direct horizontal line fill skipping boundary checking completely
/// Warning: Only call when it is guaranteed that the span is on screen.
#[inline(always)]
fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    let w = fb.width() as usize;
    let start_idx = y as usize * w + x0 as usize;
    let end_idx = y as usize * w + x1 as usize;

    // Safety check fallback to standard fill if things went horribly wrong,
    // though the contract says it should be safe.
    if end_idx < fb.as_mut_slice().len() && start_idx <= end_idx {
        fb.as_mut_slice()[start_idx..=end_idx].fill(color);
    }
}

pub fn draw_rect(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let right = x + width as i32 - 1;
    let bottom = y + height as i32 - 1;

    // Top
    draw_horizontal_line(fb, x, right, y, color);
    // Bottom
    draw_horizontal_line(fb, x, right, bottom, color);
    // Left
    draw_line_2d_local(fb, IVec2::new(x, y), IVec2::new(x, bottom), color);
    // Right
    draw_line_2d_local(fb, IVec2::new(right, y), IVec2::new(right, bottom), color);
}

pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    fb.clear_rect(x, y, width, height, color);
}

pub fn draw_rounded_rect(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: i32,
    color: u32,
) {
    if width == 0 || height == 0 {
        return;
    }

    let radius = radius
        .max(0)
        .min((width as i32) / 2)
        .min((height as i32) / 2);

    if radius == 0 {
        draw_rect(fb, x, y, width, height, color);
        return;
    }

    let inner_w = (width as i32) - 2 * radius;
    let inner_h = (height as i32) - 2 * radius;

    let cx_left = x + radius;
    let cx_right = x + width as i32 - 1 - radius;
    let cy_top = y + radius;
    let cy_bottom = y + height as i32 - 1 - radius;

    let is_on_screen = x >= 0
        && y >= 0
        && (x + width as i32) <= fb.width() as i32
        && (y + height as i32) <= fb.height() as i32;

    // Draw straight edges
    if inner_w > 0 {
        draw_horizontal_line(fb, cx_left, cx_right, y, color); // Top
        draw_horizontal_line(fb, cx_left, cx_right, y + height as i32 - 1, color); // Bottom
    }
    if inner_h > 0 {
        draw_line_2d_local(fb, IVec2::new(x, cy_top), IVec2::new(x, cy_bottom), color); // Left
        draw_line_2d_local(
            fb,
            IVec2::new(x + width as i32 - 1, cy_top),
            IVec2::new(x + width as i32 - 1, cy_bottom),
            color,
        ); // Right
    }

    let mut cx = 0;
    let mut cy = radius;
    let mut d = 3 - 2 * radius;

    if is_on_screen {
        let w = fb.width() as i32;
        let buf = fb.as_mut_slice();

        while cx <= cy {
            // Optimized on-screen rendering without bounds checking
            // Top Left
            buf[((cy_top - cy) * w + (cx_left - cx)) as usize] = color;
            buf[((cy_top - cx) * w + (cx_left - cy)) as usize] = color;

            // Top Right
            buf[((cy_top - cy) * w + (cx_right + cx)) as usize] = color;
            buf[((cy_top - cx) * w + (cx_right + cy)) as usize] = color;

            // Bottom Left
            buf[((cy_bottom + cy) * w + (cx_left - cx)) as usize] = color;
            buf[((cy_bottom + cx) * w + (cx_left - cy)) as usize] = color;

            // Bottom Right
            buf[((cy_bottom + cy) * w + (cx_right + cx)) as usize] = color;
            buf[((cy_bottom + cx) * w + (cx_right + cy)) as usize] = color;

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    } else {
        while cx <= cy {
            fb.set_pixel(cx_left - cx, cy_top - cy, color);
            fb.set_pixel(cx_left - cy, cy_top - cx, color);
            fb.set_pixel(cx_right + cx, cy_top - cy, color);
            fb.set_pixel(cx_right + cy, cy_top - cx, color);
            fb.set_pixel(cx_left - cx, cy_bottom + cy, color);
            fb.set_pixel(cx_left - cy, cy_bottom + cx, color);
            fb.set_pixel(cx_right + cx, cy_bottom + cy, color);
            fb.set_pixel(cx_right + cy, cy_bottom + cx, color);

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    }
}

pub fn fill_rounded_rect(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: i32,
    color: u32,
) {
    if width == 0 || height == 0 {
        return;
    }

    let radius = radius
        .max(0)
        .min((width as i32) / 2)
        .min((height as i32) / 2);

    if radius == 0 {
        fill_rect(fb, x, y, width, height, color);
        return;
    }

    // Fill the central inner rectangle block
    fill_rect(
        fb,
        x,
        y + radius,
        width,
        height - 2 * (radius as u32),
        color,
    );

    let cx_left = x + radius;
    let cx_right = x + width as i32 - 1 - radius;
    let cy_top = y + radius;
    let cy_bottom = y + height as i32 - 1 - radius;

    let is_on_screen = x >= 0
        && y >= 0
        && (x + width as i32) <= fb.width() as i32
        && (y + height as i32) <= fb.height() as i32;

    let mut cx = 0;
    let mut cy = radius;
    let mut d = 3 - 2 * radius;

    if is_on_screen {
        while cx <= cy {
            // Draw lines for corners, bypassing boundaries checks since we know it's on screen
            draw_horizontal_line_unchecked(fb, cx_left - cx, cx_right + cx, cy_top - cy, color);
            draw_horizontal_line_unchecked(fb, cx_left - cx, cx_right + cx, cy_bottom + cy, color);

            // To avoid overdraw on the middle portions if cx != cy
            if cx != cy {
                draw_horizontal_line_unchecked(fb, cx_left - cy, cx_right + cy, cy_top - cx, color);
                draw_horizontal_line_unchecked(
                    fb,
                    cx_left - cy,
                    cx_right + cy,
                    cy_bottom + cx,
                    color,
                );
            }

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    } else {
        while cx <= cy {
            draw_horizontal_line(fb, cx_left - cx, cx_right + cx, cy_top - cy, color);
            draw_horizontal_line(fb, cx_left - cx, cx_right + cx, cy_bottom + cy, color);

            if cx != cy {
                draw_horizontal_line(fb, cx_left - cy, cx_right + cy, cy_top - cx, color);
                draw_horizontal_line(fb, cx_left - cy, cx_right + cy, cy_bottom + cx, color);
            }

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    }
}

fn draw_line_2d_local(fb: &mut Framebuffer, p0: IVec2, p1: IVec2, color: u32) {
    let mut x0 = p0.x;
    let mut y0 = p0.y;
    let x1 = p1.x;
    let y1 = p1.y;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        fb.set_pixel(x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_rect(&mut fb, 10, 10, 20, 20, 0xFFFFFFFF);
    }

    #[test]
    fn test_fill_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_rect(&mut fb, 10, 10, 20, 20, 0xFFFFFFFF);
    }

    #[test]
    fn test_draw_rounded_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_rounded_rect(&mut fb, 10, 10, 20, 20, 5, 0xFFFFFFFF);
    }

    #[test]
    fn test_fill_rounded_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_rounded_rect(&mut fb, 10, 10, 20, 20, 5, 0xFFFFFFFF);
    }

    #[test]
    fn test_rounded_rect_oob() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_rounded_rect(&mut fb, -10, -10, 50, 50, 10, 0xFFFFFFFF);
        draw_rounded_rect(&mut fb, 80, 80, 50, 50, 10, 0xFFFFFFFF);
    }
}
