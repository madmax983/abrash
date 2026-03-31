use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_chromatic_aberration;

#[test]
fn test_chromatic_aberration_shift() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw a white pixel at (50, 50)
    fb.set_pixel(50, 50, 0xFFFF_FFFF);

    // Apply effect with offset 5
    apply_chromatic_aberration(
        &mut fb,
        &abrash::post_process::ChromaticAberrationConfig { offset: 5 },
    );

    // Original position (50, 50) should have:
    // Red: comes from (45, 50) -> Black (0)
    // Green: comes from (50, 50) -> White (255)
    // Blue: comes from (55, 50) -> Black (0)
    // So (50, 50) should be Green (0xFF00_FF00)
    // Wait, let's trace carefully:
    // NewPixel(x,y).Red = OldPixel(x - offset, y).Red
    // NewPixel(x,y).Green = OldPixel(x, y).Green
    // NewPixel(x,y).Blue = OldPixel(x + offset, y).Blue

    // At (50, 50):
    // R = Old(45, 50).R = 0
    // G = Old(50, 50).G = 255
    // B = Old(55, 50).B = 0
    // Result: 0, 255, 0 (Green)
    assert_eq!(
        fb.get_pixel(50, 50).unwrap() & 0x00FF_FFFF,
        0x0000_FF00,
        "Center pixel should be Green"
    );

    // At (55, 50):
    // R = Old(50, 50).R = 255
    // G = Old(55, 50).G = 0
    // B = Old(60, 50).B = 0
    // Result: 255, 0, 0 (Red)
    assert_eq!(
        fb.get_pixel(55, 50).unwrap() & 0x00FF_FFFF,
        0x00FF_0000,
        "Right-shifted pixel should receive Red"
    );

    // At (45, 50):
    // R = Old(40, 50).R = 0
    // G = Old(45, 50).G = 0
    // B = Old(50, 50).B = 255
    // Result: 0, 0, 255 (Blue)
    assert_eq!(
        fb.get_pixel(45, 50).unwrap() & 0x00FF_FFFF,
        0x0000_00FF,
        "Left-shifted pixel should receive Blue"
    );
}

#[test]
fn test_chromatic_aberration_boundary() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw white pixel at (0, 5)
    fb.set_pixel(0, 5, 0xFFFF_FFFF);

    apply_chromatic_aberration(
        &mut fb,
        &abrash::post_process::ChromaticAberrationConfig { offset: 2 },
    );

    // At (0, 5):
    // R = Old(-2, 5) -> 0 (Black)
    // G = Old(0, 5) -> 255
    // B = Old(2, 5) -> 0
    // Result: Green
    assert_eq!(fb.get_pixel(0, 5).unwrap() & 0x00FF_FFFF, 0x0000_FF00);

    // At (2, 5):
    // R = Old(0, 5) -> 255
    // G = Old(2, 5) -> 0
    // B = Old(4, 5) -> 0
    // Result: Red
    assert_eq!(fb.get_pixel(2, 5).unwrap() & 0x00FF_FFFF, 0x00FF_0000);
}
