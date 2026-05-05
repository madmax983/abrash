use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_scanlines;

#[test]
fn test_verify_scanlines_correctness() {
    let width = 4;
    let height = 4;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with pattern
    // Row 0: White (Full brightness)
    // Row 1: White (Should be darkened)
    // Row 2: Red
    // Row 3: Semi-transparent Green
    for x in 0..width {
        fb.set_pixel(x as i32, 0, 0xFFFF_FFFF);
        fb.set_pixel(x as i32, 1, 0xFFFF_FFFF);
        fb.set_pixel(x as i32, 2, 0xFFFF_0000);
        // Alpha 0x80, Green 0xFF -> 0x8000FF00
        fb.set_pixel(x as i32, 3, 0x8000_FF00);
    }

    apply_scanlines(&mut fb);

    // Verify Row 0 (Even) - Unchanged
    for x in 0..width {
        assert_eq!(
            fb.get_pixel(x as i32, 0).unwrap(),
            0xFFFF_FFFF,
            "Row 0 pixel {x} modified"
        );
    }

    // Verify Row 1 (Odd) - Darkened
    // 0xFF >> 1 = 0x7F
    // Expected: 0xFF7F7F7F
    for x in 0..width {
        assert_eq!(
            fb.get_pixel(x as i32, 1).unwrap(),
            0xFF7F_7F7F,
            "Row 1 pixel {x} incorrect"
        );
    }

    // Verify Row 2 (Even) - Unchanged
    for x in 0..width {
        assert_eq!(
            fb.get_pixel(x as i32, 2).unwrap(),
            0xFFFF_0000,
            "Row 2 pixel {x} modified"
        );
    }

    // Verify Row 3 (Odd) - Darkened + Alpha Preservation
    // Original: 0x8000FF00 (Alpha 128)
    // SWAR Logic correctly preserves Alpha: 0x80007F00
    for x in 0..width {
        assert_eq!(
            fb.get_pixel(x as i32, 3).unwrap(),
            0x8000_7F00,
            "Row 3 pixel {x} incorrect (alpha check)"
        );
    }
}
