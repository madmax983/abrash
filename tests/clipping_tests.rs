use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::draw_line;
use proptest::prelude::*;

#[test]
fn test_draw_line_clip_boundary_exact() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let white = 0xFFFFFFFF;

    // Line exactly on the right boundary (should be clipped out or handled safe)
    // x range is 0..100. So 100 is out of bounds.
    draw_line(&mut fb, 100, 0, 100, 99, white);

    // Ensure no pixels were drawn (since it's out of bounds)
    // Or if clipping clamps it to 99?
    // Cohen-Sutherland should reject x=100 completely if x_min=0, x_max=99.
    // Wait, clip_line takes width/height.
    // In rasterizer.rs: clip_line(fb.width() as i32, ...)
    // compute_out_code: if x >= width { code |= RIGHT }
    // So x=100 is RIGHT.
    // If both points are RIGHT, it rejects.
    // So this should draw nothing.
    for y in 0..100 {
        assert_eq!(fb.get_pixel(99, y), Some(0xFF000000));
    }
}

#[test]
fn test_draw_line_clip_boundary_minus_one() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let white = 0xFFFFFFFF;

    // Line exactly on the left boundary - 1
    draw_line(&mut fb, -1, 0, -1, 99, white);

    for y in 0..100 {
        assert_eq!(fb.get_pixel(0, y), Some(0xFF000000));
    }
}

#[test]
fn test_draw_line_enters_screen() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let white = 0xFFFFFFFF;

    // Line from -50 to 50
    draw_line(&mut fb, -50, 50, 50, 50, white);

    // Should draw from 0 to 50
    assert_eq!(fb.get_pixel(0, 50), Some(white));
    assert_eq!(fb.get_pixel(50, 50), Some(white));
    // Should not draw at -1 (obviously, but checking side effects)
}

proptest! {
    #[test]
    fn test_draw_line_fuzz(
        x0 in -200..300i32,
        y0 in -200..300i32,
        x1 in -200..300i32,
        y1 in -200..300i32
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let white = 0xFFFFFFFF;

        // This should not panic
        draw_line(&mut fb, x0, y0, x1, y1, white);

        // Verify that we didn't corrupt memory or panic.
        // Also verify that all drawn pixels are valid (though Framebuffer panics or ignores invalid set_pixel).
        // The key is that draw_line uses `unsafe` `set_pixel_unchecked` after clipping.
        // If clipping fails, we might get UB / crash.
        // Rust's Vec won't protect us in unchecked access (it's UB).
        // But running this in a test harness with many iterations helps catch crashes.
    }

    #[test]
    fn test_draw_line_huge_coords(
        x0 in -1000000..1000000i32,
        y0 in -1000000..1000000i32,
        x1 in -1000000..1000000i32,
        y1 in -1000000..1000000i32
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let white = 0xFFFFFFFF;
        draw_line(&mut fb, x0, y0, x1, y1, white);
    }
}
