#![allow(missing_docs)]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::{draw_rounded_rect, fill_rounded_rect};
use std::time::Instant;

#[test]
fn test_off_screen_rounded_rect_performance() {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    let start = Instant::now();

    // Massive offscreen rounded rect
    fill_rounded_rect(
        &mut fb,
        -2_147_483_640,
        -2_147_483_640,
        2_147_483_000,
        2_147_483_000,
        1_073_741_000,
        0xFFFF_FFFF,
    );

    let duration = start.elapsed();

    // It shouldn't take more than a millisecond if properly optimized (early exit)
    assert!(
        duration.as_millis() < 50,
        "Off-screen rounded rect rendering took too long: {} ms. Expected early bounds rejection.",
        duration.as_millis()
    );
}

#[test]
fn test_off_screen_draw_rounded_rect_performance() {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    let start = Instant::now();

    // Massive offscreen rounded rect
    draw_rounded_rect(
        &mut fb,
        -2_147_483_640,
        -2_147_483_640,
        2_147_483_000,
        2_147_483_000,
        1_073_741_000,
        0xFFFF_FFFF,
    );

    let duration = start.elapsed();

    // It shouldn't take more than a millisecond if properly optimized (early exit)
    assert!(
        duration.as_millis() < 50,
        "Off-screen draw rounded rect rendering took too long: {} ms. Expected early bounds rejection.",
        duration.as_millis()
    );
}
