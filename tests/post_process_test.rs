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
    assert_eq!(r, 76, "Expected grayscale value 76 for pure red, got {}", r);
}

#[test]
fn test_apply_grayscale_large() {
    // 16 pixels (4x4), enough to trigger AVX2 path (chunks of 8)
    let mut fb = Framebuffer::new(4, 4).unwrap();
    // Fill with red
    fb.clear(0xFFFF0000);

    // Apply grayscale
    post_process::apply_grayscale(&mut fb);

    // Check all pixels
    for y in 0..4 {
        for x in 0..4 {
            let pixel = fb.get_pixel(x, y).unwrap();
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            assert_eq!(r, 76, "Pixel at ({}, {}) should be gray 76", x, y);
            assert_eq!(g, 76);
            assert_eq!(b, 76);
        }
    }
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
    assert_eq!(p1, 0xFF7F7F7F, "Row 1 should be darkened to half brightness");
}

#[test]
fn test_apply_scanlines_large() {
    let mut fb = Framebuffer::new(8, 4).unwrap();
    fb.clear(0xFFFFFFFF);

    post_process::apply_scanlines(&mut fb);

    // Row 0, 2 (even) -> Unchanged
    for x in 0..8 {
        assert_eq!(fb.get_pixel(x, 0).unwrap(), 0xFFFFFFFF);
        assert_eq!(fb.get_pixel(x, 2).unwrap(), 0xFFFFFFFF);
    }

    // Row 1, 3 (odd) -> Darkened
    for x in 0..8 {
        assert_eq!(fb.get_pixel(x, 1).unwrap(), 0xFF7F7F7F);
        assert_eq!(fb.get_pixel(x, 3).unwrap(), 0xFF7F7F7F);
    }
}

#[test]
fn test_apply_invert() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    // Fill with white
    fb.clear(0xFFFFFFFF);

    // Apply invert (should become black, alpha preserved)
    post_process::apply_invert(&mut fb);

    let p0 = fb.get_pixel(0, 0).unwrap();
    assert_eq!(p0, 0xFF000000, "White (0xFFFFFFFF) inverted should be Black (0xFF000000)");

    // Test another color: Red 0xFFFF0000 -> Cyan 0xFF00FFFF
    fb.set_pixel(0, 0, 0xFFFF0000);
    post_process::apply_invert(&mut fb);
    let p_red = fb.get_pixel(0, 0).unwrap();
    assert_eq!(p_red, 0xFF00FFFF, "Red (0xFFFF0000) inverted should be Cyan (0xFF00FFFF)");
}

#[test]
fn test_apply_invert_large() {
    let mut fb = Framebuffer::new(8, 2).unwrap();
    fb.clear(0xFFFFFFFF);

    post_process::apply_invert(&mut fb);

    for y in 0..2 {
        for x in 0..8 {
            assert_eq!(fb.get_pixel(x, y).unwrap(), 0xFF000000);
        }
    }
}
