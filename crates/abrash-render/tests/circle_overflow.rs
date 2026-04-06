use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::circle::draw_circle;

#[test]
fn test_draw_circle_overflow() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    // Use a large radius to trigger overflow in d calculation
    // d = 3 - 2 * radius
    draw_circle(&mut fb, 5, 5, 2_147_483_647, 0xFFFF_FFFF);
}
