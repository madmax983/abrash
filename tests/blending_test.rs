use abrash::framebuffer::Framebuffer;

#[test]
fn test_alpha_blending() {
    let mut fb = Framebuffer::new(1, 1).unwrap();

    // Set background to Blue
    fb.set_pixel(0, 0, 0xFF0000FF);

    // Draw 50% Red over it: 0x80FF0000
    // Expected:
    // Alpha: 0xFF (usually kept at max for framebuffer) or blended?
    // Let's assume standard premultiplied or straight alpha.
    // Straight alpha:
    // Out = Src * A + Dst * (1 - A)
    // Red: 0xFF * 0.5 + 0x00 * 0.5 = 127 (0x7F)
    // Green: 0x00 * 0.5 + 0x00 * 0.5 = 0
    // Blue: 0x00 * 0.5 + 0xFF * 0.5 = 127 (0x7F)
    // Result should be roughly 0xFF7F007F

    fb.set_pixel(0, 0, 0x80FF0000);

    let pixel = fb.get_pixel(0, 0).unwrap();

    // Note: The alpha component of the result depends on the blend mode.
    // Usually the framebuffer alpha is ignored or set to 0xFF (Opaque).

    let r = (pixel >> 16) & 0xFF;
    let b = pixel & 0xFF;

    assert!(r >= 0x7E && r <= 0x81, "Red component should be ~127, got {}", r);
    assert!(b >= 0x7E && b <= 0x81, "Blue component should be ~127, got {}", b);
}
