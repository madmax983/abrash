use abrash::experimental::synthesizer::{
    Blend, Checkerboard, LinearGradient, Solid, Synthesizer, ValueNoise,
};

#[test]
fn test_solid_synthesizer() {
    let solid = Solid::new(0xFF_11_22_33);
    assert_eq!(solid.sample(0.0, 0.0), 0xFF_11_22_33);
    assert_eq!(solid.sample(0.5, 0.5), 0xFF_11_22_33);
    assert_eq!(solid.sample(1.0, 1.0), 0xFF_11_22_33);
}

#[test]
fn test_linear_gradient() {
    let grad = LinearGradient::new(0xFF_00_00_00, 0xFF_FF_FF_FF);
    assert_eq!(grad.sample(0.0, 0.5), 0xFF_00_00_00);
    // At u=1.0, weight is 256. blend_swar(c0, c1, w, inv_w) = (c0 * w + c1 * inv_w) / 256.
    // Given the arguments (end_color, start_color, weight, 256-weight), at weight=256 it fully samples end_color.
    // NOTE: blend_swar calculates alpha per channel.
    let sampled = grad.sample(1.0, 0.5);
    let r = (sampled >> 16) & 0xFF;
    assert!(r >= 254); // Should be very close to white
}

#[test]
fn test_checkerboard() {
    let c1 = Solid::new(0xFF_FF_00_00);
    let c2 = Solid::new(0xFF_00_FF_00);
    let check = Checkerboard::new(c1, c2, 2); // 2x2 grid

    // (0,0) is even (0+0 = 0) -> c1
    assert_eq!(check.sample(0.25, 0.25), 0xFF_FF_00_00);
    // (1,0) is odd (1+0 = 1) -> c2
    assert_eq!(check.sample(0.75, 0.25), 0xFF_00_FF_00);
}

#[test]
fn test_blend() {
    let c1 = Solid::new(0xFF_00_00_00);
    let c2 = Solid::new(0xFF_FF_FF_FF);
    let blend = Blend::new(c1, c2, 0.5);

    // 50% blend of black and white should be approx 0xFF_7F_7F_7F
    let sampled = blend.sample(0.5, 0.5);
    let r = (sampled >> 16) & 0xFF;
    assert!((r as i32 - 127).abs() <= 1, "Expected ~127, got {r}");
}

#[test]
fn test_value_noise() {
    let noise = ValueNoise::new(1234, 10.0, 0xFF_00_00_00, 0xFF_FF_FF_FF);
    let _s = noise.sample(0.5, 0.5); // Ensure it runs without panic
}
