use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_extreme_coordinates_crash() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height);
    let mut zb = ZBuffer::new(width, height);

    // v0 projects to Y = i32::MIN (approx)
    // ndc_y = y / w. If w=1, ndc_y = y.
    // screen_y = ((1.0 - ndc_y) * 0.5 * height)
    // If ndc_y is f32::MAX, screen_y is huge negative -> i32::MIN
    let v0 = (Vec3::new(0.0, f32::MAX, 0.0), 1.0);

    // v2 projects to Y = i32::MAX (approx)
    // If ndc_y is f32::MIN, screen_y is huge positive -> i32::MAX
    let v2 = (Vec3::new(0.0, f32::MIN, 0.0), 1.0);

    // v1 somewhere in between
    let v1 = (Vec3::new(0.0, 0.0, 0.0), 1.0);

    let color = 0xFFFFFFFF;

    // This should not panic
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
}
