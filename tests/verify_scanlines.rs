use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_scanlines;

#[test]
fn test_verify_scanlines() {
    let width = 4;
    let height = 4;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Fill with white
    fb.clear(0xFFFFFFFF);

    apply_scanlines(&mut fb);

    // Row 0 (Even) should be untouched
    for x in 0..width {
        assert_eq!(fb.get_pixel(x as i32, 0).unwrap(), 0xFFFFFFFF);
    }

    // Row 1 (Odd) should be darkened
    // 0xFFFFFFFF -> 0xFF7F7F7F
    for x in 0..width {
        assert_eq!(fb.get_pixel(x as i32, 1).unwrap(), 0xFF7F7F7F);
    }

    // Row 2 (Even) should be untouched
    for x in 0..width {
        assert_eq!(fb.get_pixel(x as i32, 2).unwrap(), 0xFFFFFFFF);
    }
}
