use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec3, Vec4};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_gouraud_alpha_blending() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // 1. Draw opaque red background quad (made of 2 triangles) at z=0.9
    let v0 = (Vec3::new(-1.0, -1.0, 0.9), 1.0);
    let v1 = (Vec3::new(1.0, -1.0, 0.9), 1.0);
    let v2 = (Vec3::new(-1.0, 1.0, 0.9), 1.0);
    let v3 = (Vec3::new(1.0, 1.0, 0.9), 1.0);

    let red = Vec4::new(1.0, 0.0, 0.0, 1.0);

    fill_triangle_gouraud(
        &mut fb, &mut zb, (v0, red), (v1, red), (v2, red),
    );
    fill_triangle_gouraud(
        &mut fb, &mut zb, (v1, red), (v3, red), (v2, red),
    );

    // Verify background is red
    let center_pixel = fb.get_pixel(50, 50).unwrap();
    assert_eq!(center_pixel, 0xFFFF0000, "Background should be opaque red");

    // 2. Draw semi-transparent blue quad at z=0.5 (closer to camera)
    // Alpha = 0.5 (approx 128/255)
    let v0 = (Vec3::new(-1.0, -1.0, 0.5), 1.0);
    let v1 = (Vec3::new(1.0, -1.0, 0.5), 1.0);
    let v2 = (Vec3::new(-1.0, 1.0, 0.5), 1.0);
    let v3 = (Vec3::new(1.0, 1.0, 0.5), 1.0);

    let blue_transparent = Vec4::new(0.0, 0.0, 1.0, 0.5);

    fill_triangle_gouraud(
        &mut fb, &mut zb, (v0, blue_transparent), (v1, blue_transparent), (v2, blue_transparent),
    );
    fill_triangle_gouraud(
        &mut fb, &mut zb, (v1, blue_transparent), (v3, blue_transparent), (v2, blue_transparent),
    );

    // 3. Check center pixel
    // Expected blending:
    // Src (Blue): (0, 0, 255), Alpha 128 (0.5)
    // Dst (Red): (255, 0, 0), Alpha 255 (1.0)
    // Result = Src * Alpha + Dst * (1 - Alpha)
    // R = 0 * 0.5 + 255 * 0.5 = 127.5 -> 127 or 128
    // G = 0
    // B = 255 * 0.5 + 0 * 0.5 = 127.5 -> 127 or 128
    // A = 255 (usually we keep destination alpha or blend it, here we assume result is opaque for display)

    let pixel = fb.get_pixel(50, 50).unwrap();
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    println!("Pixel: {:08X} (R: {}, G: {}, B: {})", pixel, r, g, b);

    // Allow some tolerance for fixed point precision
    assert!(r >= 120 && r <= 135, "Red channel expected ~127, got {}", r);
    assert!(g == 0, "Green channel expected 0, got {}", g);
    assert!(b >= 120 && b <= 135, "Blue channel expected ~127, got {}", b);
}
