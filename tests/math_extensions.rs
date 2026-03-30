#[cfg(test)]
mod tests {
    use abrash::math::{Mat4, Vec2, Vec3};
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_vec3_additions() {
        let v1 = Vec3::new(0.0, 0.0, 0.0);
        let v2 = Vec3::new(10.0, 10.0, 10.0);

        // Test lerp
        let v_lerp = v1.lerp(v2, 0.5);
        assert_eq!(v_lerp, Vec3::new(5.0, 5.0, 5.0));

        // Test length_sq
        let v3 = Vec3::new(1.0, 2.0, 3.0);
        assert!((v3.length_sq() - 14.0).abs() < f32::EPSILON);

        // Test Div<f32>
        let v4 = Vec3::new(10.0, 20.0, 30.0);
        let v_div = v4 / 2.0;
        assert_eq!(v_div, Vec3::new(5.0, 10.0, 15.0));
    }

    #[test]
    fn test_vec2_new_primitives() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length_sq() - 25.0).abs() < f32::EPSILON);
        assert!((v.length() - 5.0).abs() < 1e-6);

        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.001);

        let p = Vec2::new(2.0, 1.0).perp();
        assert_eq!(p, Vec2::new(-1.0, 2.0));

        let clamped = Vec2::new(5.0, -5.0).clamp(Vec2::new(0.0, 0.0), Vec2::new(4.0, 1.0));
        assert_eq!(clamped, Vec2::new(4.0, 0.0));
    }

    #[test]
    fn test_vec2_projection_cross_angle() {
        let v = Vec2::new(3.0, 4.0);
        let x_axis = Vec2::new(1.0, 0.0);

        let proj = v.project_onto(x_axis);
        let rej = v.reject_from(x_axis);
        assert_eq!(proj, Vec2::new(3.0, 0.0));
        assert_eq!(rej, Vec2::new(0.0, 4.0));

        let cross = Vec2::new(1.0, 0.0).cross(Vec2::new(0.0, 1.0));
        assert!((cross - 1.0).abs() < f32::EPSILON);

        let angle = x_axis.angle_between(Vec2::new(0.0, 1.0));
        assert!((angle - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn test_vec3_projection_and_angle() {
        let v = Vec3::new(2.0, 2.0, 0.0);
        let x_axis = Vec3::new(1.0, 0.0, 0.0);

        let proj = v.project_onto(x_axis);
        let rej = v.reject_from(x_axis);
        assert_eq!(proj, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(rej, Vec3::new(0.0, 2.0, 0.0));

        let angle = x_axis.angle_between(Vec3::new(0.0, 1.0, 0.0));
        assert!((angle - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn test_vec3_distance_and_clamp_abs() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 3.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-5);
        assert!((a.distance_sq(b) - 25.0).abs() < 1e-5);

        let abs = Vec3::new(-1.0, 2.0, -3.0).abs();
        assert_eq!(abs, Vec3::new(1.0, 2.0, 3.0));

        let clamped =
            Vec3::new(5.0, -3.0, 0.5).clamp(Vec3::new(0.0, -1.0, 1.0), Vec3::new(4.0, 3.0, 2.0));
        assert_eq!(clamped, Vec3::new(4.0, -1.0, 1.0));
    }

    #[test]
    fn test_mat4_transpose_determinant_and_affine_inverse() {
        let m =
            Mat4::rotation_z(0.3) * Mat4::translation(2.0, -1.0, 0.5) * Mat4::scale(2.0, 3.0, 4.0);

        let mt = m.transpose();
        assert!((mt.m[1][0] - m.m[0][1]).abs() < f32::EPSILON);
        assert!((mt.m[3][2] - m.m[2][3]).abs() < f32::EPSILON);

        let det = m.determinant();
        assert!((det - 24.0).abs() < 1e-3);

        let inv = m.inverse_affine();
        let ident = m * inv;
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((ident.m[i][j] - expected).abs() < 1e-3);
            }
        }
    }
}
