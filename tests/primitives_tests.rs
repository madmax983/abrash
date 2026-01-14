use abrash::framebuffer::Framebuffer;
use abrash::primitives::{draw_line, plot_pixel};

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

#[test]
fn test_draw_line_horizontal() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_line(&mut fb, 10, 50, 90, 50, white);

    // Check pixels along the line
    for x in 10..=90 {
        assert_eq!(fb.get_pixel(x, 50), Some(white), "Pixel at ({}, 50) should be white", x);
    }
}

#[test]
fn test_draw_line_vertical() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_line(&mut fb, 50, 10, 50, 90, white);

    // Check pixels along the line
    for y in 10..=90 {
        assert_eq!(fb.get_pixel(50, y), Some(white), "Pixel at (50, {}) should be white", y);
    }
}
