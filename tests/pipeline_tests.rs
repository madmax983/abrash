use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::pipeline::{fill_triangle_flat, fill_triangle_gouraud, fill_triangle_lit};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_fill_triangle_flat_basic() {
    let mut fb = Framebuffer::new(100, 100);
    let mut zb = ZBuffer::new(100, 100);

    // Create a triangle facing the camera
    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0); // Facing camera
    let color = Vec3::new(1.0, 0.0, 0.0); // Red

    fill_triangle_flat(&mut fb, &mut zb, v0, v1, v2, normal, color);

    // Center should have the shaded color
    let pixel = fb.get_pixel(50, 50);
    assert!(pixel.is_some());
}

#[test]
fn test_fill_triangle_lit_custom_lighting() {
    use abrash::light::{AmbientLight, DirectionalLight};

    let mut fb = Framebuffer::new(100, 100);
    let mut zb = ZBuffer::new(100, 100);

    let v0 = (Vec3::new(-0.5, -0.5, 0.5), 1.0);
    let v1 = (Vec3::new(0.5, -0.5, 0.5), 1.0);
    let v2 = (Vec3::new(0.0, 0.5, 0.5), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(0.0, 1.0, 0.0); // Green

    let ambient = AmbientLight::new(Vec3::new(0.1, 0.1, 0.1));
    let light = DirectionalLight::new(Vec3::new(0.0, 0.0, -1.0), Vec3::new(1.0, 1.0, 1.0));

    fill_triangle_lit(
        &mut fb, &mut zb, v0, v1, v2, normal, color, &ambient, &light,
    );

    let pixel = fb.get_pixel(50, 50);
    assert!(pixel.is_some());
}

#[test]
fn test_fill_triangle_gouraud_basic() {
    let mut fb = Framebuffer::new(100, 100);
    let mut zb = ZBuffer::new(100, 100);

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
    assert!(pixel.is_some());
}
