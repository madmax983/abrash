#![allow(clippy::float_cmp)]
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
    #[allow(clippy::imprecise_flops)]
    let original_length = (v.x * v.x + v.y * v.y).sqrt();

    // Test multiple rotation angles
    for angle in [0.0, PI / 4.0, PI / 2.0, PI, 2.0 * PI] {
        let mat = Mat2::rotation(angle);
        let rotated = mat.transform(v);
        #[allow(clippy::imprecise_flops)]
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
fn test_vec3_mul_element_wise() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    let c = a * b;
    assert_eq!(c, Vec3::new(4.0, 10.0, 18.0));
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
fn test_mat4_inverse() {
    let m = Mat4::rotation_y(0.5) * Mat4::translation(1.0, 2.0, 3.0) * Mat4::scale(2.0, 3.0, 4.0);
    let inv = m.inverse();
    let ident = m * inv;

    // Check if m * m^-1 == Identity
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!(
                (ident.m[i][j] - expected).abs() < 1e-4,
                "Mismatch at [{}][{}]: {} != {}",
                i,
                j,
                ident.m[i][j],
                expected
            );
        }
    }
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
    let v = Vec3::new(0.000_001, 0.0, 0.0);
    let n = v.normalize();
    // Specific behavior of this math lib:
    // If length < 0.0001, it returns the vector itself instead of normalizing or returning zero/NaN.
    assert_eq!(n.x, 0.000_001);
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

#[test]
fn test_project_triangle_simd_precision() {
    use abrash::math::{Vec3, project_to_screen_optimized, project_triangle_to_screen};

    let half_width = 1000.0;
    let half_height = 500.0;

    // Random-ish coordinates
    let v0 = Vec3::new(1.23, -4.56, 10.0);
    let w0 = 10.0;
    let v1 = Vec3::new(-7.89, 0.12, 5.0);
    let w1 = 5.0;
    let v2 = Vec3::new(0.0, 0.0, 100.0);
    let w2 = 100.0;

    // SIMD
    let (s0, s1, s2) = project_triangle_to_screen(v0, w0, v1, w1, v2, w2, half_width, half_height);

    // Scalar (reference)
    let r0 = project_to_screen_optimized(v0, w0, half_width, half_height);
    let r1 = project_to_screen_optimized(v1, w1, half_width, half_height);
    let r2 = project_to_screen_optimized(v2, w2, half_width, half_height);

    // Check precision (integers should match exactly if float diff is small enough, or be off by 1)
    // Inv_w is float.

    // Allow 1e-5 error for inv_w (rcp approximation)
    assert!(
        (s0.inv_w - r0.inv_w).abs() < 1e-5,
        "v0 inv_w mismatch: {} vs {}",
        s0.inv_w,
        r0.inv_w
    );
    assert!(
        (s1.inv_w - r1.inv_w).abs() < 1e-5,
        "v1 inv_w mismatch: {} vs {}",
        s1.inv_w,
        r1.inv_w
    );
    assert!(
        (s2.inv_w - r2.inv_w).abs() < 1e-5,
        "v2 inv_w mismatch: {} vs {}",
        s2.inv_w,
        r2.inv_w
    );

    // Coordinates
    // Since coordinates are large integers, 1e-5 difference in float might flip the integer.
    // e.g. 100.499999 vs 100.500001 -> 100 vs 101?
    // Truncation happens.
    // We allow +/- 1 pixel diff.
    assert!(
        (s0.x - r0.x).abs() <= 1,
        "v0 x mismatch: {} vs {}",
        s0.x,
        r0.x
    );
    assert!(
        (s0.y - r0.y).abs() <= 1,
        "v0 y mismatch: {} vs {}",
        s0.y,
        r0.y
    );
    assert!(
        (s1.x - r1.x).abs() <= 1,
        "v1 x mismatch: {} vs {}",
        s1.x,
        r1.x
    );
    assert!(
        (s1.y - r1.y).abs() <= 1,
        "v1 y mismatch: {} vs {}",
        s1.y,
        r1.y
    );
    assert!(
        (s2.x - r2.x).abs() <= 1,
        "v2 x mismatch: {} vs {}",
        s2.x,
        r2.x
    );
    assert!(
        (s2.y - r2.y).abs() <= 1,
        "v2 y mismatch: {} vs {}",
        s2.y,
        r2.y
    );
}
