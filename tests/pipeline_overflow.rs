use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::pipeline::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_fill_triangle_overflow() {
    let mut fb = Framebuffer::new(100, 100);
    let mut zb = ZBuffer::new(100, 100);

    // Create vertices that project to extremely large screen coordinates.
    // Screen width is 100.
    // project_to_screen logic:
    // inv_w = 1.0 / w
    // ndc_x = v.x * inv_w
    // screen_x = ((ndc_x + 1.0) * 0.5 * width) as i32

    // We want screen_x to overflow or be very large.
    // Let's target screen_x around 3,000,000,000 (exceeds i32::MAX of 2,147,483,647).
    // 3e9 = ((ndc_x + 1) * 50)
    // ndc_x ≈ 6e7
    // v.x * inv_w ≈ 6e7

    // If w = 0.0002 (just above 0.0001 threshold), inv_w = 5000.
    // v.x * 5000 = 6e7 => v.x = 12,000.

    // Let's use larger values to be sure.
    // w = 0.0002 => inv_w = 5000.
    // v.x = 20,000.0 => ndc_x = 1e8.
    // screen_x = 1e8 * 50 = 5e9. This will overflow/saturate to i32::MAX.

    let v0_pos = Vec3::new(-20000.0, 0.0, 10.0);
    let v1_pos = Vec3::new(20000.0, 0.0, 10.0);
    let v2_pos = Vec3::new(0.0, 100.0, 10.0);
    let w = 0.0002;

    let v0 = ((v0_pos, w), Vec3::new(1.0, 0.0, 0.0));
    let v1 = ((v1_pos, w), Vec3::new(0.0, 1.0, 0.0));
    let v2 = ((v2_pos, w), Vec3::new(0.0, 0.0, 1.0));

    // This call should NOT panic with "attempt to subtract with overflow"
    fill_triangle_gouraud(&mut fb, &mut zb, v0, v1, v2);
}
