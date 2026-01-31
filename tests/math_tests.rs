use abrash::math::{Mat2, Mat4, Vec2, Vec3};

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

        assert!(
            (original_length - rotated_length).abs() < 0.0001,
            "Rotation should preserve vector length"
        );
    }
}

#[test]
fn test_vec3_new() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
    assert_eq!(v.z, 3.0);
}

#[test]
fn test_vec3_dot() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 1.0, 0.0);
    assert_eq!(a.dot(b), 0.0);

    let c = Vec3::new(1.0, 2.0, 3.0);
    let d = Vec3::new(4.0, 5.0, 6.0);
    assert_eq!(c.dot(d), 32.0);
}

#[test]
fn test_vec3_cross() {
    let x = Vec3::new(1.0, 0.0, 0.0);
    let y = Vec3::new(0.0, 1.0, 0.0);
    let z = x.cross(y);
    assert!((z.z - 1.0).abs() < 0.001);
}

#[test]
fn test_vec3_normalize() {
    let v = Vec3::new(3.0, 0.0, 4.0);
    let n = v.normalize();
    assert!((n.length() - 1.0).abs() < 0.001);
}

#[test]
fn test_mat4_identity() {
    let m = Mat4::identity();
    let v = Vec3::new(1.0, 2.0, 3.0);
    let (result, w) = m.transform_point(v);
    assert!((result.x - 1.0).abs() < 0.001);
    assert!((result.y - 2.0).abs() < 0.001);
    assert!((result.z - 3.0).abs() < 0.001);
    assert!((w - 1.0).abs() < 0.001);
}

#[test]
fn test_mat4_translation() {
    let m = Mat4::translation(10.0, 20.0, 30.0);
    let v = Vec3::new(1.0, 2.0, 3.0);
    let (result, _) = m.transform_point(v);
    assert!((result.x - 11.0).abs() < 0.001);
    assert!((result.y - 22.0).abs() < 0.001);
    assert!((result.z - 33.0).abs() < 0.001);
}

#[test]
fn test_mat4_scale() {
    let m = Mat4::scale(2.0, 3.0, 4.0);
    let v = Vec3::new(1.0, 1.0, 1.0);
    let (result, _) = m.transform_point(v);
    assert!((result.x - 2.0).abs() < 0.001);
    assert!((result.y - 3.0).abs() < 0.001);
    assert!((result.z - 4.0).abs() < 0.001);
}

#[test]
fn test_mat4_perspective() {
    use std::f32::consts::PI;
    let proj = Mat4::perspective(PI / 2.0, 1.0, 0.1, 100.0);
    let v = Vec3::new(0.0, 0.0, -1.0);
    let (_, w) = proj.transform_point(v);
    assert!(w.abs() > 0.001);
}

#[test]
fn test_mat4_transform_normal() {
    let m = Mat4::rotation_y(std::f32::consts::PI / 2.0); // 90 degree Y rotation
    let normal = Vec3::new(0.0, 0.0, 1.0); // Pointing +Z

    let result = m.transform_normal(normal);

    // After 90 degree Y rotation, +Z becomes +X
    assert!((result.x - 1.0).abs() < 0.01);
    assert!(result.y.abs() < 0.01);
    assert!(result.z.abs() < 0.01);
}

#[test]
fn test_vec3_normalize_tiny() {
    let v = Vec3::new(0.000001, 0.0, 0.0);
    let n = v.normalize();
    // Specific behavior of this math lib:
    // If length < 0.0001, it returns the vector itself instead of normalizing or returning zero/NaN.
    assert_eq!(n.x, 0.000001);
}

#[test]
fn test_mat4_look_at_parallel() {
    let eye = Vec3::new(0.0, 0.0, 0.0);
    let target = Vec3::new(0.0, 0.0, -1.0);
    // Up vector parallel to view direction (0, 0, -1)
    let up = Vec3::new(0.0, 0.0, -1.0);

    // This checks that it doesn't panic
    let m = Mat4::look_at(eye, target, up);

    // The result will be degenerate (likely zeros) but should be valid floats
    assert!(!m.m[0][0].is_nan());
}

#[test]
fn test_mat4_perspective_edge() {
    use std::f32::consts::PI;
    // Near = Far, should produce infinity/NaN in projection matrix
    // because of division by (near - far)
    let proj = Mat4::perspective(PI / 2.0, 1.0, 10.0, 10.0);

    // Verify it doesn't crash, even if values are non-finite
    assert!(proj.m[2][2].is_infinite() || proj.m[2][2].is_nan());
}
