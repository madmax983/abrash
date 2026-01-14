use abrash::framebuffer::Framebuffer;
use abrash::primitives::plot_pixel;

#[test]
fn test_plot_pixel() {
    let mut fb = Framebuffer::new(10, 10);
    let white = 0xFFFF_FFFF;

    plot_pixel(&mut fb, 5, 5, white);

    assert_eq!(fb.get_pixel(5, 5), Some(white));
}

#[test]
fn test_plot_pixel_bounds() {
    let mut fb = Framebuffer::new(10, 10);
    let white = 0xFFFF_FFFF;

    // Should not panic on out of bounds
    plot_pixel(&mut fb, -1, -1, white);
    plot_pixel(&mut fb, 100, 100, white);
}
