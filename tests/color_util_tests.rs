use abrash::rasterizer::color_to_u32;
use abrash::math::Vec4;

#[test]
fn test_color_to_u32() {
    let white = Vec4::new(1.0, 1.0, 1.0, 1.0);
    assert_eq!(color_to_u32(white), 0xFFFFFFFF);

    let red = Vec4::new(1.0, 0.0, 0.0, 1.0);
    assert_eq!(color_to_u32(red), 0xFFFF0000);

    let semi_transparent_blue = Vec4::new(0.0, 0.0, 1.0, 0.5);
    // 0.5 * 255 = 127.5 -> 127 = 0x7F
    assert_eq!(color_to_u32(semi_transparent_blue), 0x7F0000FF);
}
