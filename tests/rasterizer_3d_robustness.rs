use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_fill_triangle_3d_neg_infinity_x() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // v0 has -Infinity X. Projecting this should result in i32::MIN screen X.
    // Y coords are within bounds to ensure scanline runs.
    let v0 = (Vec3::new(f32::NEG_INFINITY, 0.0, 5.0), 1.0);
    let v1 = (Vec3::new(50.0, 50.0, 5.0), 1.0);
    let v2 = (Vec3::new(50.0, 90.0, 5.0), 1.0);

    // This should NOT panic.
    // It might panic if the rasterizer computes `-xs` where xs is i32::MIN.
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFFFFFF);
}

#[test]
fn test_fill_triangle_3d_nan_coordinates() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    let v0 = (Vec3::new(f32::NAN, 0.0, 5.0), 1.0);
    let v1 = (Vec3::new(50.0, 50.0, 5.0), 1.0);
    let v2 = (Vec3::new(50.0, 90.0, 5.0), 1.0);

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFFFFFF);
}

#[test]
fn test_fill_triangle_3d_huge_coordinates() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Huge coordinates that might cause overflow but are finite
    let v0 = (Vec3::new(-1e30, 0.0, 5.0), 1.0);
    let v1 = (Vec3::new(1e30, 50.0, 5.0), 1.0);
    let v2 = (Vec3::new(0.0, 100.0, 5.0), 1.0);

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFFFFFF);
}
