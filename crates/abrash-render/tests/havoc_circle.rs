use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};

#[test]
fn test_havoc_circle_radius() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    draw_circle(&mut fb, 50, 50, i32::MAX, 0xFFFFFFFF);
}
