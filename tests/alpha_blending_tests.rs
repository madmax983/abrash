use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec3, Vec4};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_alpha_blending() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Fill background with Red (manual clear or draw a big rect)
    fb.clear(0xFFFF0000); // ARGB: Opaque Red

    // Draw a semi-transparent Blue triangle covering the center
    // Coordinates are in Clip Space.
    let v0 = (Vec3::new(0.0, 0.5, 0.5), 1.0);
    let v1 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.5, -0.5, 0.5), 1.0);

    // Blue with 50% alpha (128/255 approx)
    let c_blue_alpha = Vec4::new(0.0, 0.0, 1.0, 0.5);

    fill_triangle_gouraud(
        &mut fb,
        &mut zb,
        ((v0.0, v0.1), c_blue_alpha),
        ((v1.0, v1.1), c_blue_alpha),
        ((v2.0, v2.1), c_blue_alpha),
    );

    let center_pixel = fb.get_pixel(50, 50).unwrap();

    // Expected Blending:
    // Src (Blue): (0, 0, 255), Alpha = 128 (approx 0.5)
    // Dst (Red):  (255, 0, 0)
    // Result = Src * Alpha + Dst * (1 - Alpha)
    // R = 0 * 0.5 + 255 * 0.5 = 127
    // G = 0
    // B = 255 * 0.5 + 0 * 0.5 = 127
    // Expected: 0xFF7F007F (approx)

    let r = (center_pixel >> 16) & 0xFF;
    let b = center_pixel & 0xFF;
    let g = (center_pixel >> 8) & 0xFF;

    println!("Pixel: {:08X} (R={}, G={}, B={})", center_pixel, r, g, b);

    // Tolerances
    assert!(r > 100 && r < 155, "Red component should be blended (approx 127), got {}", r);
    assert!(b > 100 && b < 155, "Blue component should be blended (approx 127), got {}", b);
    assert!(g < 10, "Green component should be near 0, got {}", g);
}
