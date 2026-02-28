use abrash::framebuffer::Framebuffer;

#[test]
fn test_clear_rect_partial_bounds() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    fb.clear_rect(5, 5, 10, 10, 0xFFFFFFFF);
    // Add tests for various bounds combinations
}
