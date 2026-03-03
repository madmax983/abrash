#![allow(clippy::unreadable_literal)]
use abrash::framebuffer::Framebuffer;
use abrash::post_process;

#[test]
fn test_apply_grayscale() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    // Fill with red
    fb.clear(0xFFFF0000);

    // Apply grayscale
    post_process::apply_grayscale(&mut fb);

    // Check pixel at (0, 0).
    // Red (255, 0, 0) -> (77*255 + 150*0 + 29*0) >> 8 = 19635 >> 8 = 76
    // Expected result: 0xFF4C4C4C (4C = 76)
    let pixel = fb.get_pixel(0, 0).unwrap();
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    assert_eq!(r, g);
    assert_eq!(g, b);
    assert_eq!(r, 76, "Expected grayscale value 76 for pure red, got {r}");
}

#[test]
fn test_apply_scanlines() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    // Fill with white
    fb.clear(0xFFFFFFFF);

    // Apply scanlines (darken odd rows)
    post_process::apply_scanlines(&mut fb);

    // Row 0 should be unchanged (white)
    let p0 = fb.get_pixel(0, 0).unwrap();
    assert_eq!(p0, 0xFFFFFFFF, "Row 0 should be unchanged");

    // Row 1 should be darkened.
    // Assuming implementation halves the brightness:
    // 0xFF -> 0x7F
    // Result: 0xFF7F7F7F
    let p1 = fb.get_pixel(0, 1).unwrap();
    assert_eq!(
        p1, 0xFF7F7F7F,
        "Row 1 should be darkened to half brightness"
    );
}
