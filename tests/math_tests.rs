use abrash::math::{Mat4, Vec3};

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
fn test_mat4_default() {
    let m = Mat4::default();
    // Identity matrix check
    assert_eq!(m.m[0][0], 1.0);
    assert_eq!(m.m[1][1], 1.0);
    assert_eq!(m.m[2][2], 1.0);
    assert_eq!(m.m[3][3], 1.0);
    assert_eq!(m.m[0][1], 0.0); // Check a non-diagonal element
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
fn test_mat4_mul_operator() {
    let t = Mat4::translation(1.0, 2.0, 3.0);
    let s = Mat4::scale(2.0, 2.0, 2.0);

    // Scale then Translate: v * S * T
    let result = s * t;

    let v = Vec3::new(1.0, 1.0, 1.0);
    let (transformed, _) = result.transform_point(v);

    // Scale first (1,1,1) -> (2,2,2), then translate -> (3,4,5)
    assert_eq!(transformed.x, 3.0);
    assert_eq!(transformed.y, 4.0);
    assert_eq!(transformed.z, 5.0);
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
