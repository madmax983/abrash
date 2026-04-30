use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::{draw_rounded_rect, fill_rounded_rect, draw_rect, fill_rect};
use std::time::Instant;

#[test]
fn test_off_screen_rounded_rect_performance() {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    let start = Instant::now();

    // Massive offscreen rounded rect
    fill_rounded_rect(
        &mut fb,
        -2147483640,
        -2147483640,
        2147483000,
        2147483000,
        1073741000,
        0xFFFFFFFF,
    );

    let duration = start.elapsed();

    // It shouldn't take more than a millisecond if properly optimized (early exit)
    assert!(duration.as_millis() < 50, "Off-screen rounded rect rendering took too long: {} ms. Expected early bounds rejection.", duration.as_millis());
}

#[test]
fn test_off_screen_draw_rounded_rect_performance() {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    let start = Instant::now();

    // Massive offscreen rounded rect
    draw_rounded_rect(
        &mut fb,
        -2147483640,
        -2147483640,
        2147483000,
        2147483000,
        1073741000,
        0xFFFFFFFF,
    );

    let duration = start.elapsed();

    // It shouldn't take more than a millisecond if properly optimized (early exit)
    assert!(duration.as_millis() < 50, "Off-screen draw rounded rect rendering took too long: {} ms. Expected early bounds rejection.", duration.as_millis());
}
