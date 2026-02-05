use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_framebuffer_overflow() {
    // This calls Framebuffer::new with u32::MAX, which will calculate size = u32::MAX * u32::MAX
    // This will overflow u64 checks and return Err
    assert!(Framebuffer::new(u32::MAX, u32::MAX).is_err());
}

#[test]
fn test_fill_triangle_3d_overflow_safe() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Coordinate that projects to i32::MIN
    // We use a very large negative X to ensure it hits the limit.
    // X = -1e8, W = 1.0 => NDC X approx -1e8.
    // Screen X approx -1e8 * 50 = -5e9, which saturates to i32::MIN.
    let v0 = (Vec3::new(-1e9, 0.0, 1.0), 1.0);
    let v1 = (Vec3::new(50.0, 50.0, 1.0), 1.0);
    let v2 = (Vec3::new(50.0, 0.0, 1.0), 1.0);

    // This panicked in debug mode due to overflow when negating i32::MIN and when calculating dx.
    // It should now run safely without panicking.
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFFFFFF);
}
