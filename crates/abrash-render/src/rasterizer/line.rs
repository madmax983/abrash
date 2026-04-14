//! Line drawing algorithms.
//!
//! Implements Bresenham's line algorithm for wireframe rendering.

use crate::clipping::clip_line_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{Vec3, project_to_screen_optimized};
use crate::zbuffer::ZBuffer;
use crate::rasterizer::core::assert_same_dimensions;

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
    assert_same_dimensions(fb, zb);
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
    use crate::framebuffer::Framebuffer;
    use crate::zbuffer::ZBuffer;
    use crate::rasterizer::line::draw_line_3d;
    use abrash_core::math::Vec3;

    #[test]
    #[should_panic(expected = "Framebuffer and ZBuffer widths must match")]
    fn should_panic_when_buffer_dimensions_mismatch() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(50, 50).unwrap(); // Mismatched size!

        let v0 = (Vec3::new(10.0, 10.0, 1.0), 1.0);
        let v1 = (Vec3::new(90.0, 90.0, 1.0), 1.0);

        draw_line_3d(&mut fb, &mut zb, v0, v1, 0xFFFFFFFF);
    }
}
