use abrash::light::{color_to_u32, u32_to_color};
use abrash::math::Vec3;

#[test]
fn test_color_conversion_roundtrip() {
    let original = 0xFFFF8040; // Orange
    let color = u32_to_color(original);
    let back = color_to_u32(color);

    // Note: Precision loss in round trip (u8 -> f32 -> u8)
    // 0x80 (128) -> 0.50196 -> 128
    // 0x40 (64) -> 0.25098 -> 64
    // But sometimes it might be off by 1 depending on float precision/rounding.
    // Let's check tolerance if needed, or exact match if logic is robust.

    // With round(), it should be exact.
    // The implementation uses cast to u32 which is floor.
    // ((val / 255.0) * 255.0) as u32
    // float ops are not always exact.

    // Let's rely on the original test's expectation of equality.
    assert_eq!(back, original);
}

#[test]
fn test_color_to_u32() {
    let white = Vec3::new(1.0, 1.0, 1.0);
    assert_eq!(color_to_u32(white), 0xFFFFFFFF);

    let red = Vec3::new(1.0, 0.0, 0.0);
    assert_eq!(color_to_u32(red), 0xFFFF0000);
}
