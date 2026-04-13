use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::circle::{draw_circle, fill_circle};

#[test]
fn test_draw_circle_overflow() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    // 3 - 2 * 2_000_000_000 panics because of multiply with overflow
    draw_circle(&mut fb, 50, 50, 2_000_000_000, 0xFFFFFFFF);
}

#[test]
fn test_fill_circle_overflow() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    fill_circle(&mut fb, 50, 50, 2_000_000_000, 0xFFFFFFFF);
}
