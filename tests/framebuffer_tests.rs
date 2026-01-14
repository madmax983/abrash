use abrash::framebuffer::Framebuffer;

#[test]
fn test_framebuffer_new() {
    let fb = Framebuffer::new(800, 600);
    assert_eq!(fb.width(), 800);
    assert_eq!(fb.height(), 600);
    assert_eq!(fb.as_slice().len(), 800 * 600);
}

#[test]
fn test_framebuffer_initialized_to_black() {
    let fb = Framebuffer::new(10, 10);
    for &pixel in fb.as_slice() {
        assert_eq!(pixel, 0xFF00_0000); // Black with full alpha
    }
}

#[test]
fn test_framebuffer_clear() {
    let mut fb = Framebuffer::new(10, 10);
    let red = 0xFFFF_0000; // Red with full alpha

    fb.clear(red);

    for &pixel in fb.as_slice() {
        assert_eq!(pixel, red);
    }
}

#[test]
fn test_framebuffer_set_pixel() {
    let mut fb = Framebuffer::new(10, 10);
    let white = 0xFFFF_FFFF;

    fb.set_pixel(5, 3, white);

    assert_eq!(fb.get_pixel(5, 3), Some(white));
}

#[test]
fn test_framebuffer_bounds_checking() {
    let mut fb = Framebuffer::new(10, 10);
    let white = 0xFFFF_FFFF;

    // Out of bounds should be ignored
    fb.set_pixel(100, 100, white);
    fb.set_pixel(-1, -1, white);

    assert_eq!(fb.get_pixel(100, 100), None);
    assert_eq!(fb.get_pixel(-1, -1), None);
}
