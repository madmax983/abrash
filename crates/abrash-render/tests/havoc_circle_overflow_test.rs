use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::circle::draw_circle;

#[test]
fn test_havoc_circle_overflow_fast_path() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    // xc = i32::MAX - 5, radius = 10
    // xc - radius = i32::MAX - 15 (>= 0)
    // xc + radius = i32::MAX + 5 (overflows to negative, so < 100 is true)
    // yc = 50, radius = 10
    // yc - radius = 40 (>= 0)
    // yc + radius = 60 (< 100)
    // This will take the fast path and use get_unchecked_mut with an out of bounds index!
    draw_circle(&mut fb, i32::MAX - 5, 50, 10, 0xFFFFFFFF);
}
