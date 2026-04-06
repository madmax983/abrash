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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::{Mat4, Vec3};
    use crate::zbuffer::ZBuffer;

    fn setup() -> (Framebuffer, ZBuffer) {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        fb.clear(0xFF00_0000); // Black background
        zb.clear();
        (fb, zb)
    }

    // Helper to generate Clip Space coordinates given a 3D Model position, simple identity-like projection
    fn make_vertex(x: f32, y: f32, z: f32) -> (Vec3, f32) {
        // Assume orthogonal projection where xy matches screen closely
        // to make coordinates predictable.
        // Use w=1.0 for simplicity, which requires x, y, z in [-1.0, 1.0] to pass clipping.
        // Screen project_to_screen_optimized calculates:
        // x_screen = (v.x / w + 1.0) * half_width
        // y_screen = (1.0 - v.y / w) * half_height
        // z_screen = v.z / w
        (Vec3::new(x, y, z), 1.0)
    }

    #[test]
    fn test_draw_horizontal_line() {
        let (mut fb, mut zb) = setup();

        // Draw from left to right across the middle
        // Model X from -0.5 to 0.5. With w=1, half_width=50:
        // x_start = (-0.5 + 1) * 50 = 25
        // x_end = (0.5 + 1) * 50 = 75
        // y = (1 - 0) * 50 = 50
        let v0 = make_vertex(-0.5, 0.0, 0.5);
        let v1 = make_vertex(0.5, 0.0, 0.5);
        let color = 0xFFFF_0000;

        draw_line_3d(&mut fb, &mut zb, v0, v1, color);

        // Check line pixels
        assert_eq!(fb.get_pixel(25, 50), Some(color));
        assert_eq!(fb.get_pixel(50, 50), Some(color));
        assert_eq!(fb.get_pixel(75, 50), Some(color));

        // Ensure nothing above/below
        assert_eq!(fb.get_pixel(50, 49), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(50, 51), Some(0xFF00_0000));
    }

    #[test]
    fn test_draw_vertical_line() {
        let (mut fb, mut zb) = setup();

        // Model Y from -0.5 to 0.5.
        // y_start = (1 - (-0.5)) * 50 = 75
        // y_end = (1 - 0.5) * 50 = 25
        let v0 = make_vertex(0.0, -0.5, 0.5);
        let v1 = make_vertex(0.0, 0.5, 0.5);
        let color = 0xFF00_FF00;

        draw_line_3d(&mut fb, &mut zb, v0, v1, color);

        assert_eq!(fb.get_pixel(50, 25), Some(color));
        assert_eq!(fb.get_pixel(50, 50), Some(color));
        assert_eq!(fb.get_pixel(50, 75), Some(color));

        // Ensure nothing left/right
        assert_eq!(fb.get_pixel(49, 50), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(51, 50), Some(0xFF00_0000));
    }

    #[test]
    fn test_z_buffer_test() {
        let (mut fb, mut zb) = setup();

        let color_behind = 0xFFFF_0000;
        let color_front = 0xFF00_FF00;

        // Draw a horizontal line further away (z=0.8)
        let v0_back = make_vertex(-0.5, 0.0, 0.8);
        let v1_back = make_vertex(0.5, 0.0, 0.8);
        draw_line_3d(&mut fb, &mut zb, v0_back, v1_back, color_behind);

        // Draw a vertical line intersecting it but closer (z=0.2)
        let v0_front = make_vertex(0.0, -0.5, 0.2);
        let v1_front = make_vertex(0.0, 0.5, 0.2);
        draw_line_3d(&mut fb, &mut zb, v0_front, v1_front, color_front);

        // The intersection point (50, 50) should be the front color
        assert_eq!(fb.get_pixel(50, 50), Some(color_front));

        // Points on the horizontal line away from intersection should remain the back color
        assert_eq!(fb.get_pixel(25, 50), Some(color_behind));

        // Draw another line intersecting but even further behind (z=0.9)
        let color_hidden = 0xFF00_00FF;
        let v0_hidden = make_vertex(-0.5, 0.0, 0.9);
        let v1_hidden = make_vertex(0.5, 0.0, 0.9);
        draw_line_3d(&mut fb, &mut zb, v0_hidden, v1_hidden, color_hidden);

        // It should NOT overwrite the front or the back line
        assert_eq!(fb.get_pixel(50, 50), Some(color_front));
        assert_eq!(fb.get_pixel(25, 50), Some(color_behind));
    }

    #[test]
    fn test_line_clipping_out_of_bounds() {
        let (mut fb, mut zb) = setup();

        // Line completely outside the frustum (e.g., behind camera z < 0 or off-screen)
        let v0 = make_vertex(2.0, 2.0, 5.0); // x=150, y=-50
        let v1 = make_vertex(3.0, 3.0, 5.0); // x=200, y=-100
        let color = 0xFFFF_FFFF;

        // This should not panic, the line will be clipped out
        draw_line_3d(&mut fb, &mut zb, v0, v1, color);

        // Verify no pixels were modified
        for pixel in fb.as_slice().iter() {
            assert_eq!(*pixel, 0xFF00_0000);
        }
    }

    #[test]
    fn test_fill_triangle_wireframe() {
        let (mut fb, mut zb) = setup();

        let v0 = make_vertex(0.0, 0.5, 0.5); // top (50, 25)
        let v1 = make_vertex(-0.5, -0.5, 0.5); // bottom left (25, 75)
        let v2 = make_vertex(0.5, -0.5, 0.5); // bottom right (75, 75)
        let color = 0xFF12_3456;

        fill_triangle_wireframe(&mut fb, &mut zb, v0, v1, v2, color);

        // Check that the three vertices are plotted
        assert_eq!(fb.get_pixel(50, 25), Some(color));
        assert_eq!(fb.get_pixel(25, 75), Some(color));
        assert_eq!(fb.get_pixel(75, 75), Some(color));

        // Midpoints along the edges should also be plotted
        // v0 to v1 midpoint roughly: (37, 50)
        // Note: Due to integer Bresenham logic, it might be off by 1, we just ensure > 0 pixels drawn
        let drawn_pixels = fb.as_slice().iter().filter(|&&p| p == color).count();
        assert!(
            drawn_pixels > 100,
            "Wireframe should draw many perimeter pixels"
        );

        // The center should be completely empty (it's wireframe, not filled)
        assert_eq!(fb.get_pixel(50, 50), Some(0xFF00_0000));
    }
}
