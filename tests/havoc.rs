use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_flat;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_random_triangles_safe(
        v0_x in -1000.0f32..1000.0, v0_y in -1000.0f32..1000.0, v0_z in 0.1f32..100.0,
        v1_x in -1000.0f32..1000.0, v1_y in -1000.0f32..1000.0, v1_z in 0.1f32..100.0,
        v2_x in -1000.0f32..1000.0, v2_y in -1000.0f32..1000.0, v2_z in 0.1f32..100.0,
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(100, 100).unwrap();

        // Use (Vec3, w) tuples where w=1.0
        let v0 = (Vec3::new(v0_x, v0_y, v0_z), 1.0);
        let v1 = (Vec3::new(v1_x, v1_y, v1_z), 1.0);
        let v2 = (Vec3::new(v2_x, v2_y, v2_z), 1.0);

        let normal = Vec3::new(0.0, 0.0, 1.0);
        let color = Vec3::new(1.0, 1.0, 1.0);

        fill_triangle_flat(&mut fb, &mut zb, v0, v1, v2, normal, color);
    }
}

#[test]
fn test_triangle_dos_huge_coordinates() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Huge coordinates in 3D
    let v0 = (Vec3::new(0.0, -1e30, 10.0), 1.0);
    let v1 = (Vec3::new(50.0, 0.0, 10.0), 1.0);
    let v2 = (Vec3::new(100.0, 1e30, 10.0), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(1.0, 1.0, 1.0);

    fill_triangle_flat(&mut fb, &mut zb, v0, v1, v2, normal, color);
}

#[test]
fn test_triangle_nan_coordinates() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    let v0 = (Vec3::new(0.0, f32::NAN, 10.0), 1.0);
    let v1 = (Vec3::new(50.0, 0.0, 10.0), 1.0);
    let v2 = (Vec3::new(100.0, 100.0, 10.0), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(1.0, 1.0, 1.0);

    fill_triangle_flat(&mut fb, &mut zb, v0, v1, v2, normal, color);
}

#[test]
fn test_framebuffer_overflow() {
    // This calls Framebuffer::new with u32::MAX, which will calculate size = u32::MAX * u32::MAX
    // This will overflow u64 checks and return Err
    assert!(Framebuffer::new(u32::MAX, u32::MAX).is_err());
}
