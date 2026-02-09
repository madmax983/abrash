use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use abrash::texture::Texture;

#[test]
fn test_rasterizer_nan_coords() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create vertices with NaN and Infinity
    let v0 = (Vec3::new(f32::NAN, f32::NAN, f32::NAN), 1.0);
    let v1 = (Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY), 1.0);
    let v2 = (Vec3::new(0.0, 0.0, 5.0), 1.0);
    let color = 0xFFFF0000;

    // This should not panic or loop forever
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
}

#[test]
fn test_rasterizer_huge_coordinates() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Coordinates that fit in f32 but overflow i32 when scaled to screen space
    // Screen space conversion: (ndc + 1.0) * 0.5 * width
    // If v.x is 1e30, screen x is 1e30.
    // i32 max is 2e9.
    // This tests clamping/saturation behavior in project_to_screen.
    let v0 = (Vec3::new(1e30, 1e30, 5.0), 1.0);
    let v1 = (Vec3::new(-1e30, -1e30, 5.0), 1.0);
    let v2 = (Vec3::new(0.0, 0.0, 5.0), 1.0);
    let color = 0xFF00FF00;

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
}

#[test]
fn test_texture_overflow_check() {
    // Attempt to create a texture that would overflow memory
    // 100,000 * 100,000 = 10^10 pixels * 4 bytes = 40GB
    // Should return Err, not Panic (OOM).
    let result = Texture::new(100_000, 100_000);
    assert!(result.is_err(), "Expected overflow error, got Ok");
}
