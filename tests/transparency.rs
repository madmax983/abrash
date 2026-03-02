use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_transparency_blending() {
    let width = 100;
    let height = 100;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill background with solid blue
    fb.clear(0xFF00_00FF);
    zb.clear();

    // Triangle vertices (covering the center)
    let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);

    // 50% Red with 0x80 alpha
    // Expected result: 0.5 * Red + 0.5 * Blue
    // Red: 0xFF, Blue: 0xFF.
    // R = 0x80 (128)
    // B = 0x80 (128) if blending is src * alpha + dst * (1-alpha)
    // Actually blend_swar does: (c0 * inv_w + c1 * w) >> 8
    // If c0 is dst (Blue), c1 is src (Red), w is alpha (128).
    // inv_w = 128.
    // Result R = (0 * 128 + 255 * 128) >> 8 = 127
    // Result B = (255 * 128 + 0 * 128) >> 8 = 127
    // So roughly 0xFF7F007F (purple)

    let transparent_red = 0x80FF_0000;

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, transparent_red);

    // Check center pixel
    let center_pixel = fb.get_pixel(50, 50).unwrap();

    println!("Center pixel: 0x{center_pixel:08X}");

    // With current implementation (overwrite), it will be 0x80FF0000 (Red=255, Blue=0)
    // With proper implementation, it should be blended.

    let r = (center_pixel >> 16) & 0xFF;
    let g = (center_pixel >> 8) & 0xFF;
    let b = center_pixel & 0xFF;

    assert!(
        r > 100 && r < 150,
        "Red channel should be blended (expected ~127, got {r})"
    );
    assert_eq!(g, 0, "Green channel should be 0");
    assert!(
        b > 100 && b < 150,
        "Blue channel should be blended (expected ~127, got {b})"
    );
}
