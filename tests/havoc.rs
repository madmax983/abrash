use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use abrash::rasterizer::fill_triangle;
use abrash::shapes::Triangle;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_random_triangles_safe(
        v0_x in -1000.0f32..1000.0, v0_y in -1000.0f32..1000.0,
        v1_x in -1000.0f32..1000.0, v1_y in -1000.0f32..1000.0,
        v2_x in -1000.0f32..1000.0, v2_y in -1000.0f32..1000.0,
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let v0 = Vec2::new(v0_x, v0_y);
        let v1 = Vec2::new(v1_x, v1_y);
        let v2 = Vec2::new(v2_x, v2_y);
        let triangle = Triangle::new(v0, v1, v2);
        fill_triangle(&mut fb, &triangle, 0xFFFFFFFF);
    }
}

#[test]
fn test_triangle_dos_huge_coordinates() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    // Huge Y range triggers 4 billion loop iterations because of saturating cast to i32::MIN..i32::MAX
    let v0 = Vec2::new(0.0, -1e30);
    let v1 = Vec2::new(50.0, 0.0);
    let v2 = Vec2::new(100.0, 1e30);
    let triangle = Triangle::new(v0, v1, v2);

    // This will likely timeout or take forever if not fixed
    // running with a timeout wrapper would be ideal, but here we just run it and expect it to be slow
    // For the sake of "Red Phase", if I run this and it takes > 5s, I count it as a fail.
    fill_triangle(&mut fb, &triangle, 0xFFFFFFFF);
}

#[test]
fn test_triangle_nan_coordinates() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let v0 = Vec2::new(0.0, f32::NAN);
    let v1 = Vec2::new(50.0, 0.0);
    let v2 = Vec2::new(100.0, 100.0);
    let triangle = Triangle::new(v0, v1, v2);

    // This should panic in debug mode due to "cannot cast float to int"
    fill_triangle(&mut fb, &triangle, 0xFFFFFFFF);
}

#[test]
fn test_framebuffer_overflow() {
    // This calls Framebuffer::new with u32::MAX, which will calculate size = u32::MAX * u32::MAX
    // This will overflow u64 checks and return Err
    assert!(Framebuffer::new(u32::MAX, u32::MAX).is_err());
}
