use abrash::framebuffer::Framebuffer;
use abrash::post_process::filters::apply_gamma_correction;

#[test]
fn test_gamma_correction_srgb() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    // Start with a mid-gray pixel (128, 128, 128)
    fb.set_pixel(0, 0, 0xFF80_8080);

    // Apply gamma correction with gamma 2.2 (approx 1/2.2 = 0.4545)
    // Formula: 255 * (128/255)^(1/2.2)
    // 128 / 255 ≈ 0.50196
    // 0.50196 ^ 0.4545 ≈ 0.7297
    // 0.7297 * 255 ≈ 186.08 -> 186
    // Expected output color is roughly 186 (0xBA)
    apply_gamma_correction(&mut fb, 2.2);

    let pixel = fb.get_pixel(0, 0).unwrap();
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    assert_eq!(r, 186, "Red channel incorrect after gamma correction");
    assert_eq!(g, 186, "Green channel incorrect after gamma correction");
    assert_eq!(b, 186, "Blue channel incorrect after gamma correction");
}

#[test]
fn test_gamma_correction_identity() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    // Start with a mid-gray pixel (128, 128, 128)
    fb.set_pixel(0, 0, 0xFF80_8080);

    // Apply gamma correction with gamma 1.0 (no change)
    apply_gamma_correction(&mut fb, 1.0);

    let pixel = fb.get_pixel(0, 0).unwrap();
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    assert_eq!(r, 128, "Red channel should remain unchanged with gamma 1.0");
    assert_eq!(g, 128, "Green channel should remain unchanged with gamma 1.0");
    assert_eq!(b, 128, "Blue channel should remain unchanged with gamma 1.0");
}
