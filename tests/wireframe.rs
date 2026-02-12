use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_wireframe;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_fill_triangle_wireframe_draws_edges() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0x00000000); // Clear to transparent black
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Triangle in center of screen
    let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
    let color = 0xFFFF0000; // Red

    fill_triangle_wireframe(&mut fb, &mut zb, v0, v1, v2, color);

    // Verify center is NOT drawn (it's wireframe)
    // Center of triangle (0, 0) projected is (50, 50)
    let center_pixel = fb.get_pixel(50, 50);
    assert_eq!(center_pixel, Some(0x00000000), "Center pixel should be empty in wireframe");

    // Verify vertices are drawn
    // Project vertices roughly:
    // v0: (0, 0.5) -> (50, 25) (since y goes down?) Wait, y is up in world, but screen y is down?
    // Let's check project_to_screen_optimized: y = (1.0 - ndc_y) * half_height.
    // v0 ndc: y = 0.5/5.0 = 0.1. (1-0.1)*50 = 45. So y=45.
    // v1 ndc: y = -0.5/5.0 = -0.1. (1-(-0.1))*50 = 55. So y=55. x = -0.1 -> (0.1+1)*50 = 55? No.
    // x = (ndc_x + 1.0) * half_width.
    // v1 ndc x = -0.1. (-0.1+1)*50 = 45.
    // v2 ndc x = 0.1. (0.1+1)*50 = 55.

    // Let's check pixels near the vertices.
    // v0 projected ~ (50, 45)
    // v1 projected ~ (45, 55)
    // v2 projected ~ (55, 55)

    // Check v0
    assert_eq!(fb.get_pixel(50, 45), Some(color), "Vertex v0 should be drawn");
    // Check v1
    assert_eq!(fb.get_pixel(45, 55), Some(color), "Vertex v1 should be drawn");
    // Check v2
    assert_eq!(fb.get_pixel(55, 55), Some(color), "Vertex v2 should be drawn");

    // Check edge point (midpoint of v1-v2)
    // Midpoint is (50, 55)
    assert_eq!(fb.get_pixel(50, 55), Some(color), "Bottom edge midpoint should be drawn");
}
