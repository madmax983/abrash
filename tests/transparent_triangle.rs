use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::rasterizer::fill_triangle_3d;
use abrash::math::Vec3;

#[test]
fn test_transparent_triangle() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // 1. Clear background to Blue
    fb.clear(0xFF0000FF);
    zb.clear();

    // 2. Draw a semi-transparent Red triangle
    // Color: 0x80FF0000 (Alpha ~128)
    let color = 0x80FF0000;

    // Triangle covering center
    // Ensure coordinates are within screen space after projection
    // We can use screen space coordinates directly if we setup projection,
    // but fill_triangle_3d takes World Space vertices and projects them.
    // Wait, fill_triangle_3d takes `(Vec3, f32)` which is `(Position, W)`.
    // It assumes Clip Space?
    // Let's check `fill_triangle_3d` impl.
    // It calls `project_to_screen`.

    // `project_to_screen` implementation in `src/math.rs` needs to be checked.
    // Assuming standard perspective divide: x/w, y/w mapped to screen.

    // If I pass w=1.0, and coordinates inside [-1, 1], it should map to screen.

    let v0 = (Vec3::new(0.0, 0.5, -1.0), 1.0);
    let v1 = (Vec3::new(-0.5, -0.5, -1.0), 1.0);
    let v2 = (Vec3::new(0.5, -0.5, -1.0), 1.0);

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);

    // 3. Sample a pixel in the center (e.g., 50, 50)
    // 0,0 is top-left usually? Or center?
    // project_to_screen: x = (v.x + 1.0) * 0.5 * width
    // So 0.0 -> center.

    let center_pixel = fb.get_pixel(width as i32 / 2, height as i32 / 2).unwrap();

    // Expected: Blend of Red (Src) and Blue (Dst)
    // Src: 0x80FF0000
    // Dst: 0xFF0000FF
    // R = 255 * 0.5 + 0 * 0.5 = 127
    // B = 0 * 0.5 + 255 * 0.5 = 127
    // A = 255 * 0.5 + 255 * 0.5 = 255

    let r = (center_pixel >> 16) & 0xFF;
    let b = center_pixel & 0xFF;

    println!("Pixel: {:08X}, R: {}, B: {}", center_pixel, r, b);

    // Allow some tolerance for integer math
    assert!(r >= 120 && r <= 135, "Red component should be blended (~127), got {}", r);
    assert!(b >= 120 && b <= 135, "Blue component should be blended (~127), got {}", b);

    // If it overwrote, R would be 255, B would be 0.
}
