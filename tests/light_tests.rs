use abrash::light::{color_to_u32, u32_to_color, DirectionalLight};
use abrash::math::Vec3;

#[test]
fn test_directional_light_intensity() {
    let light = DirectionalLight::new(
        Vec3::new(0.0, -1.0, 0.0), // Light pointing down
        Vec3::new(1.0, 1.0, 1.0),  // White light
    );

    // Surface facing up should be fully lit
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let intensity = light.intensity(normal);
    assert!((intensity - 1.0).abs() < 0.001);

    // Surface facing down should be unlit
    let normal = Vec3::new(0.0, -1.0, 0.0);
    let intensity = light.intensity(normal);
    assert!(intensity < 0.001);
}

#[test]
fn test_color_conversion_roundtrip() {
    let original = 0xFFFF8040; // Orange
    let color = u32_to_color(original);
    let back = color_to_u32(color);
    assert_eq!(back, original);
}

#[test]
fn test_color_to_u32() {
    let white = Vec3::new(1.0, 1.0, 1.0);
    assert_eq!(color_to_u32(white), 0xFFFFFFFF);

    let red = Vec3::new(1.0, 0.0, 0.0);
    assert_eq!(color_to_u32(red), 0xFFFF0000);
}
