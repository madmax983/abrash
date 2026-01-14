use abrash::math::{Vec2, Mat2};

#[test]
fn test_vec2_new() {
    let v = Vec2::new(3.0, 4.0);
    assert_eq!(v.x, 3.0);
    assert_eq!(v.y, 4.0);
}

#[test]
fn test_vec2_add() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let result = a + b;
    assert_eq!(result, Vec2::new(4.0, 6.0));
}

#[test]
fn test_vec2_sub() {
    let a = Vec2::new(5.0, 7.0);
    let b = Vec2::new(2.0, 3.0);
    let result = a - b;
    assert_eq!(result, Vec2::new(3.0, 4.0));
}

#[test]
fn test_vec2_scale() {
    let v = Vec2::new(2.0, 3.0);
    let result = v * 2.5;
    assert_eq!(result, Vec2::new(5.0, 7.5));
}

#[test]
fn test_mat2_rotation() {
    use std::f32::consts::PI;

    // 90 degree rotation
    let mat = Mat2::rotation(PI / 2.0);
    let v = Vec2::new(1.0, 0.0);
    let result = mat.transform(v);

    // Allow small floating point error
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 1.0).abs() < 0.0001);
}

#[test]
fn test_rotation_preserves_length() {
    use std::f32::consts::PI;

    let v = Vec2::new(3.0, 4.0);
    let original_length = (v.x * v.x + v.y * v.y).sqrt();

    // Test multiple rotation angles
    for angle in [0.0, PI / 4.0, PI / 2.0, PI, 2.0 * PI] {
        let mat = Mat2::rotation(angle);
        let rotated = mat.transform(v);
        let rotated_length = (rotated.x * rotated.x + rotated.y * rotated.y).sqrt();

        assert!((original_length - rotated_length).abs() < 0.0001,
                "Rotation should preserve vector length");
    }
}
