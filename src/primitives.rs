//! Rasterization primitives.
//!
//! Software rendering functions that operate on framebuffers.
//! All primitives perform bounds checking.

use crate::framebuffer::Framebuffer;

pub fn plot_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: u32) {
    fb.set_pixel(x, y, color);
}

/// Draw a line using Bresenham's algorithm
pub fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        fb.set_pixel(x, y, color);

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}
