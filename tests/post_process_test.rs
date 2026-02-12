use abrash::framebuffer::Framebuffer;
use abrash::post_process;

#[test]
fn test_apply_sepia() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    // Use a known color: Red
    fb.set_pixel(0, 0, 0xFFFF0000);

    post_process::apply_sepia(&mut fb);

    let pixel = fb.get_pixel(0, 0).unwrap();
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    // Allow small margin of error for fixed point
    assert!((r as i32 - 100).abs() <= 2, "Expected R ~ 100, got {}", r);
    assert!((g as i32 - 89).abs() <= 2, "Expected G ~ 89, got {}", g);
    assert!((b as i32 - 69).abs() <= 2, "Expected B ~ 69, got {}", b);
}

#[test]
fn test_apply_grayscale() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    fb.set_pixel(0, 0, 0xFFFF0000); // Red
    post_process::apply_grayscale(&mut fb);
    // Red component is 255. 77*255/256 = 76.
    let p = fb.get_pixel(0, 0).unwrap();
    assert_eq!(p & 0xFF, 76);
}

#[test]
fn test_apply_scanlines() {
    let mut fb = Framebuffer::new(1, 2).unwrap();
    fb.clear(0xFFFFFFFF); // White
    post_process::apply_scanlines(&mut fb);

    // Row 0 is untouched
    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);

    // Row 1 is darkened (halved)
    // 0xFF >> 1 = 0x7F
    assert_eq!(fb.get_pixel(0, 1).unwrap(), 0xFF7F7F7F);
}

#[test]
fn test_apply_invert() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    fb.set_pixel(0, 0, 0xFF000000); // Black
    post_process::apply_invert(&mut fb);
    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF); // White
}
