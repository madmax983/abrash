use abrash::rasterizer::{FilterMode, Texture};

#[test]
fn test_bilinear_interpolation() {
    let mut tex = Texture::new(2, 2).unwrap();
    tex.filter_mode = FilterMode::Bilinear;

    // Layout:
    // Red   Green
    // Blue  White
    let c_red = 0xFFFF_0000;
    let c_green = 0xFF00_FF00;
    let c_blue = 0xFF00_00FF;
    let c_white = 0xFFFF_FFFF;

    tex.set_pixel(0, 0, c_red);
    tex.set_pixel(1, 0, c_green);
    tex.set_pixel(0, 1, c_blue);
    tex.set_pixel(1, 1, c_white);

    // 1. Center of Top-Left Pixel (0,0) -> Should be Red
    // u = 0.25, v = 0.25 -> u_tex = 0.5, v_tex = 0.5 -> u_img = 0.0, v_img = 0.0
    // wx = 0, wy = 0 -> pure c00
    let c = tex.get_pixel(0.25, 0.25);
    assert_eq!(c, c_red, "Center of TL pixel should be Red");

    // 2. Center of Top-Right Pixel (1,0) -> Should be Green
    // u = 0.75, v = 0.25
    let c = tex.get_pixel(0.75, 0.25);
    assert_eq!(c, c_green, "Center of TR pixel should be Green");

    // 3. Exact Middle of Texture -> Average of all 4
    // u = 0.5, v = 0.5 -> u_tex = 1.0, v_tex = 1.0 -> u_img = 0.5, v_img = 0.5
    // wx = 128, wy = 128
    let c = tex.get_pixel(0.5, 0.5);

    // Expected:
    // Top blend: (Red + Green) / 2 = (FF0000 + 00FF00) / 2 = 7F7F00
    // Bottom blend: (Blue + White) / 2 = (0000FF + FFFFFF) / 2 = 7F7FFF
    // Final: (7F7F00 + 7F7FFF) / 2 = 7F7F7F (approx)

    // We need to be careful with rounding in the implementation.
    // The implementation uses shifts.

    // Let's check components.
    // Red channel: (255 + 0 + 0 + 255) / 4 = 127.5 -> 127
    // Green channel: (0 + 255 + 0 + 255) / 4 = 127.5 -> 127
    // Blue channel: (0 + 0 + 255 + 255) / 4 = 127.5 -> 127

    let r = (c >> 16) & 0xFF;
    let g = (c >> 8) & 0xFF;
    let b = c & 0xFF;

    // Allow small error due to integer math
    assert!((r as i32 - 127).abs() <= 1, "Red expected ~127, got {r}");
    assert!((g as i32 - 127).abs() <= 1, "Green expected ~127, got {g}");
    assert!((b as i32 - 127).abs() <= 1, "Blue expected ~127, got {b}");

    // 4. Horizontal edge between Red and Green (top row)
    // u = 0.5, v = 0.25
    let c = tex.get_pixel(0.5, 0.25);
    // Red + Green / 2
    let r = (c >> 16) & 0xFF;
    let g = (c >> 8) & 0xFF;
    let b = c & 0xFF;

    assert!((r as i32 - 127).abs() <= 1, "Red expected ~127, got {r}");
    assert!((g as i32 - 127).abs() <= 1, "Green expected ~127, got {g}");
    assert_eq!(b, 0, "Blue expected 0, got {b}");
}
