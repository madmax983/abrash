//! Rasterization primitives.
//!
//! Software rendering functions that operate on framebuffers.
//! All primitives perform bounds checking.

use crate::framebuffer::Framebuffer;
use crate::shapes::{Polygon, Triangle};

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

/// Draw a circle outline using midpoint algorithm
pub fn draw_circle(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    if radius <= 0 {
        if radius == 0 {
            fb.set_pixel(cx, cy, color);
        }
        return;
    }

    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Draw 8 octants
        fb.set_pixel(cx + x, cy + y, color);
        fb.set_pixel(cx - x, cy + y, color);
        fb.set_pixel(cx + x, cy - y, color);
        fb.set_pixel(cx - x, cy - y, color);
        fb.set_pixel(cx + y, cy + x, color);
        fb.set_pixel(cx - y, cy + x, color);
        fb.set_pixel(cx + y, cy - x, color);
        fb.set_pixel(cx - y, cy - x, color);

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

/// Fill a circle using midpoint algorithm with horizontal lines
pub fn fill_circle(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    if radius <= 0 {
        if radius == 0 {
            fb.set_pixel(cx, cy, color);
        }
        return;
    }

    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Draw horizontal lines for each y level (fills the circle)
        draw_hline(fb, cx - x, cx + x, cy + y, color);
        draw_hline(fb, cx - x, cx + x, cy - y, color);
        draw_hline(fb, cx - y, cx + y, cy + x, color);
        draw_hline(fb, cx - y, cx + y, cy - x, color);

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

/// Fill a triangle using scanline rasterization
pub fn fill_triangle(fb: &mut Framebuffer, tri: &Triangle, color: u32) {
    // Sort vertices by y coordinate (v0.y <= v1.y <= v2.y)
    let mut v0 = tri.v0;
    let mut v1 = tri.v1;
    let mut v2 = tri.v2;

    if v0.y > v1.y { std::mem::swap(&mut v0, &mut v1); }
    if v0.y > v2.y { std::mem::swap(&mut v0, &mut v2); }
    if v1.y > v2.y { std::mem::swap(&mut v1, &mut v2); }

    let total_height = v2.y - v0.y;
    if total_height < 0.001 {
        return; // Degenerate triangle
    }

    // Rasterize the triangle in two halves
    for y in (v0.y as i32)..=(v2.y as i32) {
        let y_f = y as f32;

        let second_half = y_f > v1.y || (v1.y - v0.y).abs() < 0.001;
        let segment_height = if second_half {
            v2.y - v1.y
        } else {
            v1.y - v0.y
        };

        let alpha = (y_f - v0.y) / total_height;
        let beta = if second_half {
            if segment_height.abs() < 0.001 { 0.0 } else { (y_f - v1.y) / segment_height }
        } else {
            if segment_height.abs() < 0.001 { 0.0 } else { (y_f - v0.y) / segment_height }
        };

        // Interpolate x coordinates along edges
        let mut x_a = v0.x + (v2.x - v0.x) * alpha;
        let mut x_b = if second_half {
            v1.x + (v2.x - v1.x) * beta
        } else {
            v0.x + (v1.x - v0.x) * beta
        };

        if x_a > x_b {
            std::mem::swap(&mut x_a, &mut x_b);
        }

        draw_hline(fb, x_a as i32, x_b as i32, y, color);
    }
}
