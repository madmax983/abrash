use abrash_core::math::henyey_greenstein;
use abrash_core::sdf::parabola_2d;
use abrash_core::math::Vec2;

#[test]
fn test_henyey_greenstein() {
    // Basic test that the function returns a valid output.
    let result = henyey_greenstein(1.0, 0.5);
    assert!(result.is_finite());
}

#[test]
fn test_parabola_2d() {
    let result = parabola_2d(Vec2::new(1.0, 1.0), 0.5);
    assert!(result.is_finite());
}

#[test]
fn test_parabola_2d_negative_cbrt() {
    // Tests that negative values evaluate correctly with cbrt() instead of returning NaN
    let result = parabola_2d(Vec2::new(-1.0, -1.0), 0.5);
    assert!(!result.is_nan(), "Should not produce NaN with negative inputs");
}
