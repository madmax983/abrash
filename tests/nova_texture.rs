use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_textured_cube_rendering() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a 2x2 texture
    // (0,0) Red, (1,0) Green
    // (0,1) Blue, (1,1) White
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFF0000); // Red
    tex.set_pixel(1, 0, 0xFF00FF00); // Green
    tex.set_pixel(0, 1, 0xFF0000FF); // Blue
    tex.set_pixel(1, 1, 0xFFFFFFFF); // White

    // Define a single textured triangle manually
    // 0: (-1, -1, 1), UV(0,0)
    // 1: ( 1, -1, 1), UV(1,0)
    // 2: ( 1,  1, 1), UV(1,1)

    let v0 = Vec3::new(-1.0, -1.0, 1.0);
    let v1 = Vec3::new(1.0, -1.0, 1.0);
    let v2 = Vec3::new(1.0, 1.0, 1.0);

    let uv0 = Vec2::new(0.0, 0.0);
    let uv1 = Vec2::new(1.0, 0.0);
    let uv2 = Vec2::new(1.0, 1.0);

    // Vertices in Clip Space (w=1.0)
    fill_triangle_textured(
        &mut fb,
        &mut zb,
        ((v0, 1.0), uv0),
        ((v1, 1.0), uv1),
        ((v2, 1.0), uv2),
        &tex,
    );

    // Verify a pixel near UV (0.25, 0.25).
    // In screen space:
    // x range -1..1 -> 0..10
    // y range -1..1 -> 10..0

    // We want a point inside the triangle.
    // Centroid of (-1,-1), (1,-1), (1,1) is (0.33, -0.33).
    // Screen X: (0.33 + 1) / 2 * 10 = 6.6 -> 6
    // Screen Y: (1 - (-0.33)) / 2 * 10 = 6.6 -> 6
    // UV at centroid should be average of (0,0), (1,0), (1,1) = (0.66, 0.33).
    // U=0.66 -> Index 1 (Green column)
    // V=0.33 -> Index 0 (Red/Green row)
    // So Pixel should be Green.

    // Let's check pixel (6, 6).
    let p = fb.get_pixel(6, 6).unwrap();

    // Check if it's Green
    assert_eq!(p, 0xFF00FF00, "Pixel at (6,6) should be Green");

    // Let's check a point closer to v0 (Red).
    // v0 is (-1, -1) -> Screen (0, 10) -> UV (0,0).
    // Point slightly inset: (-0.8, -0.8).
    // Screen X: (-0.8 + 1)/2 * 10 = 1.
    // Screen Y: (1 - (-0.8))/2 * 10 = 9.
    // UV approx (0.1, 0.1). Texel (0, 0) -> Red.

    let p_red = fb.get_pixel(1, 9).unwrap();
    assert_eq!(p_red, 0xFFFF0000, "Pixel at (1,9) should be Red");

    // Let's check a point closer to v2 (White).
    // v2 is (1, 1) -> Screen (10, 0) -> UV (1,1).
    // Point slightly inset: (0.8, 0.8).
    // Screen X: 9.
    // Screen Y: 1.
    // UV approx (0.9, 0.9). Texel (1, 1) -> White.

    let p_white = fb.get_pixel(9, 1).unwrap();
    assert_eq!(p_white, 0xFFFFFFFF, "Pixel at (9,1) should be White");
}
