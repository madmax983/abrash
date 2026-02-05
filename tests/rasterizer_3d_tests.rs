use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{fill_triangle_gouraud, fill_triangle_lit};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_fill_triangle_flat_basic() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Create a triangle facing the camera
    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0); // Facing camera
    let color = Vec3::new(1.0, 0.0, 0.0); // Red

    // Explicit lighting values (previously hidden in fill_triangle_flat)
    let ambient = Vec3::new(0.2, 0.2, 0.2);
    let sun_dir = Vec3::new(-0.5, -1.0, -0.5).normalize();
    let sun_color = Vec3::new(1.0, 1.0, 1.0);

    fill_triangle_lit(
        &mut fb, &mut zb, v0, v1, v2, normal, color, ambient, sun_dir, sun_color,
    );

    // Center should have the shaded color
    let pixel = fb.get_pixel(50, 50);
    assert_ne!(
        pixel,
        Some(0),
        "The center pixel should have been colored, not be the default black."
    );
}

#[test]
fn test_fill_triangle_lit_custom_lighting() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(0.0, 1.0, 0.0); // Green

    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
    let light_color = Vec3::new(1.0, 1.0, 1.0);

    fill_triangle_lit(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        normal,
        color,
        ambient,
        light_dir,
        light_color,
    );

    let pixel = fb.get_pixel(50, 50);
    assert_ne!(
        pixel,
        Some(0),
        "The center pixel should have been colored, not be the default black."
    );
}

#[test]
fn test_fill_triangle_gouraud_basic() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Triangle with different colors at each vertex
    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let c0 = Vec3::new(1.0, 0.0, 0.0); // Red
    let c1 = Vec3::new(0.0, 1.0, 0.0); // Green
    let c2 = Vec3::new(0.0, 0.0, 1.0); // Blue

    fill_triangle_gouraud(&mut fb, &mut zb, (v0, c0), (v1, c1), (v2, c2));

    // Should render without panicking
    let pixel = fb.get_pixel(50, 50);
    assert_ne!(
        pixel,
        Some(0),
        "The center pixel should have been colored, not be the default black."
    );
}

#[test]
fn test_fill_triangle_off_screen() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Triangle completely off-screen (Clip Space coordinates).
    // Pipeline converts these to Screen Coordinates.
    // x = 2.0, w = 1.0 -> NDC x = 2.0.
    // Screen X = (2.0 + 1.0) * 0.5 * 100 = 150.
    // Since 150 > 100 (width), it is off-screen.
    let v0 = (Vec3::new(2.0, 2.0, 0.5), 1.0);
    let v1 = (Vec3::new(3.0, 2.0, 0.5), 1.0);
    let v2 = (Vec3::new(2.5, 3.0, 0.5), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(1.0, 0.0, 0.0);

    let ambient = Vec3::new(0.2, 0.2, 0.2);
    let sun_dir = Vec3::new(-0.5, -1.0, -0.5).normalize();
    let sun_color = Vec3::new(1.0, 1.0, 1.0);

    // Should not panic
    fill_triangle_lit(
        &mut fb, &mut zb, v0, v1, v2, normal, color, ambient, sun_dir, sun_color,
    );

    // Should remain black
    for &pixel in fb.as_slice() {
        assert_eq!(pixel, 0xFF00_0000);
    }
}
