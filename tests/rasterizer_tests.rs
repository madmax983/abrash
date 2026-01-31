use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use abrash::rasterizer::{
    draw_circle, draw_hline, draw_line, draw_polygon, draw_vline, fill_circle, fill_triangle,
    plot_pixel,
};
use abrash::shapes::{Polygon, Triangle};

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
        assert_eq!(
            fb.get_pixel(x, 50),
            Some(white),
            "Pixel at ({}, 50) should be white",
            x
        );
    }
}

#[test]
fn test_draw_line_vertical() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_line(&mut fb, 50, 10, 50, 90, white);

    // Check pixels along the line
    for y in 10..=90 {
        assert_eq!(
            fb.get_pixel(50, y),
            Some(white),
            "Pixel at (50, {}) should be white",
            y
        );
    }
}

#[test]
fn test_draw_line_diagonal() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_line(&mut fb, 10, 10, 50, 50, white);

    // Diagonal line should have pixels along y=x
    assert_eq!(fb.get_pixel(10, 10), Some(white));
    assert_eq!(fb.get_pixel(30, 30), Some(white));
    assert_eq!(fb.get_pixel(50, 50), Some(white));
}

#[test]
fn test_draw_line_steep() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    // Steep line (more vertical than horizontal)
    draw_line(&mut fb, 50, 10, 60, 90, white);

    assert_eq!(fb.get_pixel(50, 10), Some(white));
    assert_eq!(fb.get_pixel(60, 90), Some(white));
}

#[test]
fn test_draw_line_reverse_direction() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    // Draw from right to left
    draw_line(&mut fb, 90, 50, 10, 50, white);

    for x in 10..=90 {
        assert_eq!(fb.get_pixel(x, 50), Some(white));
    }
}

#[test]
fn test_draw_line_single_pixel() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_line(&mut fb, 50, 50, 50, 50, white);

    assert_eq!(fb.get_pixel(50, 50), Some(white));
}

#[test]
fn test_draw_polygon_triangle() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    let triangle = Polygon::new(vec![
        Vec2::new(50.0, 10.0),
        Vec2::new(90.0, 90.0),
        Vec2::new(10.0, 90.0),
    ]);

    draw_polygon(&mut fb, &triangle, white);

    // Check that vertices are drawn
    assert_eq!(fb.get_pixel(50, 10), Some(white));
    assert_eq!(fb.get_pixel(90, 90), Some(white));
    assert_eq!(fb.get_pixel(10, 90), Some(white));
}

#[test]
fn test_draw_hline() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_hline(&mut fb, 10, 90, 50, white);

    for x in 10..=90 {
        assert_eq!(fb.get_pixel(x, 50), Some(white));
    }
    assert_eq!(fb.get_pixel(5, 50), Some(0xFF000000));
}

#[test]
fn test_draw_vline() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_vline(&mut fb, 50, 10, 90, white);

    for y in 10..=90 {
        assert_eq!(fb.get_pixel(50, y), Some(white));
    }
    assert_eq!(fb.get_pixel(50, 5), Some(0xFF000000));
}

#[test]
fn test_fill_triangle() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    let tri = Triangle::new(
        Vec2::new(50.0, 10.0),
        Vec2::new(90.0, 90.0),
        Vec2::new(10.0, 90.0),
    );

    fill_triangle(&mut fb, &tri, white);

    // Center of triangle should be filled
    assert_eq!(fb.get_pixel(50, 50), Some(white));
    // Vertices should be filled
    assert_eq!(fb.get_pixel(50, 10), Some(white));
    // Outside should be black
    assert_eq!(fb.get_pixel(5, 5), Some(0xFF000000));
}

#[test]
fn test_draw_circle() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_circle(&mut fb, 50, 50, 20, white);

    // Points on the circle (radius 20 from center 50,50)
    assert_eq!(fb.get_pixel(70, 50), Some(white)); // Right
    assert_eq!(fb.get_pixel(30, 50), Some(white)); // Left
    assert_eq!(fb.get_pixel(50, 70), Some(white)); // Bottom
    assert_eq!(fb.get_pixel(50, 30), Some(white)); // Top

    // Center should be empty (not filled)
    assert_eq!(fb.get_pixel(50, 50), Some(0xFF000000));
}

#[test]
fn test_draw_circle_zero_radius() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    draw_circle(&mut fb, 50, 50, 0, white);

    // Just the center pixel
    assert_eq!(fb.get_pixel(50, 50), Some(white));
}

#[test]
fn test_fill_circle() {
    let mut fb = Framebuffer::new(100, 100);
    let white = 0xFFFFFFFF;

    fill_circle(&mut fb, 50, 50, 20, white);

    // Center should be filled
    assert_eq!(fb.get_pixel(50, 50), Some(white));
    // Edge should be filled
    assert_eq!(fb.get_pixel(70, 50), Some(white));
    // Outside should be black
    assert_eq!(fb.get_pixel(75, 50), Some(0xFF000000));
}
