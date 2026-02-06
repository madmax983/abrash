use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec3, Vec4};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_alpha_blending() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Clear background to Red
    fb.clear(0xFFFF0000);

    // Triangle vertex (screen center roughly)
    // Z is irrelevant for this test as long as it passes depth check (default clear is infinite depth, we use 0.5)
    let v0 = (Vec3::new(0.0, 0.5, 0.5), 1.0);
    let v1 = (Vec3::new(-0.8, -0.8, 0.5), 1.0);
    let v2 = (Vec3::new(0.8, -0.8, 0.5), 1.0);

    // Blue color with 50% alpha (0.5)
    // Result should be Blend(Red, Blue, 0.5) = 0.5*Red + 0.5*Blue = Purple
    let c_translucent = Vec4::new(0.0, 0.0, 1.0, 0.5);

    fill_triangle_gouraud(
        &mut fb,
        &mut zb,
        ((v0.0, v0.1), c_translucent),
        ((v1.0, v1.1), c_translucent),
        ((v2.0, v2.1), c_translucent),
    );

    // Check center pixel
    // 100x100 screen. Center is (50, 50).
    // Original: FF0000 (Red)
    // Overlay: 0000FF (Blue) at 0.5 alpha
    // Expected: R=127, G=0, B=127. Alpha should be 255 (fully opaque buffer)
    // Note: Exact value might vary depending on blending implementation (floor vs round), so allow small margin.

    let pixel = fb.get_pixel(50, 50).unwrap();
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    assert!(r >= 120 && r <= 135, "Red channel expected ~127, got {}", r);
    assert_eq!(g, 0, "Green channel expected 0, got {}", g);
    assert!(b >= 120 && b <= 135, "Blue channel expected ~127, got {}", b);
}
