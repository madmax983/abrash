use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::texture::fill_quad_textured_gouraud;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_gouraud_quad_rendering() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tex = Texture::new(2, 2).unwrap();
    for y in 0..2 {
        for x in 0..2 {
            tex.set_pixel(x, y, 0xFFFF_FFFF);
        }
    }

    let v0 = (
        (Vec3::new(-1.0, -1.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec2::new(0.0, 0.0),
    );
    let v1 = (
        (Vec3::new(1.0, -1.0, 0.0), 1.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::new(1.0, 0.0),
    );
    let v2 = (
        (Vec3::new(1.0, 1.0, 0.0), 1.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec2::new(1.0, 1.0),
    );
    let v3 = (
        (Vec3::new(-1.0, 1.0, 0.0), 1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec2::new(0.0, 1.0),
    );

    fill_quad_textured_gouraud(&mut fb, &mut zb, v0, v1, v2, v3, &tex);

    // Sample (80, 80) -> NDC (0.6, -0.6)
    // R = 0.5 * (0.6 + 1) = 0.8
    // G = 0.5 * (-0.6 + 1) = 0.2

    let pixel = fb.get_pixel(80, 80).unwrap();
    let r = ((pixel >> 16) & 0xFF) as f32 / 255.0;
    let g = ((pixel >> 8) & 0xFF) as f32 / 255.0;
    let b = (pixel & 0xFF) as f32 / 255.0;

    let epsilon = 0.05;
    assert!((r - 0.8).abs() < epsilon, "Red: expected 0.8, got {r}");
    assert!((g - 0.2).abs() < epsilon, "Green: expected 0.2, got {g}");
    assert!((b - 0.0).abs() < epsilon, "Blue: expected 0.0, got {b}");
}
