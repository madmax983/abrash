use abrash_core::framebuffer::Framebuffer;

#[test]
fn test_clear_rect_matches_safe_fill() {
    let mut fb_safe = Framebuffer::new(100, 100).unwrap();
    let mut fb_unchecked = Framebuffer::new(100, 100).unwrap();

    fb_safe.clear(0);
    fb_unchecked.clear(0);

    // Using exact same input arguments
    fb_safe.clear_rect(10, 10, 50, 50, 0xFFFFFFFF);
    fb_unchecked.clear_rect(10, 10, 50, 50, 0xFFFFFFFF);

    assert_eq!(fb_safe.as_slice(), fb_unchecked.as_slice());
}
