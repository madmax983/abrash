#![cfg(feature = "nova")]

use abrash_render::experimental::sharpen::apply_sharpen;
use abrash_render::framebuffer::Framebuffer;

#[test]
fn test_apply_sharpen_basic() {
    let mut fb = Framebuffer::new(3, 3).unwrap();

    // Set up a 3x3 image with a dark center and light edges
    // Background
    fb.clear(0xFFFF_FFFF);
    // Dark center pixel
    fb.set_pixel(1, 1, 0xFF00_0000);

    // After sharpening with a basic 3x3 kernel:
    //  0 -1  0
    // -1  5 -1
    //  0 -1  0
    // The center pixel should remain black (or become very dark if clamped)
    // The adjacent white pixels should be pushed "whiter" or clipped at white.
    // Let's test a non-extreme case to be sure it actually applies math.

    let mut fb2 = Framebuffer::new(3, 3).unwrap();
    // 0xAA (170) gray background
    fb2.clear(0xFFAA_AAAA);
    // 0x55 (85) dark center
    fb2.set_pixel(1, 1, 0xFF55_5555);

    // Apply 1.0 (full) sharpen
    // Center pixel:
    // Original = 85
    // Neighbors = 170 (top, bottom, left, right)
    // New center = 5 * 85 - 170 - 170 - 170 - 170 = 425 - 680 = -255 -> clamped to 0
    // So center pixel should become black (0x00)
    apply_sharpen(
        &mut fb2,
        &abrash_render::experimental::sharpen::SharpenConfig { amount: 1.0 },
    );

    let center_pixel = fb2.get_pixel(1, 1).unwrap();
    let r = (center_pixel >> 16) & 0xFF;
    let g = (center_pixel >> 8) & 0xFF;
    let b = center_pixel & 0xFF;

    assert_eq!(r, 0, "Center pixel R channel should be 0, got {r}");
    assert_eq!(g, 0, "Center pixel G channel should be 0, got {g}");
    assert_eq!(b, 0, "Center pixel B channel should be 0, got {b}");

    // Test that the border remains unmodified
    let border_pixel = fb2.get_pixel(0, 0).unwrap();
    assert_eq!(
        border_pixel, 0xFFAA_AAAA,
        "Border pixel should remain unmodified"
    );
}

#[test]
fn test_apply_sharpen_amount_zero() {
    let mut fb = Framebuffer::new(3, 3).unwrap();
    fb.clear(0xFFAA_AAAA);
    fb.set_pixel(1, 1, 0xFF55_5555);

    apply_sharpen(
        &mut fb,
        &abrash_render::experimental::sharpen::SharpenConfig { amount: 0.0 },
    );

    assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF55_5555);
}
