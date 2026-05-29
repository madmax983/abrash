//! # 3D Line Rendering 📏
//!
//! Line drawing is the foundation of wireframe rendering and debugging.
//! Unlike simple 2D screen-space lines (like a UI border), 3D lines exist
//! in world space and must participate in the full graphics pipeline.
//!
//! This module implements Bresenham's line algorithm combined with Z-buffering
//! and 3D frustum clipping to allow lines to interact correctly with solid objects.
//!
//! Why this matters: When a 3D line passes behind a solid wall, it shouldn't be drawn.
//! When a line goes behind the camera, it must be clipped to the near plane before
//! perspective division, otherwise projection math creates wild distortions.

use crate::clipping::clip_line_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{Vec3, project_to_screen_optimized};
use crate::rasterizer::core::assert_same_dimensions;
use crate::zbuffer::ZBuffer;

/// Draws a 3D line between two clip-space vertices with depth testing.
///
/// This is essential for wireframe rendering (`fill_triangle_wireframe`) and
/// debugging collision shapes or light paths where depth interaction with the rest
/// of the 3D scene is required.
///
/// ## How it works
/// 1. **Frustum Clipping**: Uses Sutherland-Hodgman to clip the line segment against the 3D viewing volume.
/// 2. **Projection**: Transforms the clipped endpoints into 2D screen space coordinates using perspective division.
/// 3. **Rasterization**: Walks the edge using Bresenham's algorithm while linearly interpolating the Z-depth to test against the [`ZBuffer`].
///
/// ## Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::math::Vec3;
/// use abrash_render::rasterizer::draw_line_3d;
/// use abrash_render::zbuffer::ZBuffer;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Two vertices in Homogeneous Clip Space (XYZ, W)
/// // They have W=1.0, so they represent normalized device coordinates (NDC) directly.
/// let v0 = (Vec3::new(-0.5, 0.0, 0.5), 1.0);
/// let v1 = (Vec3::new(0.5, 0.0, 0.5), 1.0);
///
/// // Draw a white line between them
/// draw_line_3d(&mut fb, &mut zb, v0, v1, 0xFFFF_FFFF);
/// ```
///
/// ## Parameters
///
/// * `fb` - Target framebuffer for pixel output.
/// * `zb` - Target Z-buffer for depth testing.
/// * `v0`, `v1` - Vertices defined as a tuple of `(Position, W)`.
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

        let dx_full = (i64::from(x1) - i64::from(x0)).abs();
        let dy_full = (i64::from(y1) - i64::from(y0)).abs();
        if dx_full > 16384 || dy_full > 16384 {
            return; // Prevent extreme values from causing DoS / Infinite Loops
        }
        let dx = dx_full as i32;
        let dy = -(dy_full as i32);
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = i64::from(dx) + i64::from(dy);

        let idx_step_x = sx as isize;
        let idx_step_y = (sy as isize) * (width as isize);
        let mut idx = (y0 as isize) * (width as isize) + (x0 as isize);

        // Calculate step size for Z interpolation
        // Total steps = max(|dx|, |dy|)
        let steps = dx.max(-dy) as f32;
        let dz = if steps > 0.0 {
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
                    let z_buffer_val = zb.as_mut_slice().get_unchecked_mut(idx as usize);
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
            if e2 >= i64::from(dy) {
                err += i64::from(dy);
                x0 += sx;
                idx += idx_step_x;
            }
            if e2 <= i64::from(dx) {
                err += i64::from(dx);
                y0 += sy;
                idx += idx_step_y;
            }
            z += dz;
        }
    }
}

/// Fills a 3D triangle in wireframe mode.
///
/// Useful for debugging the geometry of a mesh or achieving a specific wireframe aesthetic.
/// It works by calling [`draw_line_3d`] on all three edges.
///
/// ## Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::math::Vec3;
/// use abrash_render::rasterizer::fill_triangle_wireframe;
/// use abrash_render::zbuffer::ZBuffer;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// let v0 = (Vec3::new(0.0, 0.5, 0.5), 1.0);
/// let v1 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
/// let v2 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
///
/// fill_triangle_wireframe(&mut fb, &mut zb, v0, v1, v2, 0xFFFF_FFFF);
/// ```
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
    use crate::rasterizer::line::draw_line_3d;
    use crate::zbuffer::ZBuffer;
    use abrash_core::math::Vec3;

    #[test]
    #[should_panic(expected = "Framebuffer and ZBuffer widths must match")]
    fn should_panic_when_buffer_dimensions_mismatch() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(50, 50).unwrap(); // Mismatched size!

        let v0 = (Vec3::new(10.0, 10.0, 1.0), 1.0);
        let v1 = (Vec3::new(90.0, 90.0, 1.0), 1.0);

        draw_line_3d(&mut fb, &mut zb, v0, v1, 0x00FF_FFFFFF);
    }
}
