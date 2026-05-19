#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sepia::apply_sepia;

#[test]
fn test_sepia_colors() {
    let mut fb = Framebuffer::new(3, 1).unwrap();

    // Pure White, Pure Red, Pure Black
    fb.set_pixel(0, 0, 0xFFFF_FFFF);
    fb.set_pixel(1, 0, 0xFFFF_0000);
    fb.set_pixel(2, 0, 0xFF00_0000);

    apply_sepia(&mut fb);

    let p1 = fb.get_pixel(0, 0).unwrap();
    let p2 = fb.get_pixel(1, 0).unwrap();
    let p3 = fb.get_pixel(2, 0).unwrap();

    // Math for White (255, 255, 255)
    // R = 255 * 0.393 + 255 * 0.769 + 255 * 0.189 = 344 (clamped to 255)
    // G = 255 * 0.349 + 255 * 0.686 + 255 * 0.168 = 306 (clamped to 255)
    // B = 255 * 0.272 + 255 * 0.534 + 255 * 0.131 = 238
    assert_eq!(p1, 0xFFFF_FFEE, "White should become light sepia");

    // Math for Red (255, 0, 0)
    // R = 255 * 0.393 = 100
    // G = 255 * 0.349 = 88
    // B = 255 * 0.272 = 69
    assert_eq!(p2, 0xFF64_5845, "Red should become dark sepia");

    // Math for Black (0, 0, 0)
    // R, G, B = 0
    assert_eq!(p3, 0xFF00_0000, "Black should remain black");
}
