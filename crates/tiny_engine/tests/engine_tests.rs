use tiny_engine::{draw_triangle, Framebuffer, Vec3};

#[test]
fn test_framebuffer_clear() {
    let mut fb = Framebuffer::new(10, 10);
    fb.clear(0xFF00FF00); // Green

    for x in 0..10 {
        for y in 0..10 {
            assert_eq!(fb.get_pixel(x, y), 0xFF00FF00);
        }
    }
}

#[test]
fn test_framebuffer_set_pixel() {
    let mut fb = Framebuffer::new(10, 10);
    fb.clear(0xFF000000); // Black

    fb.set_pixel(5, 5, 0xFFFF0000); // Red

    assert_eq!(fb.get_pixel(5, 5), 0xFFFF0000);
    assert_eq!(fb.get_pixel(4, 5), 0xFF000000);
}

#[test]
fn test_rasterize_triangle() {
    let mut fb = Framebuffer::new(20, 20);
    fb.clear(0xFF000000); // Black

    // Draw a red triangle in the middle of the 20x20 screen
    let v0 = Vec3::new(10.0, 5.0, 0.5);
    let v1 = Vec3::new(5.0, 15.0, 0.5);
    let v2 = Vec3::new(15.0, 15.0, 0.5);

    draw_triangle(&mut fb, v0, v1, v2, 0xFFFF0000);

    // Pixel deep inside the triangle should be red
    assert_eq!(fb.get_pixel(10, 10), 0xFFFF0000);

    // Pixel outside should be black
    assert_eq!(fb.get_pixel(2, 2), 0xFF000000);
}
