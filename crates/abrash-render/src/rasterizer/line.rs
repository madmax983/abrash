//! Line drawing algorithms.
//!
//! Implements Bresenham's line algorithm for wireframe rendering.

use crate::clipping::clip_line_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{Vec3, project_to_screen_optimized};
use crate::zbuffer::ZBuffer;

/// Draw a 3D line with Z-buffering.
///
/// Handles frustum clipping and perspective projection.
///
/// # Arguments
///
/// * `v0`, `v1` - Vertices defined as `(Position, W)`.
pub fn draw_line_3d(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    color: u32,
) {
    // Clip against frustum (returns None if fully culled)
    if let Some((v0_clipped, v1_clipped)) = clip_line_to_frustum(
        v0,
        v1,
        |v| *v,
        |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t),
    ) {
        let width = fb.width();
        let height = fb.height();
        let half_width = width as f32 * 0.5;
        let half_height = height as f32 * 0.5;

        // Project to screen
        let p0 = project_to_screen_optimized(v0_clipped.0, v0_clipped.1, half_width, half_height);
        let p1 = project_to_screen_optimized(v1_clipped.0, v1_clipped.1, half_width, half_height);

        // Bresenham's algorithm with Z interpolation
        // Standard integer-based line drawing
        let mut x0 = p0.x;
        let mut y0 = p0.y;
        let x1 = p1.x;
        let y1 = p1.y;

        // Z-interpolation (linear in screen space for simplicity/speed, though technically 1/z is linear)
        // For wireframes, linear Z is usually acceptable.
        let mut z = p0.z;
        let z_end = p1.z;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        // Calculate step size for Z interpolation
        // Total steps = max(|dx|, |dy|)
        let steps = dx.max(-dy);
        let dz = if steps > 0 {
            (z_end - z) / (steps as f32)
        } else {
            0.0
        };

        loop {
            // Check bounds (clipping should handle most cases, but guard against precision issues)
            if x0 >= 0 && x0 < width as i32 && y0 >= 0 && y0 < height as i32 {
                // Z-test
                // SAFETY: Bounds checked.
                unsafe {
                    let idx = (y0 as usize) * (width as usize) + (x0 as usize);
                    let z_buffer_val = zb.as_mut_slice().get_unchecked_mut(idx);
                    // Use standard depth test (less is closer for negative Z, wait.
                    // Project to screen produces z = v.z / w.
                    // If using standard OpenGL conventions, z is in [-1, 1].
                    // But rasterizer uses z < *depth_val.
                    // Let's assume standard behavior.
                    if z < *z_buffer_val {
                        *z_buffer_val = z;
                        fb.set_pixel_unchecked(x0 as usize, y0 as usize, color);
                    }
                }
            }

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
            z += dz;
        }
    }
}

/// Fill a 3D triangle in wireframe mode.
///
/// Draws the three edges of the triangle as lines.
pub fn fill_triangle_wireframe(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    color: u32,
) {
    draw_line_3d(fb, zb, v0, v1, color);
    draw_line_3d(fb, zb, v1, v2, color);
    draw_line_3d(fb, zb, v2, v0, color);
}

/// Draw a 2D line using Bresenham's algorithm without Z-buffering or 3D transformations.
/// This is heavily optimized for GUI, HUDs, and 2D overlays.
pub fn draw_line_2d(fb: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // Fast path: fully out of bounds check
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);

    if max_x < 0 || min_x >= width || max_y < 0 || min_y >= height {
        return; // Entirely off-screen
    }

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    // Optimization: Horizontal fast path
    if dy == 0 {
        if y0 >= 0 && y0 < height {
            let start = min_x.max(0);
            let end = max_x.min(width - 1);
            if start <= end {
                let w = width as usize;
                let offset = (y0 as usize) * w;
                fb.as_mut_slice()[offset + start as usize..=offset + end as usize].fill(color);
            }
        }
        return;
    }

    // Optimization: Vertical fast path
    if dx == 0 {
        if x0 >= 0 && x0 < width {
            let start = min_y.max(0);
            let end = max_y.min(height - 1);
            if start <= end {
                let w = width as usize;
                let x = x0 as usize;
                let slice = fb.as_mut_slice();
                for y in start..=end {
                    slice[(y as usize) * w + x] = color;
                }
            }
        }
        return;
    }

    let fully_visible = min_x >= 0 && max_x < width && min_y >= 0 && max_y < height;

    if fully_visible {
        // Unchecked fast path
        let slice = fb.as_mut_slice();
        let w = width as usize;
        loop {
            // SAFETY: Pre-checked bounding box guarantees bounds.
            unsafe {
                *slice.get_unchecked_mut((y0 as usize) * w + (x0 as usize)) = color;
            }
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
    } else {
        // Safe bounded path
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
}

/// Draw a thick 2D line.
///
/// Implemented by drawing multiple parallel Bresenham lines perpendicular to the main direction.
pub fn draw_thick_line_2d(
    fb: &mut Framebuffer,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    thickness: i32,
    color: u32,
) {
    if thickness <= 1 {
        draw_line_2d(fb, x0, y0, x1, y1, color);
        return;
    }

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    // Calculate perpendicular step direction for thickness
    let (px, py) = if dx > -dy { (0, 1) } else { (1, 0) };

    // Determine the offset range for the thickness to keep it centered
    let half_t = thickness / 2;
    let offset_start = -half_t;
    let offset_end = offset_start + thickness;

    loop {
        // Draw the thickness perpendicular to the line
        for offset in offset_start..offset_end {
            let px_offset = px * offset;
            let py_offset = py * offset;
            fb.set_pixel(x0 + px_offset, y0 + py_offset, color);
        }

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
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_draw_line_2d_horizontal() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        draw_line_2d(&mut fb, 2, 5, 8, 5, 0xFFFFFFFF);

        for x in 2..=8 {
            assert_eq!(fb.get_pixel(x, 5), Some(0xFFFFFFFF));
        }
        assert_eq!(fb.get_pixel(1, 5), Some(0));
        assert_eq!(fb.get_pixel(9, 5), Some(0));
    }

    #[test]
    fn test_draw_line_2d_vertical() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        draw_line_2d(&mut fb, 5, 2, 5, 8, 0xFFFFFFFF);

        for y in 2..=8 {
            assert_eq!(fb.get_pixel(5, y), Some(0xFFFFFFFF));
        }
        assert_eq!(fb.get_pixel(5, 1), Some(0));
        assert_eq!(fb.get_pixel(5, 9), Some(0));
    }

    #[test]
    fn test_draw_line_2d_diagonal() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        draw_line_2d(&mut fb, 2, 2, 5, 5, 0xFFFFFFFF);

        assert_eq!(fb.get_pixel(2, 2), Some(0xFFFFFFFF));
        assert_eq!(fb.get_pixel(3, 3), Some(0xFFFFFFFF));
        assert_eq!(fb.get_pixel(4, 4), Some(0xFFFFFFFF));
        assert_eq!(fb.get_pixel(5, 5), Some(0xFFFFFFFF));
    }

    #[test]
    fn test_draw_line_2d_out_of_bounds() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        // Should clip and draw visible portion without panicking
        draw_line_2d(&mut fb, -5, 5, 15, 5, 0xFFFFFFFF);

        assert_eq!(fb.get_pixel(0, 5), Some(0xFFFFFFFF));
        assert_eq!(fb.get_pixel(9, 5), Some(0xFFFFFFFF));
    }

    #[test]
    fn test_draw_thick_line_2d_horizontal() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        draw_thick_line_2d(&mut fb, 2, 5, 8, 5, 3, 0xFFFFFFFF);

        // Center line
        for x in 2..=8 {
            assert_eq!(fb.get_pixel(x, 4), Some(0xFFFFFFFF));
            assert_eq!(fb.get_pixel(x, 5), Some(0xFFFFFFFF));
            assert_eq!(fb.get_pixel(x, 6), Some(0xFFFFFFFF));
        }
        // Outside thickness
        assert_eq!(fb.get_pixel(5, 3), Some(0));
        assert_eq!(fb.get_pixel(5, 7), Some(0));
    }
}
