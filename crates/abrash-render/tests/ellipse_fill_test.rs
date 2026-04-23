use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::ellipse::fill_ellipse;

#[test]
fn test_fill_ellipse_matches_safe_behavior() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    fb.clear(0);
    fill_ellipse(&mut fb, 50, 50, 20, 30, 0xFFFFFFFF);
    // Spot check a known point inside the ellipse that should be filled
    assert_eq!(fb.get_pixel(50, 50), Some(0xFFFFFFFF));
    assert_eq!(fb.get_pixel(50, 25), Some(0xFFFFFFFF));
    // Spot check a point outside
    assert_eq!(fb.get_pixel(10, 10), Some(0));
}
