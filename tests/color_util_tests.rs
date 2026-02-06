use abrash::math::Vec3;
use abrash::rasterizer::color_to_u32;

#[test]
fn test_color_to_u32() {
    let white = Vec3::new(1.0, 1.0, 1.0);
    assert_eq!(color_to_u32(white), 0xFFFF_FFFF);

    let red = Vec3::new(1.0, 0.0, 0.0);
    assert_eq!(color_to_u32(red), 0xFFFF_0000);
}
