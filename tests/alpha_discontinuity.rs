
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::rasterizer::fill_triangle_3d;
use abrash::math::Vec3;

#[test]
fn test_alpha_discontinuity() {
    let width = 10;
    let height = 10;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Clear to Red (0xFFFF0000)
    fb.clear(0xFFFF0000);

    // Triangle (Blue) covering the screen
    // Vertices in Clip Space. w=5.0.
    // v0: (-5, -5) -> NDC (-1, -1) -> Screen Top-Left
    // v1: (5, -5) -> NDC (1, -1) -> Screen Top-Right
    // v2: (0, 5) -> NDC (0, 1) -> Screen Bottom-Center
    // Note: Screen Y is flipped (1.0 - ndc_y). NDC 1 -> Screen 0. NDC -1 -> Screen Height.
    // Wait: (1.0 - (-1)) * 5 = 10. (1.0 - 1) * 5 = 0.
    // So NDC -1 is Bottom. NDC 1 is Top.
    // So (-5, -5) is Bottom-Left. (0, 5) is Top-Center.

    // Let's use large triangle to be sure.
    let v0 = (Vec3::new(-10.0, -10.0, 5.0), 5.0);
    let v1 = (Vec3::new(10.0, -10.0, 5.0), 5.0);
    let v2 = (Vec3::new(0.0, 10.0, 5.0), 5.0);

    // Pixel (5, 5) should be covered (Center).

    // Case 1: Alpha 255 (Opaque Blue)
    let color_opaque = 0xFF0000FF;
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color_opaque);

    let pixel_255 = fb.get_pixel(5, 5).unwrap();

    // Should be Blue
    assert_eq!(pixel_255, 0xFF0000FF, "Alpha 255 should be opaque Blue. Got {:08X}", pixel_255);

    // Case 2: Alpha 254 (Almost Opaque Blue)
    fb.clear(0xFFFF0000); // Red
    zb.clear();

    let color_almost_opaque = 0xFE0000FF; // Alpha 254
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color_almost_opaque);

    let pixel_254 = fb.get_pixel(5, 5).unwrap();
    let r = (pixel_254 >> 16) & 0xFF;
    let b = pixel_254 & 0xFF;

    println!("Alpha 254 result: R={} B={} (Hex {:08X})", r, b, pixel_254);

    // With bug: Alpha 254 -> dest weight 254 -> Mostly Red.
    // If bug exists, R will be high (close to 255).

    if r > 200 {
        panic!("BUG CONFIRMED: Alpha 254 resulted in R={}, expected < 50 (Mostly Blue)", r);
    }
}
