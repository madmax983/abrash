use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::draw_circle_aa;

#[test]
fn test_draw_circle_aa_edges() {
    let mut fb = Framebuffer::new(30, 30).unwrap();
    fb.clear(0xFF00_0000); // Black background

    let color = 0xFFFFFFFF; // White
    draw_circle_aa(&mut fb, 15, 15, 10, color);

    // Assert that the pixel exactly at (15, 5) is fully white.
    let top_pixel = fb.get_pixel(15, 5).unwrap();
    assert_eq!(top_pixel, color);

    // In Wu's anti-aliased circle, fractional coordinates should have blended colors.
    // Check some diagonal pixels that aren't perfectly on an integer boundary.
    // The radius is 10. For x=7, y should be sqrt(100 - 49) = sqrt(51) ≈ 7.14
    // This means pixel (15+7, 15+7) = (22, 22) should be drawn with a high intensity,
    // and (15+7, 15+8) = (22, 23) should be drawn with a lower intensity.

    let p_main = fb.get_pixel(22, 22).unwrap();
    let p_outer = fb.get_pixel(22, 23).unwrap();

    // Ensure they are not black
    assert_ne!(p_main, 0xFF00_0000);
    assert_ne!(p_outer, 0xFF00_0000);

    // In Wu's algorithm, depending on how x and y are iterated, the intensity is split.
    // But both should have *some* intensity.
    // We expect the sum of their intensities to be roughly equal to a full pixel.
    let intensity_main = p_main & 0xFF;
    let intensity_outer = p_outer & 0xFF;

    assert!(intensity_main > 0, "Main pixel should have intensity");
    assert!(intensity_outer > 0, "Outer pixel should have intensity");
}
