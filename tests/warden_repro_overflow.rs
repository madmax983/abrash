
use abrash::rasterizer::fill_triangle_3d;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;

#[test]
fn test_draw_scanline_flat_overflow() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a triangle where one vertex is extremely far to the left (i32::MIN)
    // The rasterizer clips to near plane, but not strictly to screen width in X during edge walking?
    // Wait, project_to_screen produces i32 coords.
    // If we manually feed vertices that project to i32::MIN, we might trigger the issue.

    // project_to_screen implementation:
    // x = (v.x * width / 2) + width / 2
    // If v.x is huge negative, x can be huge negative.

    // Let's try to construct such a case.
    let huge_coord = -2_000_000.0; // Large enough to result in very small x

    let v0 = (Vec3::new(huge_coord, 0.0, 5.0), 5.0);
    let v1 = (Vec3::new(0.0, -10.0, 5.0), 5.0);
    let v2 = (Vec3::new(0.0, 10.0, 5.0), 5.0);
    let color = 0xFFFF0000;

    // This might not be enough to trigger i32::MIN, but let's see if it crashes or produces weird results.
    // The specific line I'm worried about is:
    // z_fixed += (-i64::from(xs)) as i32 * dz_dx_fixed;
    // inside draw_scanline_flat.

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
}
