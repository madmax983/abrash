//! Rasterization primitives.
//!
//! Software rendering functions that operate on framebuffers.
//! All primitives perform bounds checking.

use crate::framebuffer::Framebuffer;
use crate::shapes::Polygon;

pub fn plot_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: u32) {
    fb.set_pixel(x, y, color);
}

/// Draw a horizontal line (optimized - uses slice fill)
pub fn draw_hline(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let x_start = x0.min(x1).max(0).min(fb.width() as i32 - 1);
    let x_end = x0.max(x1).max(0).min(fb.width() as i32 - 1);

    let y = y as usize;
    let width = fb.width() as usize;
    let start = y * width + x_start as usize;
    let end = y * width + x_end as usize + 1;

    let pixels = fb.as_mut_slice();
    pixels[start..end].fill(color);
}

/// Draw a vertical line (optimized - stride access)
pub fn draw_vline(fb: &mut Framebuffer, x: i32, y0: i32, y1: i32, color: u32) {
    if x < 0 || x >= fb.width() as i32 {
        return;
    }

    let y_start = y0.min(y1).max(0).min(fb.height() as i32 - 1);
    let y_end = y0.max(y1).max(0).min(fb.height() as i32 - 1);

    let x = x as usize;
    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    for y in y_start..=y_end {
        let idx = y as usize * width + x;
        pixels[idx] = color;
    }
}

/// Draw a line using Bresenham's algorithm (internal)
fn draw_line_bresenham(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
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

/// Draw a line using the best available method
pub fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    // Dispatch to optimized versions for axis-aligned lines
    if y0 == y1 {
        draw_hline(fb, x0, x1, y0, color);
        return;
    }
    if x0 == x1 {
        draw_vline(fb, x0, y0, y1, color);
        return;
    }

    // Fall back to Bresenham for diagonal lines
    draw_line_bresenham(fb, x0, y0, x1, y1, color);
}

/// Draw a wireframe polygon
pub fn draw_polygon(fb: &mut Framebuffer, polygon: &Polygon, color: u32) {
    let verts = polygon.vertices();
    if verts.is_empty() {
        return;
    }

    // Draw lines between consecutive vertices
    for i in 0..verts.len() {
        let v0 = verts[i];
        let v1 = verts[(i + 1) % verts.len()];

        draw_line(
            fb,
            v0.x as i32, v0.y as i32,
            v1.x as i32, v1.y as i32,
            color,
        );
    }
}
