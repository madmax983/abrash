use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_lit;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_triangle_fully_behind_camera() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Vertices with negative W (behind camera)
    // Clip space coordinates.
    // If w < 0, it should be clipped.
    let v0 = (Vec3::new(0.0, 0.0, 0.0), -1.0);
    let v1 = (Vec3::new(1.0, 0.0, 0.0), -1.0);
    let v2 = (Vec3::new(0.0, 1.0, 0.0), -1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.2, 0.2, 0.2);
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);

    // This should NOT panic and should NOT draw anything (or at least not garbage)
    fill_triangle_lit(&mut fb, &mut zb, v0, v1, v2, normal, color, ambient, light_dir, light_color);

    // Verify framebuffer is empty
    for pixel in fb.as_slice() {
        assert_eq!(*pixel, 0xFF000000, "Framebuffer should be empty for triangle behind camera");
    }
}

#[test]
fn test_triangle_straddling_near_plane() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Triangle straddling the near plane (w=0).
    // v0 is behind (w = -1.0)
    // v1, v2 are in front (w = 1.0)
    // This triangle should be clipped into a quad (2 triangles) or 1 triangle depending on configuration.
    // v0 is top-leftish.

    // We position v1 and v2 so they are on screen.
    // Screen is 100x100.
    // v1: (0.5, 0.0, 0.0), w=1.0 -> NDC (0.5, 0.0) -> Screen (75, 50) roughly
    // v2: (-0.5, 0.0, 0.0), w=1.0 -> NDC (-0.5, 0.0) -> Screen (25, 50)
    // v0: (0.0, 1.0, 0.0), w=-1.0 -> Behind.
    // Clip against w > epsilon.
    // Intersection on edge v0-v1 and v0-v2.

    // Ensure CCW winding for backface culling
    let v0 = (Vec3::new(0.0, 1.0, 0.0), -1.0);
    let v1 = (Vec3::new(-0.5, -0.5, 0.0), 1.0);
    let v2 = (Vec3::new(0.5, -0.5, 0.0), 1.0);

    let normal = Vec3::new(0.0, 0.0, 1.0);
    let color = Vec3::new(1.0, 1.0, 1.0); // White
    let ambient = Vec3::new(1.0, 1.0, 1.0); // Full ambient to ignore lighting math
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(0.0, 0.0, 0.0);

    // This often causes panic or artifacts if not clipped
    fill_triangle_lit(&mut fb, &mut zb, v0, v1, v2, normal, color, ambient, light_dir, light_color);

    // Check center pixel (50, 50) - should be drawn?
    // The triangle base v1-v2 is at y=-0.5 (screen y ~75).
    // The peak is clipped.
    // It should draw *something*.

    let mut drawn = false;
    for pixel in fb.as_slice() {
        if *pixel != 0xFF000000 {
            drawn = true;
            break;
        }
    }
    assert!(drawn, "Should draw the visible part of the straddling triangle");
}
