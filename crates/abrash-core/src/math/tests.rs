#[allow(clippy::wildcard_imports)]
use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_reflect() {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let n = Vec3::new(0.0, 1.0, 0.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < f32::EPSILON);
        assert!((r.y - 1.0).abs() < f32::EPSILON);
        assert!((r.z - 0.0).abs() < f32::EPSILON);

        let v2 = Vec3::new(1.0, 2.0, 3.0);
        let n2 = Vec3::new(0.0, 1.0, 0.0);
        let r2 = v2.reflect(n2);
        assert!((r2.x - 1.0).abs() < f32::EPSILON);
        assert!((r2.y - -2.0).abs() < f32::EPSILON);
        assert!((r2.z - 3.0).abs() < f32::EPSILON);
    }

    // ── Vec2 component ops ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_abs() {
        let v = Vec2::new(-2.0, 3.0).abs();
        assert_eq!(v, Vec2::new(2.0, 3.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_sign() {
        let v = Vec2::new(-5.0, 0.0).sign();
        assert_eq!(v.x, -1.0);
        // f32::signum(0.0) = 1.0 in Rust
        assert!((v.y - 0.0_f32.signum()).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_floor_ceil_round() {
        let v = Vec2::new(1.6, -1.6);
        assert_eq!(v.floor(), Vec2::new(1.0, -2.0));
        assert_eq!(v.ceil(), Vec2::new(2.0, -1.0));
        assert_eq!(v.round(), Vec2::new(2.0, -2.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_fract() {
        let f = Vec2::new(2.75, -1.25).fract();
        assert!((f.x - 0.75).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_step() {
        let edge = Vec2::new(1.0, 2.0);
        let v = Vec2::new(0.5, 3.0);
        let s = v.step(edge);
        assert_eq!(s, Vec2::new(0.0, 1.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_reflect() {
        let v = Vec2::new(1.0, -1.0);
        let n = Vec2::new(0.0, 1.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 1.0).abs() < 1e-6);
    }

    // ── Vec3 component ops ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_sign() {
        let v = Vec3::new(-3.0, 0.5, 0.0).sign();
        assert_eq!(v.x, -1.0);
        assert_eq!(v.y, 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_floor_ceil_round_fract() {
        let v = Vec3::new(1.7, -1.3, 2.5);
        assert_eq!(v.floor(), Vec3::new(1.0, -2.0, 2.0));
        assert_eq!(v.ceil(), Vec3::new(2.0, -1.0, 3.0));
        assert!((v.fract().x - 0.7).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_step() {
        let e = Vec3::new(1.0, 2.0, 3.0);
        let v = Vec3::new(0.5, 2.0, 5.0);
        let s = v.step(e);
        assert_eq!(s, Vec3::new(0.0, 1.0, 1.0));
    }

    // ── Vec4 component ops ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec4_abs_sign() {
        let v = Vec4::new(-1.0, 2.0, -3.0, 0.0);
        let a = v.abs();
        assert_eq!(a, Vec4::new(1.0, 2.0, 3.0, 0.0));
        let s = v.sign();
        assert_eq!(s.x, -1.0);
        assert_eq!(s.y, 1.0);
        assert_eq!(s.z, -1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec4_floor_fract_roundtrip() {
        let v = Vec4::new(3.7, -0.3, 1.5, 2.9);
        let f = v.floor();
        let frac = v.fract();
        assert!((f.x + frac.x - v.x).abs() < 1e-5);
        assert!((f.y + frac.y - v.y).abs() < 1e-5);
    }

    // ── Spline tests ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_quadratic_endpoints() {
        let p = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 0.0);
        assert!(p.length() < 1e-5);
        let p1 = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 1.0);
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_cubic_endpoints() {
        let a = Vec3::ZERO;
        let d = Vec3::new(3.0, 0.0, 0.0);
        let p0 = bezier_cubic(a, Vec3::X, Vec3::new(2.0, 1.0, 0.0), d, 0.0);
        assert!(p0.length() < 1e-5);
        let p1 = bezier_cubic(a, Vec3::X, Vec3::new(2.0, 1.0, 0.0), d, 1.0);
        assert!((p1 - d).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_cubic_tangent_endpoints() {
        // At t=0 tangent should be 3*(p1-p0)
        let p0 = Vec3::ZERO;
        let p1 = Vec3::X;
        let p2 = Vec3::new(2.0, 0.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);
        let tang = bezier_cubic_tangent(p0, p1, p2, p3, 0.0);
        assert!((tang - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn catmull_rom_endpoints() {
        // Passes through p1 at t=0 and p2 at t=1
        let p = catmull_rom(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
            0.0,
        );
        assert!(p.length() < 1e-5);
        let p1 = catmull_rom(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
            1.0,
        );
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hermite_endpoints() {
        let p = hermite(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::X, 0.0);
        assert!(p.length() < 1e-5);
        let p1 = hermite(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::X, 1.0);
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hermite_zero_tangent_midpoint() {
        // With zero tangents, midpoint should be at 0.5 on each axis (symmetric)
        let p = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 0.5);
        assert!((p.x - 0.5).abs() < 1e-5);
    }

    // ── Mat4::unproject test ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn unproject_ortho_center() {
        // Orthographic proj centered at origin — screen center should unproject along -Z
        let proj = Mat4::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
        let dir = Mat4::unproject(400.0, 300.0, 800, 600, &proj);
        // Ortho ray is axis-aligned; x and y should be near 0 at screen center
        assert!(dir.x.abs() < 1e-3, "expected x≈0, got {}", dir.x);
        assert!(dir.y.abs() < 1e-3, "expected y≈0, got {}", dir.y);
    }

    // ── Mat4::row ─────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mat4_row_identity() {
        let m = Mat4::identity();
        assert_eq!(m.row(0), Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(m.row(1), Vec4::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(m.row(3), Vec4::new(0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mat4_row_translation() {
        let m = Mat4::translation(5.0, -3.0, 7.0);
        // Translation is in row 3 in row-vector convention
        let r3 = m.row(3);
        assert!((r3.x - 5.0).abs() < 1e-5);
        assert!((r3.y - (-3.0)).abs() < 1e-5);
        assert!((r3.z - 7.0).abs() < 1e-5);
    }

    // ── Vec smoothstep ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_smoothstep_endpoints() {
        let e0 = Vec2::ZERO;
        let e1 = Vec2::ONE;
        assert_eq!(Vec2::ZERO.smoothstep(e0, e1), Vec2::ZERO);
        assert_eq!(Vec2::ONE.smoothstep(e0, e1), Vec2::ONE);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_smoothstep_midpoint() {
        let v = Vec2::splat(0.5).smoothstep(Vec2::ZERO, Vec2::ONE);
        assert!((v.x - 0.5).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_smoothstep_endpoints() {
        let e0 = Vec3::ZERO;
        let e1 = Vec3::ONE;
        let at_zero = Vec3::ZERO.smoothstep(e0, e1);
        let at_one = Vec3::ONE.smoothstep(e0, e1);
        assert!(at_zero.length() < 1e-5);
        assert!((at_one - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_fast_normalize_accuracy() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let n1 = v.normalize();
        let n2 = v.fast_normalize();

        let diff = n1 - n2;
        assert!(diff.x.abs() < 0.001);
        assert!(diff.y.abs() < 0.001);
        assert!(diff.z.abs() < 0.001);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_transform_points_parallel_threshold() {
        // Test parallel implementation properly falls back and maintains correctness
        let points = vec![Vec3::new(1.0, 2.0, 3.0); 100];
        let mut output = vec![(Vec3::default(), 0.0); 100];
        let m = Mat4::translation(5.0, 5.0, 5.0);

        m.transform_points_parallel(&points, &mut output);

        for (p, _) in output {
            assert!((p.x - 6.0).abs() < 0.001);
            assert!((p.y - 7.0).abs() < 0.001);
            assert!((p.z - 8.0).abs() < 0.001);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_transform_point_simd_vs_scalar() {
        // Scalar implementation reference
        fn transform_point_scalar(m: &Mat4, v: Vec3) -> (Vec3, f32) {
            let x = m.m[0][0] * v.x + m.m[1][0] * v.y + m.m[2][0] * v.z + m.m[3][0];
            let y = m.m[0][1] * v.x + m.m[1][1] * v.y + m.m[2][1] * v.z + m.m[3][1];
            let z = m.m[0][2] * v.x + m.m[1][2] * v.y + m.m[2][2] * v.z + m.m[3][2];
            let w = m.m[0][3] * v.x + m.m[1][3] * v.y + m.m[2][3] * v.z + m.m[3][3];
            (Vec3::new(x, y, z), w)
        }

        let m = Mat4::rotation_y(0.5) * Mat4::translation(10.0, 5.0, 2.0);
        let v = Vec3::new(1.0, 2.0, 3.0);

        // This uses the SIMD implementation because we are compiling with simd feature
        let (simd_p, simd_w) = m.transform_point(v);
        let (scalar_p, scalar_w) = transform_point_scalar(&m, v);

        let diff_p = simd_p - scalar_p;
        assert!(
            diff_p.x.abs() < 0.0001,
            "X mismatch: {} vs {}",
            simd_p.x,
            scalar_p.x
        );
        assert!(
            diff_p.y.abs() < 0.0001,
            "Y mismatch: {} vs {}",
            simd_p.y,
            scalar_p.y
        );
        assert!(
            diff_p.z.abs() < 0.0001,
            "Z mismatch: {} vs {}",
            simd_p.z,
            scalar_p.z
        );
        assert!(
            (simd_w - scalar_w).abs() < 0.0001,
            "W mismatch: {simd_w} vs {scalar_w}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_perspective_projection() {
        use std::f32::consts::PI;
        let fov = PI / 2.0; // 90 degrees
        let aspect = 1.0;
        let near = 1.0;
        let far = 10.0;
        let proj = Mat4::perspective(fov, aspect, near, far);

        // Point on near plane (0, 0, -1) -> should map to w=1, z/w = -1 (OpenGL style: -1 to 1)
        // Wait, standard GL perspective maps -near to -1 and -far to 1 (or 0 to 1 depending on depth range).
        // Let's check the implementation:
        // [0][0] = f / aspect
        // [2][2] = (far + near) / (near - far) (This is typically negative)
        // [2][3] = -1.0
        // [3][2] = 2 * far * near / (near - far)
        //
        // p = (0, 0, -near)
        // x' = 0
        // y' = 0
        // z' = p.z * m[2][2] + m[3][2]
        // w' = p.z * m[2][3] + m[3][3] = -p.z = near
        //
        // z_ndc = z' / w'
        // Let's verify with actual values.

        let p_near = Vec3::new(0.0, 0.0, -near);
        let (p_near_prime, w_near) = proj.transform_point(p_near);

        assert!(
            (w_near - near).abs() < 1e-5,
            "w at near plane should be near"
        );
        // In standard GL, z_ndc at near is -1.0
        let z_ndc_near = p_near_prime.z / w_near;
        assert!(
            (z_ndc_near - (-1.0)).abs() < 1e-5,
            "NDZ z at near should be -1.0, got {z_ndc_near}"
        );

        let p_far = Vec3::new(0.0, 0.0, -far);
        let (p_far_prime, w_far) = proj.transform_point(p_far);
        assert!((w_far - far).abs() < 1e-5, "w at far plane should be far");
        // In standard GL, z_ndc at far is 1.0
        let z_ndc_far = p_far_prime.z / w_far;
        assert!(
            (z_ndc_far - 1.0).abs() < 1e-5,
            "NDC z at far should be 1.0, got {z_ndc_far}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_look_at() {
        let eye = Vec3::new(0.0, 0.0, 10.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        // Point at target (world origin) should map to (0, 0, -10) in camera space
        // because camera is at (0, 0, 10) looking at origin, so origin is 10 units in front (negative Z)
        let p = Vec3::new(0.0, 0.0, 0.0);
        let (p_view, _) = view.transform_point(p);

        // Relaxed tolerance due to fast_inv_sqrt usage in look_at normalization
        let epsilon = 1e-3;
        assert!((p_view.x - 0.0).abs() < epsilon, "X mismatch: {}", p_view.x);
        assert!((p_view.y - 0.0).abs() < epsilon, "Y mismatch: {}", p_view.y);
        assert!(
            (p_view.z - (-10.0)).abs() < epsilon,
            "Z mismatch: {}",
            p_view.z
        );

        // Point at eye should map to (0, 0, 0)
        let (p_eye, _) = view.transform_point(eye);
        assert!(p_eye.length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    #[allow(clippy::float_cmp)]
    fn test_project_to_screen_optimized_edge_cases() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test w = 0 (singular)
        // Code falls back to 1.0 if w.abs() <= 0.0001
        let p = Vec3::new(100.0, 100.0, 10.0);
        let sp = project_to_screen_optimized(p, 0.0, half_width, half_height);

        // Expected behavior: inv_w = 1.0, so x = 100.0, y = 100.0
        // ndc_x = 100.0. screen_x = (100+1)*400 = 40400.
        assert!((sp.inv_w - 1.0).abs() < f32::EPSILON);
        assert_eq!(sp.x, 40400);

        // Test very small w (but > epsilon)
        // w = 0.0002. inv_w = 5000.
        // x = 1.0. ndc_x = 5000.
        // screen_x = (5000+1)*400 = 2000400.
        let sp_small =
            project_to_screen_optimized(Vec3::new(1.0, 0.0, 0.0), 0.0002, half_width, half_height);
        assert!((sp_small.inv_w - 5000.0).abs() < 1e-1);
        assert_eq!(sp_small.x, 2000400);

        // Test negative w (behind camera)
        // w = -1.0. inv_w = -1.0.
        // x = 1.0. ndc_x = -1.0.
        // screen_x = (-1+1)*400 = 0.
        let sp_neg =
            project_to_screen_optimized(Vec3::new(1.0, 0.0, 0.0), -1.0, half_width, half_height);
        assert!((sp_neg.inv_w - -1.0).abs() < f32::EPSILON);
        assert_eq!(sp_neg.x, 0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_normalize_zero() {
        let v = Vec3::new(0.0, 0.0, 0.0);
        let n = v.normalize();
        assert!((n.x - 0.0).abs() < f32::EPSILON);
        assert!((n.y - 0.0).abs() < f32::EPSILON);
        assert!((n.z - 0.0).abs() < f32::EPSILON);

        let v_small = Vec3::new(1e-5, 0.0, 0.0);
        let n_small = v_small.normalize();
        // Should return original if length < 0.0001
        assert!((n_small.x - 1e-5).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_fast_inv_sqrt_sanity() {
        let x = 4.0;
        let y = fast_inv_sqrt(x);
        // 1/sqrt(4) = 0.5
        assert!((y - 0.5).abs() < 0.01);

        let x = 16.0;
        let y = fast_inv_sqrt(x);
        // 1/sqrt(16) = 0.25
        assert!((y - 0.25).abs() < 0.01);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_project_to_screen_safety() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test Infinity
        let v_inf = Vec3::new(f32::INFINITY, 0.0, 0.0);
        let sp_inf = project_to_screen_optimized(v_inf, 1.0, half_width, half_height);
        // Expect clamping to max/min range
        assert!(sp_inf.x == 2147483520);

        // Test Negative Infinity
        let v_neg_inf = Vec3::new(f32::NEG_INFINITY, 0.0, 0.0);
        let sp_neg_inf = project_to_screen_optimized(v_neg_inf, 1.0, half_width, half_height);
        assert!(sp_neg_inf.x == -2147483520);

        // Test NaN
        let v_nan = Vec3::new(f32::NAN, 0.0, 0.0);
        let sp_nan = project_to_screen_optimized(v_nan, 1.0, half_width, half_height);
        // Expect clamping to MIN/MAX range (NaN maps to MIN in this implementation)
        assert_eq!(sp_nan.x, -2147483520);

        // Test Large Number (overflowing i32 but finite)
        let v_large = Vec3::new(1e30, 0.0, 0.0);
        let sp_large = project_to_screen_optimized(v_large, 1.0, half_width, half_height);
        // Should clamp to 2147483520 (approx i32::MAX)
        assert_eq!(sp_large.x, 2147483520);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat2_rotation() {
        use std::f32::consts::FRAC_PI_2;
        let m = Mat2::rotation(FRAC_PI_2);
        // cos(90) is approx 0, sin(90) is 1
        assert!(m.m[0][0].abs() < 1e-6);
        assert!((m.m[0][1] - (-1.0)).abs() < 1e-6);
        assert!((m.m[1][0] - 1.0).abs() < 1e-6);
        assert!(m.m[1][1].abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat2_transform() {
        use std::f32::consts::FRAC_PI_2;
        let m = Mat2::rotation(FRAC_PI_2);
        let v = Vec2::new(1.0, 0.0);
        let result = m.transform(v);
        // (1, 0) rotated 90 deg -> (0, 1)
        assert!(result.x.abs() < 1e-6);
        assert!((result.y - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat2_transform_batch() {
        use std::f32::consts::PI;
        let m = Mat2::rotation(PI);
        let vertices = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let result = m.transform_batch(&vertices);
        // Rotate 180 degrees -> (-x, -y)
        assert!((result[0].x - (-1.0)).abs() < 1e-6);
        assert!(result[0].y.abs() < 1e-6);
        assert!(result[1].x.abs() < 1e-6);
        assert!((result[1].y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat2_transform_in_place() {
        use std::f32::consts::PI;
        let m = Mat2::rotation(PI);
        let mut vertices = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        m.transform_in_place(&mut vertices);
        // Rotate 180 degrees -> (-x, -y)
        assert!((vertices[0].x - (-1.0)).abs() < 1e-6);
        assert!(vertices[0].y.abs() < 1e-6);
        assert!(vertices[1].x.abs() < 1e-6);
        assert!((vertices[1].y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec2_rotate() {
        let rotated = Vec2::new(1.0, 0.0).rotate(std::f32::consts::FRAC_PI_2);
        assert!(rotated.x.abs() < 0.02);
        assert!((rotated.y - 1.0).abs() < 0.02);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert!((v.x - 1.0).abs() < f32::EPSILON);
        assert!((v.y - 2.0).abs() < f32::EPSILON);
        assert!((v.z - 3.0).abs() < f32::EPSILON);
        assert!((v.w - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_add() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = v1 + v2;
        assert!((result.x - 6.0).abs() < f32::EPSILON);
        assert!((result.y - 8.0).abs() < f32::EPSILON);
        assert!((result.z - 10.0).abs() < f32::EPSILON);
        assert!((result.w - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_sub() {
        let v1 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let v2 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = v1 - v2;
        assert!((result.x - 4.0).abs() < f32::EPSILON);
        assert!((result.y - 4.0).abs() < f32::EPSILON);
        assert!((result.z - 4.0).abs() < f32::EPSILON);
        assert!((result.w - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_reflect() {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let r = v.reflect(normal);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 1.0).abs() < 1e-6);
        assert!(r.z.abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_reflect_normalized_matches_reflect() {
        let v = Vec3::new(0.25, -0.5, 1.2);
        let n = Vec3::new(0.0, 1.0, 0.0);
        let a = v.reflect(n);
        let b = v.reflect_normalized(n);
        assert!((a.x - b.x).abs() < 1e-6);
        assert!((a.y - b.y).abs() < 1e-6);
        assert!((a.z - b.z).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_refract_air_to_glass() {
        // 45-degree incidence from air to glass.
        let incident = Vec3::new(1.0, -1.0, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.0 / 1.5);

        // Should still travel downward, bent toward the normal.
        assert!(refracted.y < 0.0);
        assert!(refracted.length() > 0.99 && refracted.length() < 1.01);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_refract_total_internal_reflection() {
        // Steep angle from dense to sparse medium should TIR.
        let incident = Vec3::new(1.0, -0.1, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.5);
        assert_eq!(refracted, Vec3::ZERO);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_face_forward() {
        let n = Vec3::new(0.0, 1.0, 0.0);
        let i_towards = Vec3::new(0.0, -1.0, 0.0);
        let i_away = Vec3::new(0.0, 1.0, 0.0);
        let nref = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(n.face_forward(i_towards, nref), n);
        assert_eq!(n.face_forward(i_away, nref), n * -1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat4_orthographic() {
        let proj = Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, 0.1, 100.0);

        let p_center = Vec3::new(0.0, 0.0, -50.0);
        let (p_center_prime, w_center) = proj.transform_point(p_center);
        assert!((w_center - 1.0).abs() < 1e-5);
        assert!(p_center_prime.x.abs() < 1e-5);
        assert!(p_center_prime.y.abs() < 1e-5);

        // Orthographic projection preserves W as 1.0
        // -50 in Z should map between -1 and 1 in NDC
        let z_ndc = p_center_prime.z / w_center;
        assert!(z_ndc >= -1.0 && z_ndc <= 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_mul_scalar() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = v * 2.5;
        assert!((result.x - 2.5).abs() < f32::EPSILON);
        assert!((result.y - 5.0).abs() < f32::EPSILON);
        assert!((result.z - 7.5).abs() < f32::EPSILON);
        assert!((result.w - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_dot_length_normalize() {
        let a = Vec4::new(1.0, 2.0, 2.0, 1.0);
        let b = Vec4::new(-1.0, 0.5, 3.0, 2.0);
        assert!((a.dot(b) - 8.0).abs() < 1e-6);
        assert!((a.length_sq() - 10.0).abs() < 1e-6);
        assert!((a.length() - 10.0_f32.sqrt()).abs() < 1e-6);

        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 0.01);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_project_and_reject() {
        let v = Vec4::new(3.0, 4.0, 0.0, 0.0);
        let onto = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let proj = v.project_onto(onto);
        let rej = v.reject_from(onto);

        assert!((proj.x - 3.0).abs() < 1e-6);
        assert!(proj.y.abs() < 1e-6);
        assert!(proj.z.abs() < 1e-6);
        assert!(proj.w.abs() < 1e-6);

        assert!(rej.x.abs() < 1e-6);
        assert!((rej.y - 4.0).abs() < 1e-6);
        assert!(rej.z.abs() < 1e-6);
        assert!(rej.w.abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_min_max_clamp_distance() {
        let a = Vec4::new(-1.0, 3.0, 10.0, 0.5);
        let b = Vec4::new(2.0, 1.0, 7.0, 2.0);

        let min = a.min(b);
        let max = a.max(b);
        assert_eq!(min, Vec4::new(-1.0, 1.0, 7.0, 0.5));
        assert_eq!(max, Vec4::new(2.0, 3.0, 10.0, 2.0));

        let clamped = Vec4::new(3.0, 0.0, 8.0, 1.5).clamp(min, max);
        assert_eq!(clamped, Vec4::new(2.0, 1.0, 8.0, 1.5));

        assert!((a.distance_sq(b) - 24.25).abs() < 1e-6);
        assert!((a.distance(b) - 24.25_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_min_max() {
        let a = Vec3::new(1.0, 5.0, -2.0);
        let b = Vec3::new(3.0, 2.0, -1.0);

        let min = a.min(b);
        assert!((min.x - 1.0).abs() < f32::EPSILON);
        assert!((min.y - 2.0).abs() < f32::EPSILON);
        assert!((min.z - -2.0).abs() < f32::EPSILON);

        let max = a.max(b);
        assert!((max.x - 3.0).abs() < f32::EPSILON);
        assert!((max.y - 5.0).abs() < f32::EPSILON);
        assert!((max.z - -1.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_orthonormal_basis() {
        let n = Vec3::new(0.3, 0.5, 0.8).normalize();
        let (t, b) = n.orthonormal_basis();

        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
        assert!(n.dot(t).abs() < 1e-4);
        assert!(n.dot(b).abs() < 1e-4);
        assert!(t.dot(b).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_orthonormal_basis_degenerate_input() {
        let (t, b) = Vec3::ZERO.orthonormal_basis();
        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
        assert!(t.dot(b).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_slerp_midpoint() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let mid = x.slerp(y, 0.5);
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!((mid.x - inv_sqrt2).abs() < 1e-4);
        assert!((mid.y - inv_sqrt2).abs() < 1e-4);
        assert!(mid.z.abs() < 1e-4);
        assert!((mid.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_project_onto_normalized_matches_regular_projection() {
        let v = Vec3::new(3.0, 4.0, 5.0);
        let unit = Vec3::new(2.0, -1.0, 3.0).normalize();
        let a = v.project_onto(unit);
        let b = v.project_onto_normalized(unit);
        assert!((a.x - b.x).abs() < 1e-5);
        assert!((a.y - b.y).abs() < 1e-5);
        assert!((a.z - b.z).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_clamp_length() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let clamped = v.clamp_length(2.0);
        assert!((clamped.length() - 2.0).abs() < 1e-4);

        let unchanged = v.clamp_length(10.0);
        assert!((unchanged.x - v.x).abs() < 1e-6);
        assert!((unchanged.y - v.y).abs() < 1e-6);
        assert!((unchanged.z - v.z).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_is_finite() {
        assert!(Vec3::new(1.0, -2.0, 3.0).is_finite());
        assert!(!Vec3::new(f32::INFINITY, 0.0, 0.0).is_finite());
        assert!(!Vec3::new(0.0, f32::NAN, 0.0).is_finite());
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat4_rotation_axis_matches_rotation_y() {
        use std::f32::consts::FRAC_PI_2;
        let rot_axis = Mat4::rotation_axis(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let rot_y = Mat4::rotation_y(FRAC_PI_2);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let (a, _) = rot_axis.transform_point(v);
        let (b, _) = rot_y.transform_point(v);
        assert!((a.x - b.x).abs() < 1e-5);
        assert!((a.y - b.y).abs() < 1e-5);
        assert!((a.z - b.z).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat4_transform_vector_ignores_translation() {
        let m = Mat4::rotation_z(1.0) * Mat4::translation(10.0, 20.0, 30.0);
        let v = Vec3::new(2.0, -1.0, 3.0);
        let transformed = m.transform_vector(v);
        let (point_transformed, _) = m.transform_point(v);
        let translated_delta = point_transformed - transformed;
        assert!((translated_delta.x - 10.0).abs() < 1e-4);
        assert!((translated_delta.y - 20.0).abs() < 1e-4);
        assert!((translated_delta.z - 30.0).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_barycentric_roundtrip() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 2.0, 0.0);
        let p = Vec3::new(0.5, 0.75, 0.0);

        let bary = p.barycentric_coordinates(a, b, c).unwrap();
        let reconstructed = Vec3::from_barycentric(a, b, c, bary);

        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-5);
        assert!((reconstructed.x - p.x).abs() < 1e-5);
        assert!((reconstructed.y - p.y).abs() < 1e-5);
        assert!((reconstructed.z - p.z).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_barycentric_degenerate_triangle_returns_none() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 1.0);
        let c = Vec3::new(2.0, 2.0, 2.0);
        let p = Vec3::new(0.2, 0.4, 0.6);
        assert!(p.barycentric_coordinates(a, b, c).is_none());
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat4_transform_vectors_batch_matches_scalar() {
        let m = Mat4::rotation_y(0.37) * Mat4::rotation_x(-0.22) * Mat4::translation(4.0, 5.0, 6.0);
        let input = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 0.5, 0.0),
            Vec3::new(0.0, -3.0, 2.0),
        ];
        let mut output = vec![Vec3::ZERO; input.len()];
        m.transform_vectors(&input, &mut output);

        for (i, v) in input.iter().enumerate() {
            let scalar = m.transform_vector(*v);
            assert!((scalar.x - output[i].x).abs() < 1e-5);
            assert!((scalar.y - output[i].y).abs() < 1e-5);
            assert!((scalar.z - output[i].z).abs() < 1e-5);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat4_transform_points_affine_matches_transform_point() {
        let m =
            Mat4::scale(2.0, 3.0, 4.0) * Mat4::rotation_z(0.5) * Mat4::translation(8.0, -2.0, 1.0);
        let input = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-4.0, 1.5, 0.25),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let mut output = vec![Vec3::ZERO; input.len()];
        m.transform_points_affine(&input, &mut output);

        for (i, p) in input.iter().enumerate() {
            let scalar = m.transform_point(*p).0;
            assert!((scalar.x - output[i].x).abs() < 1e-5);
            assert!((scalar.y - output[i].y).abs() < 1e-5);
            assert!((scalar.z - output[i].z).abs() < 1e-5);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    #[cfg(target_arch = "x86_64")]
    fn test_project_to_screen_simd_consistency() {
        let half_width = 400.0;
        let half_height = 300.0;

        let test_cases = vec![
            (Vec3::new(100.0, 100.0, 10.0), 1.0, "Normal"),
            (Vec3::new(0.0, 0.0, 0.0), 1.0, "Origin"),
            (Vec3::new(1.0, 1.0, 1.0), 0.0000001, "Small w (epsilon)"),
            (Vec3::new(1.0, 1.0, 1.0), 0.0, "Zero w"),
            (Vec3::new(1.0, 1.0, 1.0), -1.0, "Negative w"),
            (Vec3::new(f32::INFINITY, 0.0, 0.0), 1.0, "Inf X"),
            (Vec3::new(f32::NAN, 0.0, 0.0), 1.0, "NaN X"),
            (Vec3::new(1e30, 0.0, 0.0), 1.0, "Large X"),
            (Vec3::new(-1e30, 0.0, 0.0), 1.0, "Large Negative X"),
            (Vec3::new(0.0, 0.0, 0.0), f32::INFINITY, "Inf W"),
        ];

        for (v, w, name) in test_cases {
            // Scalar
            let s_scalar = project_to_screen_optimized(v, w, half_width, half_height);

            // SIMD (Triangle)
            let (s_tri_0, _, _) =
                project_triangle_to_screen(v, w, v, w, v, w, half_width, half_height);

            // Verify X and Y (allow off-by-one due to float precision + truncation)
            assert!(
                (i64::from(s_scalar.x) - i64::from(s_tri_0.x)).abs() <= 1,
                "X mismatch for case {}: {} vs {}",
                name,
                s_scalar.x,
                s_tri_0.x
            );
            assert!(
                (i64::from(s_scalar.y) - i64::from(s_tri_0.y)).abs() <= 1,
                "Y mismatch for case {}: {} vs {}",
                name,
                s_scalar.y,
                s_tri_0.y
            );

            // Check z and inv_w with some tolerance
            let z_diff = (s_scalar.z - s_tri_0.z).abs();
            let inv_w_diff = (s_scalar.inv_w - s_tri_0.inv_w).abs();

            let tolerance = if w.abs() > 1e-4 {
                0.002 // Approximation error
            } else {
                1.0 // Loose tolerance for fallback/singularities
            };

            if s_scalar.z.is_nan() {
                assert!(s_tri_0.z.is_nan(), "Z NaN mismatch for case: {name}");
            } else {
                assert!(
                    z_diff < tolerance || (s_scalar.z.is_infinite() && s_tri_0.z.is_infinite()),
                    "Z mismatch for {}: {} vs {} (diff: {})",
                    name,
                    s_scalar.z,
                    s_tri_0.z,
                    z_diff
                );
            }

            if s_scalar.inv_w.is_nan() {
                assert!(s_tri_0.inv_w.is_nan(), "InvW NaN mismatch for case: {name}");
            } else {
                assert!(
                    inv_w_diff < tolerance
                        || (s_scalar.inv_w.is_infinite() && s_tri_0.inv_w.is_infinite()),
                    "InvW mismatch for {}: {} vs {} (diff: {})",
                    name,
                    s_scalar.inv_w,
                    s_tri_0.inv_w,
                    inv_w_diff
                );
            }
        }
    }
}

#[cfg(test)]
mod tests_scalar_utils {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(-5.0, 5.0, 0.5), 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_saturate() {
        assert_eq!(saturate(-1.0), 0.0);
        assert_eq!(saturate(2.0), 1.0);
        assert_eq!(saturate(0.5), 0.5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_smoothstep_edges() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
        // Below edge0
        assert_eq!(smoothstep(2.0, 4.0, 1.0), 0.0);
        // Above edge1
        assert_eq!(smoothstep(2.0, 4.0, 5.0), 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_smootherstep_edges() {
        assert_eq!(smootherstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smootherstep(0.0, 1.0, 1.0), 1.0);
        // Smoother step is also symmetric
        assert!((smootherstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_remap() {
        assert_eq!(remap(5.0, 0.0, 10.0, 0.0, 1.0), 0.5);
        assert_eq!(remap(0.0, 0.0, 10.0, 0.0, 100.0), 0.0);
        assert_eq!(remap(10.0, 0.0, 10.0, 0.0, 100.0), 100.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_fast_atan2_accuracy() {
        for deg in (0..360).step_by(10) {
            let rad = (deg as f32) * PI / 180.0;
            let y = rad.sin();
            let x = rad.cos();
            let expected = y.atan2(x);
            let got = fast_atan2(y, x);
            assert!(
                (got - expected).abs() < 0.005,
                "atan2 mismatch at {deg}°: got={got} expected={expected}"
            );
        }
    }

    // ── ping_pong / map_range / wrap ─────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ping_pong_basic() {
        assert!((ping_pong(0.0, 1.0) - 0.0).abs() < 1e-5);
        assert!((ping_pong(0.5, 1.0) - 0.5).abs() < 1e-5);
        assert!((ping_pong(1.0, 1.0) - 1.0).abs() < 1e-5);
        assert!((ping_pong(1.5, 1.0) - 0.5).abs() < 1e-5);
        assert!((ping_pong(2.0, 1.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn map_range_basic() {
        assert!((map_range(5.0, 0.0, 10.0, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((map_range(0.0, 0.0, 10.0, -1.0, 1.0) - (-1.0)).abs() < 1e-5);
        assert!((map_range(10.0, 0.0, 10.0, -1.0, 1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn wrap_basic() {
        assert!((wrap(1.5, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((wrap(-0.5, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((wrap(0.3, 0.0, 1.0) - 0.3).abs() < 1e-5);
    }

    // ── Bézier eval ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quadratic_bezier_endpoints() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(0.5, 1.0, 0.0);
        let p2 = Vec3::new(1.0, 0.0, 0.0);
        let at0 = quadratic_bezier_eval(p0, p1, p2, 0.0);
        let at1 = quadratic_bezier_eval(p0, p1, p2, 1.0);
        assert!((at0 - p0).length() < 1e-5);
        assert!((at1 - p2).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cubic_bezier_endpoints() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(0.0, 1.0, 0.0);
        let p2 = Vec3::new(1.0, 1.0, 0.0);
        let p3 = Vec3::new(1.0, 0.0, 0.0);
        let at0 = cubic_bezier_eval(p0, p1, p2, p3, 0.0);
        let at1 = cubic_bezier_eval(p0, p1, p2, p3, 1.0);
        assert!((at0 - p0).length() < 1e-5);
        assert!((at1 - p3).length() < 1e-5);
    }

    // ── Vec2 polar / angle_to ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_from_to_polar_roundtrip() {
        let orig = Vec2::new(3.0, 4.0);
        let (r, theta) = orig.to_polar();
        let back = Vec2::from_polar(r, theta);
        assert!((back.x - orig.x).abs() < 1e-5);
        assert!((back.y - orig.y).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_angle_to_ccw() {
        let right = Vec2::new(1.0, 0.0);
        let up = Vec2::new(0.0, 1.0);
        let a = right.angle_to(up);
        assert!(
            (a - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "expected π/2, got {a}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_angle_to_cw_negative() {
        let up = Vec2::new(0.0, 1.0);
        let right = Vec2::new(1.0, 0.0);
        let a = up.angle_to(right);
        assert!(
            (a + std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "expected -π/2, got {a}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec2_splat_and_axes() {
        assert_eq!(Vec2::splat(3.0), Vec2::new(3.0, 3.0));
        assert_eq!(Vec2::X, Vec2::new(1.0, 0.0));
        assert_eq!(Vec2::Y, Vec2::new(0.0, 1.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec2_from_to_angle() {
        let v = Vec2::from_angle(PI / 4.0);
        let angle = v.to_angle();
        assert!((angle - PI / 4.0).abs() < 0.001);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec3_constants() {
        assert_eq!(Vec3::X, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(Vec3::Y, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(Vec3::Z, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(Vec3::UP, Vec3::Y);
        assert_eq!(Vec3::RIGHT, Vec3::X);
        assert_eq!(Vec3::FORWARD, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(Vec3::splat(5.0), Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_vec4_completions() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(2.0, 0.0, 0.0, 0.0);
        assert_eq!(a.dot(b), 2.0);
        assert!((a.length() - (1.0f32 + 4.0 + 9.0 + 16.0).sqrt()).abs() < 1e-5);
        assert_eq!(a.xyz(), Vec3::new(1.0, 2.0, 3.0));
        let n = Vec4::new(1.0, 0.0, 0.0, 0.0).normalize();
        assert!((n.x - 1.0).abs() < 0.01, "normalize x: {}", n.x);
    }
}

#[cfg(test)]
mod tests_mat3 {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5 && (a.z - b.z).abs() < 1e-5
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_identity_transform() {
        let m = Mat3::identity();
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert!(approx_eq_vec3(m.transform(v), v));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_rotation_y_90() {
        let m = Mat3::rotation_y(FRAC_PI_2);
        // X rotates to -Z in right-handed system
        let v = m.transform(Vec3::X);
        assert!(approx_eq_vec3(v, Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_from_mat4() {
        let m4 = Mat4::rotation_x(FRAC_PI_2);
        let m3 = Mat3::from_mat4(&m4);
        let v = Vec3::new(0.0, 1.0, 0.0);
        let via_mat4 = m4.transform_point(v).0;
        let via_mat3 = m3.transform(v);
        assert!(approx_eq_vec3(via_mat3, via_mat4));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_inverse_identity() {
        let m = Mat3::identity();
        let inv = m.inverse();
        assert!((inv.m[0][0] - 1.0).abs() < 1e-5);
        assert!((inv.m[1][1] - 1.0).abs() < 1e-5);
        assert!((inv.m[2][2] - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_inverse_transpose_matches_normal_transform() {
        // For a rotation matrix, inverse-transpose == original (orthonormal)
        let m = Mat3::rotation_z(0.7);
        let it = m.inverse_transpose();
        let v = Vec3::new(1.0, 0.5, 0.25).normalize();
        let via_m = m.transform(v);
        let via_it = it.transform(v);
        assert!(approx_eq_vec3(via_m, via_it));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_mul() {
        let rx = Mat3::rotation_x(FRAC_PI_2);
        let ry = Mat3::rotation_y(FRAC_PI_2);
        let combined = rx * ry;
        let v = Vec3::X;
        let expected = ry.transform(rx.transform(v));
        let got = combined.transform(v);
        assert!(approx_eq_vec3(got, expected));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_mat3_transpose() {
        let m = Mat3::rotation_y(0.5);
        let mt = m.transpose();
        // For rotation, transpose == inverse
        let v = Vec3::new(0.3, -0.7, 1.2);
        let rotated = m.transform(v);
        let restored = mt.transform(rotated);
        assert!(approx_eq_vec3(restored, v));
    }

    // ── Mat2 completions ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mat2_identity_is_noop() {
        let i = Mat2::identity();
        let v = Vec2::new(3.0, -4.0);
        let out = i.transform(v);
        assert!((out.x - v.x).abs() < 1e-6);
        assert!((out.y - v.y).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mat2_determinant_rotation() {
        let m = Mat2::rotation(1.2);
        assert!((m.determinant() - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mat2_inverse_roundtrip() {
        let m = Mat2::scale(2.0, 3.0);
        let inv = m.inverse();
        let prod = m * inv;
        let id = Mat2::identity();
        for i in 0..2 {
            for j in 0..2 {
                assert!((prod.m[i][j] - id.m[i][j]).abs() < 1e-5);
            }
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mat2_mul_rotations_compose() {
        let a = Mat2::rotation(0.4);
        let b = Mat2::rotation(-0.4);
        let prod = a * b;
        let id = Mat2::identity();
        for i in 0..2 {
            for j in 0..2 {
                assert!((prod.m[i][j] - id.m[i][j]).abs() < 1e-5, "[{i}][{j}]");
            }
        }
    }

    // ── Polynomial solvers ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quadratic_two_roots() {
        let (r, n) = quadratic_solve(1.0, -5.0, 6.0);
        assert_eq!(n, 2);
        assert!((r[0] - 2.0).abs() < 1e-5);
        assert!((r[1] - 3.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quadratic_no_real_roots() {
        let (_, n) = quadratic_solve(1.0, 0.0, 1.0);
        assert_eq!(n, 0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quadratic_double_root() {
        let (r, n) = quadratic_solve(1.0, -4.0, 4.0);
        assert_eq!(n, 1);
        assert!((r[0] - 2.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cubic_three_roots() {
        let (r, n) = cubic_solve(1.0, -6.0, 11.0, -6.0);
        assert_eq!(n, 3);
        assert!((r[0] - 1.0).abs() < 1e-3, "r[0]={}", r[0]);
        assert!((r[1] - 2.0).abs() < 1e-3, "r[1]={}", r[1]);
        assert!((r[2] - 3.0).abs() < 1e-3, "r[2]={}", r[2]);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cubic_one_real_root() {
        let (r, n) = cubic_solve(1.0, 0.0, 0.0, -8.0);
        assert_eq!(n, 1);
        assert!((r[0] - 2.0).abs() < 1e-4, "r[0]={}", r[0]);
    }

    // ── Splines ───────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn kochanek_bartels_zero_params_matches_catmull_rom() {
        let (p0, p1, p2, p3) = (
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
        );
        let tcb = kochanek_bartels(p0, p1, p2, p3, 0.5, 0.0, 0.0, 0.0);
        let cr = catmull_rom(p0, p1, p2, p3, 0.5);
        assert!(
            (tcb.x - cr.x).abs() < 1e-5,
            "x mismatch: tcb={} cr={}",
            tcb.x,
            cr.x
        );
        assert!((tcb.y - cr.y).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_split_midpoint_matches_curve() {
        let (p0, p1, p2, p3) = (
            Vec3::ZERO,
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        );
        let (left, right) = bezier_cubic_split(p0, p1, p2, p3, 0.5);
        let mid = bezier_cubic(p0, p1, p2, p3, 0.5);
        assert!((left[3].x - mid.x).abs() < 1e-5);
        assert!((right[0].x - mid.x).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_split_endpoints_preserved() {
        let (p0, p1, p2, p3) = (
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        );
        let (left, right) = bezier_cubic_split(p0, p1, p2, p3, 0.3);
        assert!((left[0].x - p0.x).abs() < 1e-5);
        assert!((right[3].x - p3.x).abs() < 1e-5);
    }

    // ── Vec3 min/max component ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_min_max_component() {
        let v = Vec3::new(3.0, 1.0, 2.0);
        assert_eq!(v.min_component(), 1.0);
        assert_eq!(v.max_component(), 3.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_min_max_negative() {
        let v = Vec3::new(-1.0, -5.0, -2.0);
        assert_eq!(v.min_component(), -5.0);
        assert_eq!(v.max_component(), -1.0);
    }

    // ── Mat4::normal_matrix ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn normal_matrix_pure_rotation_is_same() {
        let m = Mat4::rotation_y(0.7);
        let nm = m.normal_matrix();
        let upper = Mat3::from_mat4(&m);
        let n = Vec3::new(1.0, 0.0, 0.0).normalize();
        let by_nm = nm.transform(n);
        let by_upper = upper.transform(n);
        assert!(
            (by_nm.x - by_upper.x).abs() < 1e-4,
            "x: {} vs {}",
            by_nm.x,
            by_upper.x
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn normal_matrix_non_uniform_scale_corrects() {
        // With non-uniform scale, normals transformed by model matrix get sheared;
        // normal_matrix corrects this.
        let model = Mat4::scale(2.0, 1.0, 1.0);
        let nm = model.normal_matrix();
        // The face normal (1,0,0) of an X-scaled box should still point in (1,0,0)
        let n = Vec3::new(1.0, 0.0, 0.0);
        let corrected = nm.transform(n).normalize();
        assert!((corrected.x - 1.0).abs() < 0.01, "x={}", corrected.x);
        assert!(corrected.y.abs() < 0.01);
    }

    // ── basis_from_normal ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn basis_from_normal_orthonormal() {
        for n in [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.577, 0.577, 0.577).normalize(),
        ] {
            let (t, b) = basis_from_normal(n);
            assert!(t.dot(n).abs() < 1e-4, "t⊥n failed for {n:?}");
            assert!(b.dot(n).abs() < 1e-4, "b⊥n failed for {n:?}");
            assert!(t.dot(b).abs() < 1e-4, "t⊥b failed for {n:?}");
            assert!((t.length() - 1.0).abs() < 1e-4);
            assert!((b.length() - 1.0).abs() < 1e-4);
        }
    }

    // ── Vec2 min/max_component ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec2_min_max_component() {
        let v = Vec2::new(3.0, -1.0);
        assert_eq!(v.min_component(), -1.0);
        assert_eq!(v.max_component(), 3.0);
        let v = Vec2::new(5.0, 5.0);
        assert_eq!(v.min_component(), 5.0);
        assert_eq!(v.max_component(), 5.0);
    }

    // ── Vec3 pow/exp/log ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec3_pow_exp_log() {
        let v = Vec3::new(4.0, 9.0, 16.0);
        let sq = v.pow(0.5);
        assert!((sq.x - 2.0).abs() < 1e-5);
        assert!((sq.y - 3.0).abs() < 1e-5);
        assert!((sq.z - 4.0).abs() < 1e-5);
        let one = Vec3::ZERO.exp();
        assert!((one.x - 1.0).abs() < 1e-5);
        let e_val = Vec3::new(1.0_f32.exp(), 1.0, 1.0);
        let l = e_val.log();
        assert!((l.x - 1.0).abs() < 1e-5);
    }

    // ── Vec4 min/max_component / pow/exp/log ─────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn vec4_component_ops() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.min_component(), 1.0);
        assert_eq!(v.max_component(), 4.0);
        let sq = v.pow(2.0);
        assert!((sq.x - 1.0).abs() < 1e-5);
        assert!((sq.w - 16.0).abs() < 1e-5);
    }

    // ── lerp_angle / angle_diff ──────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lerp_angle_basic() {
        use std::f32::consts::PI;
        // Lerp from 0 to 90° — unambiguous short path
        let a = lerp_angle(0.0, PI / 2.0, 0.5);
        assert!((a - PI / 4.0).abs() < 1e-5, "expected π/4 got {a}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lerp_angle_wraps() {
        // lerping from 350° to 10° should go through 0° (short path), not 180°
        let deg350 = 350.0_f32.to_radians();
        let deg10 = 10.0_f32.to_radians();
        let mid = lerp_angle(deg350, deg10, 0.5);
        let mid_deg = mid.to_degrees().rem_euclid(360.0);
        assert!(
            mid_deg < 20.0 || mid_deg > 340.0,
            "expected near 0°, got {mid_deg}°"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn angle_diff_sign() {
        use std::f32::consts::PI;
        let d = angle_diff(0.0, PI / 2.0);
        assert!((d - PI / 2.0).abs() < 1e-5);
        let d2 = angle_diff(PI / 2.0, 0.0);
        assert!((d2 + PI / 2.0).abs() < 1e-5);
    }

    // ── bspline_eval ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bspline_convex_hull() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(0.0, 2.0, 0.0);
        let p2 = Vec3::new(2.0, 2.0, 0.0);
        let p3 = Vec3::new(2.0, 0.0, 0.0);
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let p = bspline_eval(p0, p1, p2, p3, t);
            assert!(p.x >= -0.01 && p.x <= 2.01, "x out of hull: {}", p.x);
            assert!(p.y >= -0.01 && p.y <= 2.01, "y out of hull: {}", p.y);
        }
    }

    // ── bezier_cubic_arc_length ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_arc_length_straight_line() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0 / 3.0, 0.0, 0.0);
        let p2 = Vec3::new(2.0 / 3.0, 0.0, 0.0);
        let p3 = Vec3::new(1.0, 0.0, 0.0);
        let len = bezier_cubic_arc_length(p0, p1, p2, p3);
        assert!((len - 1.0).abs() < 1e-4, "straight line, got {len}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bezier_arc_length_positive() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(0.0, 1.0, 0.0);
        let p2 = Vec3::new(1.0, 1.0, 0.0);
        let p3 = Vec3::new(1.0, 0.0, 0.0);
        let len = bezier_cubic_arc_length(p0, p1, p2, p3);
        assert!(len > 0.0);
        // Arc length must be >= straight-line distance
        assert!(len >= (p3 - p0).length());
    }

    // ── spherical / cylindrical coordinates ──────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spherical_round_trip() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let (r, theta, phi) = cartesian_to_spherical(v);
        let v2 = spherical_to_cartesian(r, theta, phi);
        assert!((v2 - v).length() < 1e-5, "round trip: {v2:?} vs {v:?}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spherical_poles() {
        // +Y pole: theta = 0
        let (r, theta, _phi) = cartesian_to_spherical(Vec3::new(0.0, 5.0, 0.0));
        assert!((r - 5.0).abs() < 1e-5);
        assert!(theta.abs() < 1e-5);
        // -Y pole: theta = pi
        let (r2, theta2, _) = cartesian_to_spherical(Vec3::new(0.0, -3.0, 0.0));
        assert!((r2 - 3.0).abs() < 1e-5);
        assert!((theta2 - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spherical_equator() {
        // Point on equator (+X axis): theta = pi/2, phi = 0
        let (r, theta, phi) = cartesian_to_spherical(Vec3::new(2.0, 0.0, 0.0));
        assert!((r - 2.0).abs() < 1e-5);
        assert!((theta - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!(phi.abs() < 1e-5 || (phi - std::f32::consts::TAU).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cylindrical_round_trip() {
        let v = Vec3::new(1.5, 4.0, -2.0);
        let (r, theta, y) = cartesian_to_cylindrical(v);
        let v2 = cylindrical_to_cartesian(r, theta, y);
        assert!((v2 - v).length() < 1e-5, "round trip: {v2:?} vs {v:?}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cylindrical_y_preserved() {
        let v = Vec3::new(3.0, 7.0, 4.0);
        let (r, theta, y) = cartesian_to_cylindrical(v);
        assert!((y - 7.0).abs() < 1e-5);
        assert!((r - 5.0).abs() < 1e-5); // sqrt(9+16)
        let v2 = cylindrical_to_cartesian(r, theta, y);
        assert!((v2.y - 7.0).abs() < 1e-5);
    }

    // ── Mat4::shear ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn shear_x_by_y() {
        let m = Mat4::shear(0.5, 0.0, 0.0, 0.0, 0.0, 0.0);
        let v = Vec3::new(0.0, 2.0, 0.0);
        let (result, _) = m.transform_point(v);
        assert!(
            (result.x - 1.0).abs() < 1e-5,
            "x = 0 + 0.5*2 = 1, got {}",
            result.x
        );
        assert!((result.y - 2.0).abs() < 1e-5);
        assert!((result.z).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn shear_identity_zero_params() {
        let m = Mat4::shear(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let v = Vec3::new(3.0, -1.0, 2.0);
        let (result, _) = m.transform_point(v);
        assert!((result - v).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn shear_y_by_x() {
        let m = Mat4::shear(0.0, 0.0, 2.0, 0.0, 0.0, 0.0); // yx = 2
        let v = Vec3::new(1.0, 0.0, 0.0);
        let (result, _) = m.transform_point(v);
        assert!(
            (result.y - 2.0).abs() < 1e-5,
            "y = 0 + 2*1 = 2, got {}",
            result.y
        );
        assert!((result.x - 1.0).abs() < 1e-5);
    }

    // ── Mat4::reflect_plane ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reflect_through_origin_xz_plane() {
        // Reflect through XZ plane (normal = +Y, point = origin)
        let m = Mat4::reflect_plane(Vec3::Y, Vec3::ZERO);
        let v = Vec3::new(1.0, 3.0, 2.0);
        let (result, _) = m.transform_point(v);
        assert!((result.x - 1.0).abs() < 1e-5);
        assert!((result.y + 3.0).abs() < 1e-5, "y flipped: {}", result.y);
        assert!((result.z - 2.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reflect_idempotent() {
        // Reflecting twice returns original point
        let n = Vec3::new(1.0, 1.0, 0.0).normalize();
        let m = Mat4::reflect_plane(n, Vec3::new(1.0, 0.0, 0.0));
        let v = Vec3::new(2.0, 3.0, 1.0);
        let (once, _) = m.transform_point(v);
        let (twice, _) = m.transform_point(once);
        assert!(
            (twice - v).length() < 1e-4,
            "double reflect = identity: {twice:?}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reflect_point_on_plane_unchanged() {
        // A point on the reflection plane should be unchanged
        let n = Vec3::Y;
        let p_on_plane = Vec3::new(3.0, 0.0, -1.0); // y = 0 plane
        let m = Mat4::reflect_plane(n, Vec3::ZERO);
        let (result, _) = m.transform_point(p_on_plane);
        assert!((result - p_on_plane).length() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_14 {
    use super::*;

    // ── bias ──────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bias_midpoint_at_half() {
        // bias(0.5, 0.5) should equal 0.5 (identity knob)
        let v = bias(0.5, 0.5);
        assert!((v - 0.5).abs() < 1e-5, "bias(0.5,0.5) = {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bias_b_near_one_biases_toward_one() {
        // b close to 1 → small exponent → output > t for t in (0,1)
        let v = bias(0.5, 0.9);
        assert!(
            v > 0.5,
            "bias(0.5,0.9) should be > 0.5 (biased up), got {v}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bias_b_near_zero_biases_toward_zero() {
        // b close to 0 → large exponent → output < t for t in (0,1)
        let v = bias(0.5, 0.1);
        assert!(
            v < 0.5,
            "bias(0.5,0.1) should be < 0.5 (biased down), got {v}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bias_extremes_clamped() {
        // t=0 → 0, t=1 → 1 regardless of b
        assert!((bias(0.0, 0.3) - 0.0).abs() < 1e-6);
        assert!((bias(1.0, 0.7) - 1.0).abs() < 1e-6);
    }

    // ── gain ──────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gain_midpoint_always_half() {
        // gain(0.5, g) = 0.5 for any g
        for g in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let v = gain(0.5, g);
            assert!((v - 0.5).abs() < 1e-5, "gain(0.5,{g}) = {v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gain_symmetric_about_half() {
        // gain(1-t, g) = 1 - gain(t, g)
        let g = 0.4_f32;
        for t in [0.1_f32, 0.25, 0.4, 0.75] {
            let a = gain(t, g);
            let b = gain(1.0 - t, g);
            assert!(
                (a + b - 1.0).abs() < 1e-5,
                "symmetry fail at t={t}: {a}+{b}"
            );
        }
    }

    // ── triangle_wave ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn triangle_wave_peaks() {
        assert!((triangle_wave(0.25) - 0.5).abs() < 1e-6);
        assert!((triangle_wave(0.75) - 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn triangle_wave_extremes() {
        assert!((triangle_wave(0.0) - 0.0).abs() < 1e-6);
        assert!((triangle_wave(0.5) - 1.0).abs() < 1e-6);
        assert!((triangle_wave(1.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn triangle_wave_periodic() {
        // t and t+1 give same result
        for t in [0.1_f32, 0.37, 0.88] {
            let a = triangle_wave(t);
            let b = triangle_wave(t + 1.0);
            assert!((a - b).abs() < 1e-6, "periodic fail at t={t}");
        }
    }

    // ── exp_decay ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exp_decay_at_zero_is_one() {
        assert!((exp_decay(0.0, 5.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exp_decay_decreases_monotonically() {
        let v1 = exp_decay(1.0, 2.0);
        let v2 = exp_decay(2.0, 2.0);
        assert!(v1 > v2, "should decrease: {v1} > {v2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exp_decay_negative_t_clamped() {
        // t<0 treated as t=0 → should return 1.0
        assert!((exp_decay(-1.0, 3.0) - 1.0).abs() < 1e-6);
    }

    // ── ease_in / ease_out / ease_in_out ──────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ease_in_clamps_and_monotone() {
        assert!((ease_in(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_in(0.5) < 0.5); // slow start
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ease_out_clamps_and_monotone() {
        assert!((ease_out(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_out(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_out(0.5) > 0.5); // fast start
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ease_in_out_symmetry() {
        // ease_in_out(1-t) = 1 - ease_in_out(t)
        for t in [0.1_f32, 0.3, 0.4] {
            let a = ease_in_out(t);
            let b = ease_in_out(1.0 - t);
            assert!((a + b - 1.0).abs() < 1e-5, "symmetry at t={t}: a={a} b={b}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ease_in_out_midpoint() {
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6);
    }

    // ── Vec3::project_onto_plane ──────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn project_onto_xz_plane() {
        // Projecting onto XZ plane (normal=Y) removes Y component
        let v = Vec3::new(3.0, 5.0, 2.0);
        let proj = v.project_onto_plane(Vec3::Y);
        assert!((proj.x - 3.0).abs() < 1e-6);
        assert!((proj.y - 0.0).abs() < 1e-6);
        assert!((proj.z - 2.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn project_orthogonal_to_normal() {
        // Result must be perpendicular to the normal
        let v = Vec3::new(1.0, 2.0, 3.0);
        let n = Vec3::new(1.0, 1.0, 0.0).normalize();
        let proj = v.project_onto_plane(n);
        assert!(proj.dot(n).abs() < 1e-5, "not orthogonal: {}", proj.dot(n));
    }

    // ── Vec4::homogenize ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn homogenize_unit_w() {
        let v = Vec4::new(3.0, 6.0, 9.0, 1.0);
        let p = v.homogenize();
        assert!((p.x - 3.0).abs() < 1e-6);
        assert!((p.y - 6.0).abs() < 1e-6);
        assert!((p.z - 9.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn homogenize_divides_by_w() {
        let v = Vec4::new(6.0, 9.0, 12.0, 3.0);
        let p = v.homogenize();
        assert!((p.x - 2.0).abs() < 1e-6, "x={}", p.x);
        assert!((p.y - 3.0).abs() < 1e-6, "y={}", p.y);
        assert!((p.z - 4.0).abs() < 1e-6, "z={}", p.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn homogenize_zero_w_returns_zero() {
        let v = Vec4::new(1.0, 2.0, 3.0, 0.0);
        let p = v.homogenize();
        assert_eq!(p, Vec3::ZERO);
    }
}

#[cfg(test)]
mod tests_pass_15 {
    use super::*;

    // ── inverse_lerp ──────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn inverse_lerp_midpoint() {
        let t = inverse_lerp(0.0, 10.0, 5.0);
        assert!((t - 0.5).abs() < 1e-6, "midpoint: {t}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn inverse_lerp_endpoints() {
        assert!((inverse_lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((inverse_lerp(0.0, 10.0, 10.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn inverse_lerp_degenerate_returns_zero() {
        // a == b → division by zero guard
        let t = inverse_lerp(5.0, 5.0, 5.0);
        assert_eq!(t, 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn inverse_lerp_is_lerp_inverse() {
        // lerp(a, b, inverse_lerp(a, b, v)) == v
        let (a, b, v) = (3.0_f32, 7.0, 5.5);
        let t = inverse_lerp(a, b, v);
        let roundtrip = lerp(a, b, t);
        assert!((roundtrip - v).abs() < 1e-5);
    }

    // ── remap_clamped ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn remap_clamped_midpoint() {
        let v = remap_clamped(5.0, 0.0, 10.0, 0.0, 1.0);
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn remap_clamped_clamps_high() {
        let v = remap_clamped(20.0, 0.0, 10.0, 0.0, 1.0);
        assert!((v - 1.0).abs() < 1e-6, "should clamp to 1: {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn remap_clamped_clamps_low() {
        let v = remap_clamped(-5.0, 0.0, 10.0, 0.0, 1.0);
        assert!((v - 0.0).abs() < 1e-6, "should clamp to 0: {v}");
    }

    // ── step ──────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn step_below_edge() {
        assert_eq!(step(0.5, 0.3), 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn step_at_edge() {
        assert_eq!(step(0.5, 0.5), 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn step_above_edge() {
        assert_eq!(step(0.5, 0.8), 1.0);
    }

    // ── sign_no_zero ──────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sign_no_zero_positive() {
        assert_eq!(sign_no_zero(3.0), 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sign_no_zero_negative() {
        assert_eq!(sign_no_zero(-2.0), -1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sign_no_zero_at_zero() {
        assert_eq!(sign_no_zero(0.0), 1.0, "zero maps to +1");
    }
}

#[cfg(test)]
mod tests_pass_16 {
    use super::*;

    // ── sawtooth_wave ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sawtooth_ramps_0_to_1() {
        assert!((sawtooth_wave(0.0) - 0.0).abs() < 1e-6);
        assert!((sawtooth_wave(0.5) - 0.5).abs() < 1e-6);
        assert!((sawtooth_wave(1.0) - 0.0).abs() < 1e-6); // resets
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sawtooth_periodic() {
        for t in [0.1_f32, 0.37, 0.9] {
            assert!((sawtooth_wave(t) - sawtooth_wave(t + 1.0)).abs() < 1e-6);
        }
    }

    // ── square_wave ───────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn square_wave_duty_half() {
        assert_eq!(square_wave(0.25, 0.5), 1.0); // first half
        assert_eq!(square_wave(0.75, 0.5), 0.0); // second half
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn square_wave_duty_zero_always_off() {
        for t in [0.0_f32, 0.5, 0.99] {
            assert_eq!(square_wave(t, 0.0), 0.0);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn square_wave_duty_one_always_on() {
        for t in [0.0_f32, 0.5, 0.99] {
            assert_eq!(square_wave(t, 1.0), 1.0);
        }
    }

    // ── pulse_wave ────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pulse_wave_inside() {
        // Centre 0.5, width 0.2 → on in [0.4, 0.6]
        assert_eq!(pulse_wave(0.5, 0.5, 0.2), 1.0);
        assert_eq!(pulse_wave(0.45, 0.5, 0.2), 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pulse_wave_outside() {
        assert_eq!(pulse_wave(0.2, 0.5, 0.2), 0.0);
        assert_eq!(pulse_wave(0.8, 0.5, 0.2), 0.0);
    }

    // ── signed_area_2d ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn signed_area_ccw_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let a = signed_area_2d(&sq);
        assert!((a - 1.0).abs() < 1e-6, "CCW unit square area = 1: {a}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn signed_area_cw_square_negative() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];
        let a = signed_area_2d(&sq);
        assert!((a + 1.0).abs() < 1e-6, "CW square area = -1: {a}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn signed_area_degenerate_line() {
        let line = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)];
        assert_eq!(signed_area_2d(&line), 0.0); // < 3 vertices
    }

    // ── polygon_centroid_2d ───────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn centroid_unit_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let c = polygon_centroid_2d(&sq);
        assert!((c.x - 0.5).abs() < 1e-5, "cx: {}", c.x);
        assert!((c.y - 0.5).abs() < 1e-5, "cy: {}", c.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn centroid_right_triangle() {
        // Right triangle (0,0),(3,0),(0,3): centroid at (1,1)
        let tri = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(0.0, 3.0),
        ];
        let c = polygon_centroid_2d(&tri);
        assert!((c.x - 1.0).abs() < 1e-5, "cx: {}", c.x);
        assert!((c.y - 1.0).abs() < 1e-5, "cy: {}", c.y);
    }

    // ── point_in_polygon_2d ───────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn point_in_triangle() {
        let tri = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(1.0, 2.0),
        ];
        assert!(point_in_polygon_2d(Vec2::new(1.0, 0.5), &tri));
        assert!(!point_in_polygon_2d(Vec2::new(3.0, 1.0), &tri));
        assert!(!point_in_polygon_2d(Vec2::new(1.0, 2.5), &tri));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn point_in_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        assert!(point_in_polygon_2d(Vec2::new(1.0, 1.0), &sq));
        assert!(!point_in_polygon_2d(Vec2::new(3.0, 1.0), &sq));
        assert!(!point_in_polygon_2d(Vec2::new(-1.0, 1.0), &sq));
    }

    // ── Vec2::clamp_length ────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn clamp_length_shrinks_long_vector() {
        let v = Vec2::new(3.0, 4.0); // length 5
        let c = v.clamp_length(2.0);
        assert!((c.length() - 2.0).abs() < 1e-5);
        // Direction preserved
        let ratio = c.x / v.x;
        assert!((c.y / v.y - ratio).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn clamp_length_leaves_short_vector() {
        let v = Vec2::new(0.5, 0.0);
        assert_eq!(v.clamp_length(2.0), v);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn clamp_length_at_exact_max() {
        let v = Vec2::new(2.0, 0.0);
        let c = v.clamp_length(2.0);
        assert!((c.length() - 2.0).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_17 {
    use super::*;

    // ── fast_sin_cos ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_fast_sin_cos() {
        let max_error = 0.002_f32;
        let test_angles = [
            0.0_f32,
            std::f32::consts::FRAC_PI_6,
            std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_3,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            std::f32::consts::PI + std::f32::consts::FRAC_PI_4,
            std::f32::consts::TAU - 0.1,
            -std::f32::consts::FRAC_PI_4,
            -std::f32::consts::PI,
            10.0,
            -10.0,
        ];

        for &angle in &test_angles {
            let (expected_sin, expected_cos) = angle.sin_cos();
            let (actual_sin, actual_cos) = fast_sin_cos(angle);
            assert!(
                (actual_sin - expected_sin).abs() <= max_error,
                "sin({angle}) expected {expected_sin}, got {actual_sin}"
            );
            assert!(
                (actual_cos - expected_cos).abs() <= max_error,
                "cos({angle}) expected {expected_cos}, got {actual_cos}"
            );
        }
    }

    // ── halton ────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn halton_base2_known_values() {
        assert!((halton(1, 2) - 0.5).abs() < 1e-7);
        assert!((halton(2, 2) - 0.25).abs() < 1e-7);
        assert!((halton(3, 2) - 0.75).abs() < 1e-7);
        assert!((halton(4, 2) - 0.125).abs() < 1e-7);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn halton_zero_is_zero() {
        assert_eq!(halton(0, 2), 0.0);
        assert_eq!(halton(0, 3), 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn halton_in_unit_interval() {
        for i in 0..64u32 {
            let v = halton(i, 2);
            assert!((0.0..1.0).contains(&v), "halton({i},2) = {v} not in [0,1)");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn halton_base3_known_values() {
        // base-3: 1→1/3, 2→2/3, 3→1/9, 4→4/9
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-7);
        assert!((halton(2, 3) - 2.0 / 3.0).abs() < 1e-7);
        assert!((halton(3, 3) - 1.0 / 9.0).abs() < 1e-7);
    }

    // ── van_der_corput ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn van_der_corput_matches_halton_base2() {
        for i in 0..32u32 {
            let a = van_der_corput(i);
            let b = halton(i, 2);
            assert!((a - b).abs() < 1e-7, "vdc({i}) = {a}, halton = {b}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn van_der_corput_in_unit_interval() {
        for i in 0..64u32 {
            let v = van_der_corput(i);
            assert!((0.0..=1.0).contains(&v));
        }
    }

    // ── hammersley_2d ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hammersley_x_is_i_over_n() {
        let p = hammersley_2d(3, 8);
        assert!((p.x - 3.0 / 8.0).abs() < 1e-7);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hammersley_y_is_vdc() {
        for i in 0..16u32 {
            let p = hammersley_2d(i, 16);
            assert!((p.y - van_der_corput(i)).abs() < 1e-9);
        }
    }

    // ── frenet_frame ──────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn frenet_frame_orthonormal() {
        let (t, n, b) = frenet_frame(Vec3::new(1.0, 0.5, 0.2), Vec3::Y);
        assert!(
            (t.length() - 1.0).abs() < 1e-5,
            "T not unit: {}",
            t.length()
        );
        assert!(
            (n.length() - 1.0).abs() < 1e-5,
            "N not unit: {}",
            n.length()
        );
        assert!(
            (b.length() - 1.0).abs() < 1e-5,
            "B not unit: {}",
            b.length()
        );
        assert!(t.dot(n).abs() < 1e-5, "T·N not zero: {}", t.dot(n));
        assert!(t.dot(b).abs() < 1e-5, "T·B not zero: {}", t.dot(b));
        assert!(n.dot(b).abs() < 1e-5, "N·B not zero: {}", n.dot(b));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn frenet_frame_x_axis_with_y_up() {
        let (t, _n, _b) = frenet_frame(Vec3::X, Vec3::Y);
        assert!((t - Vec3::X).length() < 1e-5, "T should be +X");
    }
}

#[cfg(test)]
mod tests_pass_18 {
    use super::*;

    // ── octahedral_encode / octahedral_decode ─────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn octahedral_round_trip_axes() {
        let axes = [
            Vec3::X,
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::Y,
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::Z,
            Vec3::new(0.0, 0.0, -1.0),
        ];
        for a in axes {
            let enc = octahedral_encode(a);
            let dec = octahedral_decode(enc);
            assert!(
                (dec - a).length() < 1e-5,
                "round-trip failed for {a:?}: got {dec:?}"
            );
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn octahedral_decoded_is_unit_length() {
        // Arbitrary encoded values should always decode to unit vectors
        for &v in &[
            Vec2::new(0.5, 0.3),
            Vec2::new(-0.7, 0.2),
            Vec2::new(0.1, -0.9),
        ] {
            let d = octahedral_decode(v);
            assert!(
                (d.length() - 1.0).abs() < 1e-5,
                "not unit: {d:?}, len {}",
                d.length()
            );
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn octahedral_encode_range() {
        // Encoded values should lie in [-1,1]²
        let normals = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.577, 0.577, 0.577).normalize_or_zero(),
            Vec3::new(-0.5, 0.5, -0.707).normalize_or_zero(),
        ];
        for n in normals {
            let e = octahedral_encode(n);
            assert!(
                e.x.abs() <= 1.001 && e.y.abs() <= 1.001,
                "encoded out of [-1,1]²: {e:?}"
            );
        }
    }

    // ── morton_encode_2d / morton_decode_2d ──────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn morton_encode_known() {
        assert_eq!(morton_encode_2d(0, 0), 0);
        assert_eq!(morton_encode_2d(1, 0), 0b01);
        assert_eq!(morton_encode_2d(0, 1), 0b10);
        assert_eq!(morton_encode_2d(1, 1), 0b11);
        assert_eq!(morton_encode_2d(2, 0), 0b0100);
        assert_eq!(morton_encode_2d(0, 2), 0b1000);
        assert_eq!(morton_encode_2d(3, 3), 0b1111);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn morton_round_trip() {
        for x in 0u32..16 {
            for y in 0u32..16 {
                let code = morton_encode_2d(x, y);
                let (dx, dy) = morton_decode_2d(code);
                assert_eq!((dx, dy), (x, y), "round-trip failed for ({x},{y})");
            }
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn morton_spatial_locality() {
        // Neighbours (0,0)-(1,0)-(0,1)-(1,1) should have codes 0-3
        let c00 = morton_encode_2d(0, 0);
        let c10 = morton_encode_2d(1, 0);
        let c01 = morton_encode_2d(0, 1);
        let c11 = morton_encode_2d(1, 1);
        assert_eq!([c00, c10, c01, c11], [0, 1, 2, 3]);
    }

    // ── spherical_fibonacci ───────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spherical_fibonacci_on_unit_sphere() {
        for i in 0..64u32 {
            let p = spherical_fibonacci(i, 64);
            assert!(
                (p.length() - 1.0).abs() < 1e-5,
                "point {i} not on unit sphere: len {}",
                p.length()
            );
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spherical_fibonacci_poles() {
        let n = 100u32;
        // First point near south pole (cos_phi ≈ -1)
        let south = spherical_fibonacci(0, n);
        assert!(south.z < -0.95, "first point should be near south pole");
        // Last point near north pole (cos_phi ≈ 1)
        let north = spherical_fibonacci(n - 1, n);
        assert!(north.z > 0.95, "last point should be near north pole");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spherical_fibonacci_coverage() {
        // 64 points should cover all octants
        let n = 64u32;
        let mut octants = [false; 8];
        for i in 0..n {
            let p = spherical_fibonacci(i, n);
            let idx = usize::from(p.x > 0.0)
                | (usize::from(p.y > 0.0) << 1)
                | (usize::from(p.z > 0.0) << 2);
            octants[idx] = true;
        }
        assert!(octants.iter().all(|&v| v), "not all octants covered");
    }

    // ── golden_ratio_sequence ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn golden_ratio_in_unit_interval() {
        for i in 0..256u32 {
            let v = golden_ratio_sequence(i);
            assert!(
                v >= 0.0 && v < 1.0,
                "golden_ratio_sequence({i}) = {v} out of [0,1)"
            );
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn golden_ratio_low_discrepancy() {
        // All 8 first values should be distinct (no clustering)
        let vals: Vec<f32> = (0..8).map(golden_ratio_sequence).collect();
        for i in 0..vals.len() {
            for j in (i + 1)..vals.len() {
                assert!(
                    (vals[i] - vals[j]).abs() > 0.05,
                    "golden_ratio values {i} and {j} are too close: {} vs {}",
                    vals[i],
                    vals[j]
                );
            }
        }
    }
}

#[cfg(test)]
mod tests_pass_19 {
    use super::*;

    // ── spring_damper ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spring_converges_to_target() {
        let mut vel = 0.0_f32;
        let mut pos = 0.0_f32;
        let target = 10.0_f32;
        for _ in 0..500 {
            let (p, v) = spring_damper(pos, &mut vel, target, 10.0, 0.016);
            pos = p;
            vel = v;
        }
        assert!(
            (pos - target).abs() < 0.01,
            "spring should converge: pos={pos}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spring_no_overshoot() {
        let mut vel = 0.0_f32;
        let mut pos = 0.0_f32;
        let target = 1.0_f32;
        let mut max_pos = 0.0_f32;
        for _ in 0..200 {
            let (p, v) = spring_damper(pos, &mut vel, target, 8.0, 0.016);
            pos = p;
            vel = v;
            max_pos = max_pos.max(pos);
        }
        // Critically damped: should not overshoot by more than 1%
        assert!(
            max_pos <= 1.01,
            "critically damped spring overshot: {max_pos}"
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spring_stationary_stays_still() {
        let mut vel = 0.0_f32;
        let (p, v) = spring_damper(5.0, &mut vel, 5.0, 10.0, 0.016);
        assert!((p - 5.0).abs() < 1e-5, "at target, should stay: {p}");
        assert!(v.abs() < 1e-5, "velocity should be zero: {v}");
    }

    // ── fast_log2 ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fast_log2_powers_of_two() {
        for i in 0u32..8 {
            let x = (1u32 << i) as f32;
            let expected = i as f32;
            let got = fast_log2(x);
            assert!(
                (got - expected).abs() < 0.01,
                "fast_log2({x}) = {got}, expected {expected}"
            );
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fast_log2_one_half() {
        assert!((fast_log2(0.5) - (-1.0)).abs() < 0.01);
    }

    // ── fast_exp2 ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fast_exp2_integer_inputs() {
        for i in -3i32..=8 {
            let expected = (2.0_f32).powi(i);
            let got = fast_exp2(i as f32);
            let rel = (got - expected).abs() / expected;
            assert!(rel < 0.05, "fast_exp2({i}) = {got}, expected {expected}");
        }
    }

    // ── next_power_of_two / prev_power_of_two ────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn next_pow2_known() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(2), 2);
        assert_eq!(next_power_of_two(3), 4);
        assert_eq!(next_power_of_two(100), 128);
        assert_eq!(next_power_of_two(256), 256);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn prev_pow2_known() {
        assert_eq!(prev_power_of_two(0), 0);
        assert_eq!(prev_power_of_two(1), 1);
        assert_eq!(prev_power_of_two(7), 4);
        assert_eq!(prev_power_of_two(8), 8);
        assert_eq!(prev_power_of_two(255), 128);
    }

    // ── smootherstep5 / smootherstep7 ────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smootherstep5_endpoints_and_midpoint() {
        assert_eq!(smootherstep5(0.0), 0.0);
        assert_eq!(smootherstep5(1.0), 1.0);
        assert!((smootherstep5(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smootherstep5_clamps() {
        assert_eq!(smootherstep5(-1.0), 0.0);
        assert_eq!(smootherstep5(2.0), 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smootherstep7_endpoints_and_midpoint() {
        assert_eq!(smootherstep7(0.0), 0.0);
        assert_eq!(smootherstep7(1.0), 1.0);
        assert!((smootherstep7(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smootherstep5_steeper_than_smoothstep_near_edges() {
        // smootherstep5 has zero second derivative at endpoints → flatter shoulders
        // Check that 0.25 is below the 3rd-order smoothstep
        let s3 = smoothstep(0.0, 1.0, 0.25);
        let s5 = smootherstep5(0.25);
        assert!(
            s5 < s3,
            "smootherstep5 should be flatter near 0: {s5} vs {s3}"
        );
    }

    // ── median3 ───────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn median3_all_permutations() {
        let vals = [1.0_f32, 3.0, 2.0];
        // All 6 orderings of (1, 2, 3) should give median=2
        assert_eq!(median3(vals[0], vals[1], vals[2]), 2.0);
        assert_eq!(median3(vals[0], vals[2], vals[1]), 2.0);
        assert_eq!(median3(vals[1], vals[0], vals[2]), 2.0);
        assert_eq!(median3(vals[1], vals[2], vals[0]), 2.0);
        assert_eq!(median3(vals[2], vals[0], vals[1]), 2.0);
        assert_eq!(median3(vals[2], vals[1], vals[0]), 2.0);
    }

    // ── in_range / approx_eq ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn in_range_inclusive_bounds() {
        assert!(in_range(0.0_f32, 0.0, 1.0));
        assert!(in_range(1.0_f32, 0.0, 1.0));
        assert!(!in_range(1.001_f32, 0.0, 1.0));
        assert!(!in_range(-0.001_f32, 0.0, 1.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn approx_eq_within_eps() {
        assert!(approx_eq(1.0_f32, 1.0 + 1e-6, 1e-5));
        assert!(!approx_eq(1.0_f32, 1.1, 1e-5));
        assert!(approx_eq(-1.0_f32, -1.0, 0.0));
    }
}

#[cfg(test)]
mod tests_pass_20 {
    use super::*;

    // ── pcg_hash ──────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcg_hash_deterministic() {
        assert_eq!(pcg_hash(0), pcg_hash(0));
        assert_eq!(pcg_hash(u32::MAX), pcg_hash(u32::MAX));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcg_hash_avalanche() {
        let a = pcg_hash(0x0000_0001);
        let b = pcg_hash(0x0000_0002);
        let diff = (a ^ b).count_ones();
        assert!(diff >= 8, "poor avalanche: only {diff} bits differ");
    }

    // ── wang_hash ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn wang_hash_deterministic() {
        assert_eq!(wang_hash(42), wang_hash(42));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn wang_hash_distinct_inputs() {
        for i in 0u32..8 {
            for j in (i + 1)..8 {
                assert_ne!(wang_hash(i), wang_hash(j), "collision at {i},{j}");
            }
        }
    }

    // ── hash_to_f32 / hash2_to_f32 ───────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hash_to_f32_in_range() {
        for i in 0u32..256 {
            let v = hash_to_f32(i);
            assert!(v >= 0.0 && v < 1.0, "hash_to_f32({i}) = {v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hash2_to_f32_in_range() {
        for x in 0u32..16 {
            for y in 0u32..16 {
                let v = hash2_to_f32(x, y);
                assert!(v >= 0.0 && v < 1.0, "hash2({x},{y}) = {v}");
            }
        }
    }

    // ── bayer8x8 ──────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bayer8x8_all_in_range() {
        for y in 0..8u32 {
            for x in 0..8u32 {
                let v = bayer8x8(x, y);
                assert!(v >= 0.0 && v < 1.0, "bayer({x},{y}) = {v}");
            }
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bayer8x8_all_unique() {
        let mut seen = std::collections::HashSet::new();
        for y in 0..8u32 {
            for x in 0..8u32 {
                let v = (bayer8x8(x, y) * 64.0).round() as u32;
                assert!(seen.insert(v), "duplicate Bayer value {v} at ({x},{y})");
            }
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bayer8x8_wraps_periodically() {
        assert_eq!(bayer8x8(0, 0), bayer8x8(8, 0));
        assert_eq!(bayer8x8(3, 5), bayer8x8(11, 13));
    }

    // ── linear_to_db / db_to_linear ──────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn db_round_trip() {
        for amp in [0.01_f32, 0.1, 0.5, 1.0, 2.0, 10.0] {
            let db = linear_to_db(amp);
            let back = db_to_linear(db);
            assert!((back - amp).abs() / amp < 1e-5, "round-trip {amp}: {back}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn db_zero_is_minus_infinity() {
        assert!(linear_to_db(0.0).is_infinite() && linear_to_db(0.0) < 0.0);
    }

    // ── snap ──────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn snap_rounds_to_grid() {
        assert!((snap(0.7_f32, 0.25) - 0.75).abs() < 1e-6);
        assert!((snap(0.3_f32, 0.25) - 0.25).abs() < 1e-6);
        assert!((snap(-0.3_f32, 0.25) - -0.25).abs() < 1e-6);
    }

    // ── bounce ────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bounce_stays_in_range() {
        for i in 0..200 {
            let t = i as f32 * 0.13;
            let v = bounce(t, 0.0, 1.0);
            assert!(v >= -1e-5 && v <= 1.0 + 1e-5, "bounce({t}) = {v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bounce_reflects_at_bounds() {
        assert!((bounce(0.0_f32, 0.0, 1.0) - 0.0).abs() < 1e-5);
        assert!((bounce(1.0_f32, 0.0, 1.0) - 1.0).abs() < 1e-5);
        assert!((bounce(1.5_f32, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((bounce(2.0_f32, 0.0, 1.0) - 0.0).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_21 {
    use super::*;

    // ── min3 / max3 ───────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn min3_all_orderings() {
        assert_eq!(min3(1.0_f32, 2.0, 3.0), 1.0);
        assert_eq!(min3(3.0_f32, 1.0, 2.0), 1.0);
        assert_eq!(min3(2.0_f32, 3.0, 1.0), 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn max3_all_orderings() {
        assert_eq!(max3(1.0_f32, 2.0, 3.0), 3.0);
        assert_eq!(max3(3.0_f32, 1.0, 2.0), 3.0);
        assert_eq!(max3(2.0_f32, 3.0, 1.0), 3.0);
    }

    // ── triangle_area_3d ──────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn unit_right_triangle_area() {
        let area = triangle_area_3d(Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!((area - 0.5).abs() < 1e-6, "unit right tri area: {area}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn unit_equilateral_triangle_area() {
        // Equilateral triangle with side 1: area = sqrt(3)/4 ≈ 0.433
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.5, 3.0_f32.sqrt() / 2.0, 0.0);
        let area = triangle_area_3d(Vec3::ZERO, b, c);
        assert!(
            (area - 3.0_f32.sqrt() / 4.0).abs() < 1e-5,
            "equilateral area: {area}"
        );
    }

    // ── signed_volume_tet ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn unit_tet_volume() {
        // Tetrahedron with vertices at (0,0,0),(1,0,0),(0,1,0),(0,0,1)
        // Volume = 1/6
        let v = signed_volume_tet(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z).abs();
        assert!((v - 1.0 / 6.0).abs() < 1e-6, "unit tet volume: {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn tet_volume_sign_encodes_winding() {
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let d = Vec3::Z;
        let v_fwd = signed_volume_tet(a, b, c, d);
        let v_rev = signed_volume_tet(a, c, b, d); // swap b and c → reverse winding
        assert!(v_fwd > 0.0, "CCW: positive");
        assert!(v_rev < 0.0, "CW: negative");
    }

    // ── perspective_correct_lerp ──────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcl_equal_w_is_linear() {
        let v = perspective_correct_lerp(0.5, 0.0, 1.0, 1.0, 1.0);
        assert!((v - 0.5).abs() < 1e-6, "equal w → linear: {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcl_endpoints_exact() {
        assert!((perspective_correct_lerp(0.0, 3.0, 7.0, 0.5, 2.0) - 3.0).abs() < 1e-5);
        assert!((perspective_correct_lerp(1.0, 3.0, 7.0, 0.5, 2.0) - 7.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcl_biases_toward_closer_vertex() {
        // Vertex 1 has larger w (closer to camera) → midpoint biased toward vertex 1's value
        let v_pcl = perspective_correct_lerp(0.5, 0.0, 1.0, 0.1, 2.0);
        let v_lin = 0.5_f32;
        assert!(
            v_pcl > v_lin,
            "PCL biased toward larger-w vertex: pcl={v_pcl}, lin={v_lin}"
        );
    }

    // ── smoothstep_sine ───────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smoothstep_sine_endpoints() {
        assert_eq!(smoothstep_sine(0.0), 0.0);
        assert_eq!(smoothstep_sine(1.0), 1.0);
        assert!((smoothstep_sine(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smoothstep_sine_clamps() {
        assert_eq!(smoothstep_sine(-1.0), 0.0);
        assert_eq!(smoothstep_sine(2.0), 1.0);
    }

    // ── ease_exp_in / ease_exp_out ────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ease_exp_in_endpoints() {
        assert!((ease_exp_in(0.0, 4.0) - 0.0).abs() < 1e-5);
        assert!((ease_exp_in(1.0, 4.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ease_exp_out_is_mirror_of_in() {
        let t = 0.3_f32;
        let out_val = ease_exp_out(t, 4.0);
        let in_val = ease_exp_in(1.0 - t, 4.0);
        assert!((out_val - (1.0 - in_val)).abs() < 1e-5, "out ≈ 1-in(1-t)");
    }
}

#[cfg(test)]
mod tests_pass_22 {
    use super::*;

    // ── reinhard ──────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reinhard_known_values() {
        assert!((reinhard(0.0) - 0.0).abs() < 1e-6);
        assert!((reinhard(1.0) - 0.5).abs() < 1e-6);
        assert!((reinhard(3.0) - 0.75).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reinhard_monotone() {
        let mut prev = reinhard(0.0);
        for i in 1..20u32 {
            let v = reinhard(i as f32 * 0.5);
            assert!(v > prev, "not increasing at {i}");
            prev = v;
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reinhard_white_brighter_than_plain() {
        let plain = reinhard(1.0);
        let white = reinhard_white(1.0, 100.0);
        assert!(white > plain, "{plain} vs {white}");
    }

    // ── aces_filmic ───────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aces_black_and_clamp() {
        assert!(aces_filmic(0.0).abs() < 1e-4);
        assert!(aces_filmic(100.0) <= 1.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aces_output_in_01() {
        for i in 0..50u32 {
            let v = aces_filmic(i as f32 * 0.2);
            assert!(v >= 0.0 && v <= 1.0, "aces({}) = {v}", i as f32 * 0.2);
        }
    }

    // ── exposure ──────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exposure_zero_ev_identity() {
        assert!((exposure(0.7, 0.0) - 0.7).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exposure_one_stop_doubles() {
        assert!((exposure(0.25, 1.0) - 0.5).abs() < 1e-5);
    }

    // ── sphere_normal_to_uv ───────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_uv_poles() {
        let (_, v_n) = sphere_normal_to_uv(Vec3::Y);
        let (_, v_s) = sphere_normal_to_uv(Vec3::new(0.0, -1.0, 0.0));
        assert!((v_n - 1.0).abs() < 1e-5, "north pole: {v_n}");
        assert!(v_s.abs() < 1e-5, "south pole: {v_s}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_uv_in_range() {
        for d in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(0.577, 0.577, 0.577)] {
            let (u, v) = sphere_normal_to_uv(d);
            assert!(
                u >= 0.0 && u <= 1.0 && v >= 0.0 && v <= 1.0,
                "uv out of range for {d:?}"
            );
        }
    }

    // ── make_view_ray ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn view_ray_centre_forward() {
        let dir = make_view_ray(Vec2::new(0.5, 0.5), core::f32::consts::FRAC_PI_2, 1.0);
        assert!(dir.z < 0.0, "centre -Z: {dir:?}");
        assert!(dir.x.abs() < 1e-5 && dir.y.abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn view_ray_unit_length() {
        for uv in [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.3, 0.7),
        ] {
            let d = make_view_ray(uv, 1.0, 1.6);
            assert!((d.length() - 1.0).abs() < 1e-5, "uv={uv:?}: {}", d.length());
        }
    }

    // ── linear_to_srgb / srgb_to_linear ──────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn srgb_round_trip() {
        for i in 0..=10u32 {
            let linear = i as f32 / 10.0;
            let back = srgb_to_linear(linear_to_srgb(linear));
            assert!((back - linear).abs() < 1e-5, "round-trip {linear}: {back}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn srgb_midgrey() {
        // Linear 0.5 maps to ~0.735 in sRGB
        let s = linear_to_srgb(0.5);
        assert!(s > 0.70 && s < 0.76, "mid-grey: {s}");
    }
}

#[cfg(test)]
mod tests_pass_23 {
    use super::*;

    // ── rgb_to_hsv / hsv_to_rgb ────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hsv_primary_colours() {
        // Red
        let (h, s, v) = rgb_to_hsv(1.0, 0.0, 0.0);
        assert!(h < 1.0 || h > 359.0, "red hue ~0: {h}");
        assert!((s - 1.0).abs() < 1e-5 && (v - 1.0).abs() < 1e-5);
        // Green
        let (h, _, _) = rgb_to_hsv(0.0, 1.0, 0.0);
        assert!((h - 120.0).abs() < 1e-3, "green hue: {h}");
        // Blue
        let (h, _, _) = rgb_to_hsv(0.0, 0.0, 1.0);
        assert!((h - 240.0).abs() < 1e-3, "blue hue: {h}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hsv_round_trip() {
        for (r, g, b) in [(0.8, 0.3, 0.1), (0.0, 0.5, 1.0), (0.2, 0.2, 0.2)] {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let (r2, g2, b2) = hsv_to_rgb(h, s, v);
            assert!((r - r2).abs() < 1e-5, "r round-trip: {r} vs {r2}");
            assert!((g - g2).abs() < 1e-5, "g round-trip: {g} vs {g2}");
            assert!((b - b2).abs() < 1e-5, "b round-trip: {b} vs {b2}");
        }
    }

    // ── rgb_to_hsl / hsl_to_rgb ────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hsl_round_trip() {
        for (r, g, b) in [(0.8, 0.3, 0.1), (0.0, 0.5, 1.0), (0.5, 0.5, 0.5)] {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!((r - r2).abs() < 1e-5, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 1e-5, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 1e-5, "b: {b} vs {b2}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hsl_mid_grey_lightness() {
        let (_, _, l) = rgb_to_hsl(0.5, 0.5, 0.5);
        assert!((l - 0.5).abs() < 1e-5, "grey lightness: {l}");
    }

    // ── fresnel_schlick ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fresnel_endpoints() {
        assert!((fresnel_schlick(1.0, 0.04) - 0.04).abs() < 1e-6);
        assert!((fresnel_schlick(0.0, 0.04) - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fresnel_monotone_increasing_toward_grazing() {
        let f1 = fresnel_schlick(0.7, 0.04);
        let f2 = fresnel_schlick(0.3, 0.04);
        assert!(f1 < f2, "more grazing → higher Fresnel: {f1} < {f2}");
    }

    // ── ray_sphere_intersect ───────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_sphere_hit_front() {
        let (t0, t1) =
            ray_sphere_intersect(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 5.0), 1.0).unwrap();
        assert!((t0 - 4.0).abs() < 1e-5 && (t1 - 6.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_sphere_miss() {
        let hit = ray_sphere_intersect(Vec3::ZERO, Vec3::Z, Vec3::new(2.0, 0.0, 5.0), 1.0);
        assert!(hit.is_none());
    }

    // ── ray_plane_intersect ────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_plane_hits() {
        // XY plane at z=3, ray from z=0 along +Z
        let t = ray_plane_intersect(Vec3::ZERO, Vec3::Z, Vec3::Z, 3.0).unwrap();
        assert!((t - 3.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_plane_parallel_misses() {
        // Ray along +X into a Z-normal plane → parallel
        let hit = ray_plane_intersect(Vec3::ZERO, Vec3::X, Vec3::Z, 1.0);
        assert!(hit.is_none());
    }

    // ── ray_aabb_intersect ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_aabb_centre_shot() {
        let (t0, t1) = ray_aabb_intersect(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, -1.0, -1.0),
            Vec3::new(4.0, 1.0, 1.0),
        )
        .unwrap();
        assert!((t0 - 2.0).abs() < 1e-5 && (t1 - 4.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_aabb_miss() {
        let hit = ray_aabb_intersect(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(4.0, 4.0, 4.0),
        );
        assert!(hit.is_none());
    }

    // ── igr_noise ──────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn igr_in_range() {
        for i in 0..64u32 {
            let n = igr_noise(i as f32, (i * 7) as f32);
            assert!(n >= 0.0 && n < 1.0, "igr out of range: {n}");
        }
    }

    // ── value_noise_2d ─────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn value_noise_in_range() {
        for i in 0..64u32 {
            let n = value_noise_2d(Vec2::new(i as f32 * 0.37, i as f32 * 0.61));
            assert!(n >= 0.0 && n <= 1.0, "noise out of range: {n}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn value_noise_smooth() {
        let p = Vec2::new(3.5, 2.5);
        let n0 = value_noise_2d(p);
        let n1 = value_noise_2d(Vec2::new(3.51, 2.51));
        assert!((n0 - n1).abs() < 0.05, "noise not smooth: {n0} vs {n1}");
    }
}

#[cfg(test)]
mod tests_pass_24 {
    use super::*;

    // ── oklab ─────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklab_round_trip() {
        for (r, g, b) in [(1.0_f32, 0.0, 0.0), (0.0, 1.0, 0.0), (0.5, 0.3, 0.8)] {
            let (l, a, bb) = linear_rgb_to_oklab(r, g, b);
            let (r2, g2, b2) = oklab_to_linear_rgb(l, a, bb);
            assert!((r - r2).abs() < 1e-4, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 1e-4, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 1e-4, "b: {b} vs {b2}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklab_white_is_l1() {
        let (l, a, b) = linear_rgb_to_oklab(1.0, 1.0, 1.0);
        assert!((l - 1.0).abs() < 1e-4, "white L≈1: {l}");
        assert!(a.abs() < 1e-4 && b.abs() < 1e-4, "white a,b≈0: {a} {b}");
    }

    // ── perlin_noise_2d ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_zero_at_integer_points() {
        for i in 0..5_i32 {
            for j in 0..5_i32 {
                let n = perlin_noise_2d(Vec2::new(i as f32, j as f32));
                assert!(n.abs() < 1e-5, "perlin zero at integer: ({i},{j}) = {n}");
            }
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_range() {
        for i in 0..100u32 {
            let n = perlin_noise_2d(Vec2::new(i as f32 * 0.37, i as f32 * 0.61));
            assert!(n > -2.0 && n < 2.0, "perlin in rough range: {n}");
        }
    }

    // ── fbm_2d ────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fbm_in_range() {
        for i in 0..50u32 {
            let n = fbm_2d(Vec2::new(i as f32 * 0.31, i as f32 * 0.71), 5, 2.0, 0.5);
            assert!(n >= 0.0 && n <= 1.0, "fBm out of range: {n}");
        }
    }

    // ── worley_noise_2d ───────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn worley_non_negative() {
        for i in 0..64u32 {
            let n = worley_noise_2d(Vec2::new(i as f32 * 0.23, i as f32 * 0.47));
            assert!(n >= 0.0, "worley non-negative: {n}");
        }
    }

    // ── rgba_over ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn porter_duff_opaque_src() {
        // Fully opaque src completely covers dst
        let (r, g, b, a) = rgba_over(0.8, 0.2, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0);
        assert!((r - 0.8).abs() < 1e-5 && (b).abs() < 1e-5 && (a - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn porter_duff_transparent_src() {
        // Fully transparent src → dst unchanged
        let (r, g, b, a) = rgba_over(0.0, 0.0, 0.0, 0.0, 0.5, 0.3, 0.1, 0.7);
        assert!((r - 0.5).abs() < 1e-5 && (a - 0.7).abs() < 1e-5);
        let _ = (g, b);
    }

    // ── color_temperature_rgb ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn warm_vs_cool_temperature() {
        let (r_warm, _, b_warm) = color_temperature_rgb(2700.0);
        let (r_cool, _, b_cool) = color_temperature_rgb(6500.0);
        assert!(r_warm > b_warm, "warm: red > blue: {r_warm} vs {b_warm}");
        assert!(
            b_cool > b_warm,
            "cool: more blue than warm: {b_cool} vs {b_warm}"
        );
    }

    // ── gcd_u32 / lcm_u32 ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gcd_basic() {
        assert_eq!(gcd_u32(12, 8), 4);
        assert_eq!(gcd_u32(7, 13), 1);
        assert_eq!(gcd_u32(0, 5), 5);
        assert_eq!(gcd_u32(100, 25), 25);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lcm_basic() {
        assert_eq!(lcm_u32(4, 6), 12);
        assert_eq!(lcm_u32(0, 5), 0);
        assert_eq!(lcm_u32(7, 3), 21);
    }
}

#[cfg(test)]
mod tests_pass_25 {
    use super::*;

    // ── luminance_rec709 ───────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn luminance_white_and_black() {
        assert!((luminance_rec709(1.0, 1.0, 1.0) - 1.0).abs() < 1e-5);
        assert!(luminance_rec709(0.0, 0.0, 0.0).abs() < 1e-9);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn luminance_green_dominates() {
        assert!(luminance_rec709(0.0, 1.0, 0.0) > luminance_rec709(1.0, 0.0, 0.0));
        assert!(luminance_rec709(0.0, 1.0, 0.0) > luminance_rec709(0.0, 0.0, 1.0));
    }

    // ── oklab_mix ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklab_mix_endpoints() {
        let (l, a, b) = oklab_mix(0.5, 0.1, 0.2, 0.8, 0.3, 0.4, 0.0);
        assert!((l - 0.5).abs() < 1e-6 && (a - 0.1).abs() < 1e-6);
        let (l, a, b) = oklab_mix(0.5, 0.1, 0.2, 0.8, 0.3, 0.4, 1.0);
        assert!((l - 0.8).abs() < 1e-6 && (a - 0.3).abs() < 1e-6 && (b - 0.4).abs() < 1e-6);
    }

    // ── oklab_rotate_hue ──────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklab_hue_rotation_180() {
        let (l, a, b) = oklab_rotate_hue(0.6, 0.2, 0.1, 180.0);
        assert!((l - 0.6).abs() < 1e-5, "L unchanged: {l}");
        assert!((a + 0.2).abs() < 1e-5, "a inverted: {a}");
        assert!((b + 0.1).abs() < 1e-5, "b inverted: {b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklab_hue_rotation_preserves_chroma() {
        let a0 = 0.3_f32;
        let b0 = 0.1_f32;
        let chroma0 = (a0 * a0 + b0 * b0).sqrt();
        let (_, a1, b1) = oklab_rotate_hue(0.5, a0, b0, 90.0);
        let chroma1 = (a1 * a1 + b1 * b1).sqrt();
        assert!(
            (chroma0 - chroma1).abs() < 1e-5,
            "chroma preserved: {chroma0} vs {chroma1}"
        );
    }

    // ── sample_cosine_hemisphere ──────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cosine_hemisphere_unit_length() {
        for i in 0..20u32 {
            let u1 = (i as f32 + 0.5) / 20.0;
            let u2 = ((i * 7 + 3) as f32) / 20.0 % 1.0;
            let d = sample_cosine_hemisphere(u1, u2);
            let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            assert!((len - 1.0).abs() < 1e-4, "unit: {len}");
            assert!(d.y >= -1e-5, "upper hemisphere: {}", d.y);
        }
    }

    // ── sample_uniform_sphere ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn uniform_sphere_unit_length() {
        for i in 0..20u32 {
            let u1 = (i as f32 + 0.5) / 20.0;
            let u2 = ((i * 7 + 3) as f32) / 20.0 % 1.0;
            let d = sample_uniform_sphere(u1, u2);
            let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            assert!((len - 1.0).abs() < 1e-4, "unit: {len}");
        }
    }

    // ── ease_back ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn back_in_endpoints() {
        assert!(ease_back_in(0.0, 1.70158).abs() < 1e-5);
        assert!((ease_back_in(1.0, 1.70158) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn back_out_overshoots() {
        assert!(ease_back_out(0.75, 1.70158) > 1.0, "overshoot near end");
    }

    // ── ease_elastic_out ──────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elastic_out_endpoints() {
        assert!(ease_elastic_out(0.0, 1.0, 0.3).abs() < 1e-5);
        assert!((ease_elastic_out(1.0, 1.0, 0.3) - 1.0).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_26 {
    use super::*;

    // ── ray_triangle_intersect ────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn moller_trumbore_hit() {
        let a = Vec3::new(-1.0, 0.0, 3.0);
        let b = Vec3::new(1.0, 0.0, 3.0);
        let c = Vec3::new(0.0, 1.0, 3.0);
        let (t, u, v) = ray_triangle_intersect(Vec3::ZERO, Vec3::Z, a, b, c).unwrap();
        assert!((t - 3.0).abs() < 1e-4, "t: {t}");
        assert!(u >= 0.0 && v >= 0.0 && u + v <= 1.0, "bary: {u} {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn moller_trumbore_miss() {
        // Ray along +X, triangle in +Z plane
        let a = Vec3::new(-1.0, 0.0, 3.0);
        let b = Vec3::new(1.0, 0.0, 3.0);
        let c = Vec3::new(0.0, 1.0, 3.0);
        let hit = ray_triangle_intersect(Vec3::ZERO, Vec3::X, a, b, c);
        assert!(hit.is_none(), "should miss");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn moller_trumbore_parallel() {
        // Ray along +Z, triangle also in +Z-parallel plane (normal = +X)
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 0.0);
        let c = Vec3::new(1.0, 0.0, 1.0);
        let hit = ray_triangle_intersect(Vec3::ZERO, Vec3::Z, a, b, c);
        assert!(hit.is_none(), "parallel should miss");
    }

    // ── ggx_ndf ───────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_ndf_smooth_brighter() {
        let d_smooth = ggx_ndf(1.0, 0.01);
        let d_rough = ggx_ndf(1.0, 1.0);
        assert!(d_smooth > d_rough, "smooth > rough at n·h=1");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_ndf_positive() {
        for r in [0.1_f32, 0.3, 0.5, 0.8, 1.0] {
            let d = ggx_ndf(0.8, r);
            assert!(d > 0.0, "positive NDF: {d}");
        }
    }

    // ── ggx_geometry_smith ────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_geometry_in_range() {
        let g = ggx_geometry_smith(0.9, 0.9, 0.5);
        assert!(g > 0.0 && g <= 1.0, "G in (0,1]: {g}");
    }

    // ── uncharted2_tonemap ────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn uncharted2_zero_in_zero_out() {
        assert!(uncharted2_tonemap(0.0).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn uncharted2_bounded() {
        // Uncharted2 saturates at (1-E/F)/partial(11.2) ≈ 1.287 as input → ∞.
        // Values above the white point (11.2) legitimately exceed 1.0; callers
        // should clamp or rely on exposure to keep scene values ≤ 11.2.
        let large = uncharted2_tonemap(1000.0);
        assert!(
            large > 0.0 && large < 1.4,
            "asymptote out of range: {large}"
        );
        // Verify the white-point itself maps to exactly 1.0
        let white = uncharted2_tonemap(11.2);
        assert!((white - 1.0).abs() < 1e-5, "white point: {white}");
    }

    // ── SH basis ──────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sh_y00_constant() {
        assert!((sh_y00() - 0.282_094_79).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sh_y1_poles() {
        // +Z: Y10 non-zero, others zero
        let b = sh_y1(Vec3::Z);
        assert!(b[0].abs() < 1e-5 && b[2].abs() < 1e-5);
        assert!(b[1] > 0.4);
        // +X: Y11 non-zero, others zero
        let bx = sh_y1(Vec3::X);
        assert!(bx[0].abs() < 1e-5 && bx[1].abs() < 1e-5);
        assert!(bx[2] > 0.4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sh_y2_z_pole() {
        let b = sh_y2(Vec3::Z);
        // At +Z: Y2^0 = 0.315... * 2 ≈ 0.63; others zero
        assert!((b[2] - 0.315_391_57 * 2.0).abs() < 1e-4, "Y20: {}", b[2]);
        assert!(b[0].abs() < 1e-5 && b[1].abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_27 {
    use super::*;
    use crate::math::Vec3;

    // ── ease_bounce_out ───────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bounce_out_endpoints() {
        assert!(ease_bounce_out(0.0).abs() < 1e-5);
        assert!((ease_bounce_out(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bounce_out_monotone_overall() {
        // Output at t=1 > output at t=0
        assert!(ease_bounce_out(1.0) > ease_bounce_out(0.0));
    }

    // ── ease_bounce_in ────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bounce_in_endpoints() {
        assert!(ease_bounce_in(0.0).abs() < 1e-5);
        assert!((ease_bounce_in(1.0) - 1.0).abs() < 1e-4);
    }

    // ── ease_circ ─────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn circ_in_slower_than_linear() {
        assert!(ease_circ_in(0.5) < 0.5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn circ_out_faster_than_linear() {
        assert!(ease_circ_out(0.5) > 0.5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn circ_in_out_endpoints() {
        assert!(ease_circ_in(0.0).abs() < 1e-6);
        assert!((ease_circ_in(1.0) - 1.0).abs() < 1e-5);
        assert!(ease_circ_out(0.0).abs() < 1e-6);
        assert!((ease_circ_out(1.0) - 1.0).abs() < 1e-5);
    }

    // ── sinc ──────────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sinc_at_zero_is_one() {
        assert!((sinc(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sinc_zero_crossings() {
        // Zeros at all non-zero integers
        assert!(sinc(1.0).abs() < 1e-6);
        assert!(sinc(2.0).abs() < 1e-6);
        assert!(sinc(-1.0).abs() < 1e-6);
    }

    // ── lanczos_kernel ────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lanczos_center_is_one() {
        assert!((lanczos_kernel(0.0, 3.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lanczos_outside_support_is_zero() {
        assert_eq!(lanczos_kernel(3.5, 3.0), 0.0);
        assert_eq!(lanczos_kernel(-4.0, 3.0), 0.0);
    }

    // ── mitchell_netravali ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mitchell_catmull_rom_interpolates() {
        // Catmull-Rom (b=0, c=0.5) is the interpolating special case: f(0)=1
        let v = mitchell_netravali(0.0, 0.0, 0.5);
        assert!((v - 1.0).abs() < 1e-5, "Catmull-Rom at 0: {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mitchell_recommended_approximates() {
        // b=1/3, c=1/3: f(0) = (6 - 2/3) / 6 = 8/9
        let v = mitchell_netravali(0.0, 1.0 / 3.0, 1.0 / 3.0);
        assert!((v - 8.0 / 9.0).abs() < 1e-5, "MN(0) approx: {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mitchell_outside_support_is_zero() {
        assert_eq!(mitchell_netravali(2.0, 1.0 / 3.0, 1.0 / 3.0), 0.0);
        assert_eq!(mitchell_netravali(-2.5, 0.0, 0.5), 0.0);
    }

    // ── oklab_to_oklch / oklch_to_oklab ──────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklch_roundtrip() {
        let (l0, a0, b0) = (0.6_f32, 0.08, 0.12);
        let (l1, c, h) = oklab_to_oklch(l0, a0, b0);
        let (l2, a2, b2) = oklch_to_oklab(l1, c, h);
        assert!((l0 - l2).abs() < 1e-5);
        assert!((a0 - a2).abs() < 1e-5, "a roundtrip: {a0} vs {a2}");
        assert!((b0 - b2).abs() < 1e-5, "b roundtrip: {b0} vs {b2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oklch_chroma_nonnegative() {
        let (_, c, _) = oklab_to_oklch(0.5, -0.1, -0.1);
        assert!(c >= 0.0);
    }

    // ── ray_disk_intersect ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_disk_hit() {
        let t = ray_disk_intersect(
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::ZERO,
            Vec3::Z,
            1.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 2.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_disk_miss_outside_radius() {
        let t = ray_disk_intersect(
            Vec3::new(2.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::ZERO,
            Vec3::Z,
            1.0,
        );
        assert!(t.is_none(), "should miss: {t:?}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_disk_parallel_miss() {
        let t = ray_disk_intersect(Vec3::new(0.0, 0.0, 1.0), Vec3::X, Vec3::ZERO, Vec3::Z, 1.0);
        assert!(t.is_none());
    }

    // ── barycentric_3d ────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn barycentric_centroid() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 3.0, 0.0);
        let centroid = Vec3::new(1.0, 1.0, 0.0);
        let (u, v, w) = barycentric_3d(centroid, a, b, c);
        assert!((u - 1.0 / 3.0).abs() < 1e-5, "u: {u}");
        assert!((v - 1.0 / 3.0).abs() < 1e-5, "v: {v}");
        assert!((w - 1.0 / 3.0).abs() < 1e-5, "w: {w}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn barycentric_vertex_a() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        let (u, v, w) = barycentric_3d(a, a, b, c);
        assert!((u - 1.0).abs() < 1e-5, "u at A: {u}");
        assert!(v.abs() < 1e-5, "v at A: {v}");
        assert!(w.abs() < 1e-5, "w at A: {w}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn barycentric_sums_to_one() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(1.0, 2.0, 0.0);
        let p = Vec3::new(0.8, 0.4, 0.0);
        let (u, v, w) = barycentric_3d(p, a, b, c);
        assert!((u + v + w - 1.0).abs() < 1e-5, "sum: {}", u + v + w);
    }
}

#[cfg(test)]
mod tests_pass_28 {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    // ── Quat::identity ────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn identity_is_unit() {
        let q = Quat::identity();
        assert!((q.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn identity_rotates_nothing() {
        let v = Quat::identity().rotate(Vec3::new(1.0, 2.0, 3.0));
        assert!((v.x - 1.0).abs() < 1e-5);
        assert!((v.y - 2.0).abs() < 1e-5);
        assert!((v.z - 3.0).abs() < 1e-5);
    }

    // ── Quat::from_axis_angle ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rotate_90_about_y() {
        // +X rotated 90° about +Y → -Z
        let q = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let v = q.rotate(Vec3::X);
        assert!(v.x.abs() < 1e-5, "x: {}", v.x);
        assert!(v.y.abs() < 1e-5, "y: {}", v.y);
        assert!((v.z + 1.0).abs() < 1e-5, "z: {}", v.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rotate_180_about_y() {
        let q = Quat::from_axis_angle(Vec3::Y, PI);
        let v = q.rotate(Vec3::X);
        assert!((v.x + 1.0).abs() < 1e-5, "x: {}", v.x);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn axis_angle_roundtrip() {
        let axis = Vec3::new(1.0, 0.0, 0.0);
        let angle = 1.2_f32;
        let q = Quat::from_axis_angle(axis, angle);
        assert!((q.angle() - angle).abs() < 1e-5, "angle: {}", q.angle());
        let a = q.axis();
        assert!((a.x - 1.0).abs() < 1e-5, "axis.x: {}", a.x);
    }

    // ── Quat multiplication ───────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mul_identity_is_noop() {
        let q = Quat::from_axis_angle(Vec3::Y, 0.7);
        let r = q * Quat::identity();
        assert!((r.x - q.x).abs() < 1e-6);
        assert!((r.w - q.w).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mul_composed_rotation() {
        // 90° about Y twice = 180° about Y: +X → -X
        let q90 = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let q180 = q90 * q90;
        let v = q180.rotate(Vec3::X);
        assert!((v.x + 1.0).abs() < 1e-5, "x: {}", v.x);
    }

    // ── Quat::conjugate / inverse ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn conjugate_undoes_rotation() {
        let q = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let rotated = q.rotate(v);
        let back = q.conjugate().rotate(rotated);
        assert!((back.x - v.x).abs() < 1e-5, "back.x: {}", back.x);
        assert!((back.y - v.y).abs() < 1e-5, "back.y: {}", back.y);
    }

    // ── Quat::slerp ───────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_t0_is_start() {
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::Y, PI);
        let s = q0.slerp(q1, 0.0);
        assert!((s.w - q0.w).abs() < 1e-5, "w: {}", s.w);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_t1_is_end() {
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::Y, PI);
        let s = q0.slerp(q1, 1.0);
        // q and -q represent the same rotation; verify via rotate rather than components.
        let v = s.rotate(Vec3::X);
        let v1 = q1.rotate(Vec3::X);
        assert!((v.x - v1.x).abs() < 1e-4, "x: {} vs {}", v.x, v1.x);
        assert!((v.z - v1.z).abs() < 1e-4, "z: {} vs {}", v.z, v1.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_midpoint_is_half_angle() {
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let mid = q0.slerp(q1, 0.5);
        assert!(
            (mid.angle() - FRAC_PI_2 / 2.0).abs() < 1e-4,
            "angle: {}",
            mid.angle()
        );
    }

    // ── Quat::to_mat3 ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn to_mat3_identity() {
        let m = Quat::identity().to_mat3();
        // diagonal = 1
        assert!((m.m[0][0] - 1.0).abs() < 1e-6);
        assert!((m.m[1][1] - 1.0).abs() < 1e-6);
        assert!((m.m[2][2] - 1.0).abs() < 1e-6);
        // off-diagonal = 0
        assert!(m.m[0][1].abs() < 1e-6);
        assert!(m.m[0][2].abs() < 1e-6);
        assert!(m.m[1][0].abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn to_mat3_consistent_with_rotate() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0).normalize(), 1.1);
        let v = Vec3::new(0.5, -0.3, 0.8);
        let via_quat = q.rotate(v);
        let via_mat = q.to_mat3() * v;
        assert!((via_quat.x - via_mat.x).abs() < 1e-5, "x diff");
        assert!((via_quat.y - via_mat.y).abs() < 1e-5, "y diff");
        assert!((via_quat.z - via_mat.z).abs() < 1e-5, "z diff");
    }

    // ── Quat::from_euler_zyx ──────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn euler_identity_is_identity() {
        let q = Quat::from_euler_zyx(0.0, 0.0, 0.0);
        assert!((q.w - 1.0).abs() < 1e-6);
        assert!(q.x.abs() < 1e-6 && q.y.abs() < 1e-6 && q.z.abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn euler_yaw_90_matches_axis_angle() {
        let q_euler = Quat::from_euler_zyx(FRAC_PI_2, 0.0, 0.0);
        let q_aa = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        assert!((q_euler.x - q_aa.x).abs() < 1e-5);
        assert!((q_euler.y - q_aa.y).abs() < 1e-5);
        assert!((q_euler.z - q_aa.z).abs() < 1e-5);
        assert!((q_euler.w - q_aa.w).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_29 {
    use super::*;
    use crate::math::Vec3;

    // ── reflect ───────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reflect_vertical_normal() {
        let r = reflect(Vec3::new(0.0, -1.0, 0.0), Vec3::Y);
        assert!((r.y - 1.0).abs() < 1e-5, "ry: {}", r.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reflect_45_degree() {
        // incident at 45° from normal → reflects at 45° on other side
        let i = Vec3::new(1.0, -1.0, 0.0).normalize();
        let r = reflect(i, Vec3::Y);
        assert!((r.x - i.x).abs() < 1e-5);
        assert!((r.y + i.y).abs() < 1e-5); // y flips
    }

    // ── refract ───────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn refract_normal_incidence_eta1() {
        // eta=1 (same medium): refracted = incident
        let i = Vec3::new(0.0, -1.0, 0.0);
        let r = refract(i, Vec3::Y, 1.0).expect("should transmit");
        assert!((r.y + 1.0).abs() < 1e-5, "ry: {}", r.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn refract_total_internal_reflection() {
        // Large eta, glancing angle → TIR
        let i = Vec3::new(0.999, -0.045, 0.0).normalize();
        let result = refract(i, Vec3::Y, 1.5);
        assert!(result.is_none(), "should TIR");
    }

    // ── closest_point_on_segment_3d ───────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_projects_onto_segment() {
        let c = closest_point_on_segment_3d(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(c.y.abs() < 1e-5, "y: {}", c.y);
        assert!(c.x.abs() < 1e-5, "x: {}", c.x);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_clamps_to_endpoint() {
        // Beyond end of segment → returns endpoint
        let c = closest_point_on_segment_3d(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!((c.x - 1.0).abs() < 1e-5, "x: {}", c.x);
    }

    // ── project_point_to_plane ────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn project_point_above_plane() {
        let p = project_point_to_plane(Vec3::new(0.0, 3.0, 0.0), Vec3::Y, 1.0);
        assert!((p.y - 1.0).abs() < 1e-5, "y: {}", p.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn project_point_on_plane_unchanged() {
        let p = Vec3::new(3.0, 1.0, 2.0);
        let proj = project_point_to_plane(p, Vec3::Y, 1.0);
        assert!((proj.x - 3.0).abs() < 1e-5);
        assert!((proj.y - 1.0).abs() < 1e-5);
    }

    // ── easing: quad ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quad_easing_endpoints() {
        for &v in &[ease_quad_in(0.0), ease_quad_out(0.0), ease_quad_in_out(0.0)] {
            assert!(v.abs() < 1e-6, "start: {v}");
        }
        for &v in &[ease_quad_in(1.0), ease_quad_out(1.0), ease_quad_in_out(1.0)] {
            assert!((v - 1.0).abs() < 1e-5, "end: {v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quad_in_slower_than_out_at_midpoint() {
        assert!(ease_quad_in(0.5) < ease_quad_out(0.5));
    }

    // ── easing: cubic ────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cubic_easing_endpoints() {
        assert!(ease_cubic_in(0.0).abs() < 1e-6);
        assert!((ease_cubic_in(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_cubic_out(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_cubic_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    // ── easing: sine ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sine_easing_endpoints() {
        assert!(ease_sine_in(0.0).abs() < 1e-6);
        assert!((ease_sine_in(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_sine_out(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_sine_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    // ── gaussian ─────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gaussian_peak_at_mean() {
        let peak = gaussian(1.0, 1.0, 0.5);
        let off = gaussian(1.5, 1.0, 0.5);
        assert!(peak > off, "peak {peak} vs off {off}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gaussian_standard_normal() {
        let v = gaussian(0.0, 0.0, 1.0);
        assert!((v - 0.398_942_28_f32).abs() < 1e-5, "N(0,1) pdf: {v}");
    }

    // ── erf_approx ───────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn erf_at_zero() {
        assert!(erf_approx(0.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn erf_odd_function() {
        let x = 0.7_f32;
        assert!((erf_approx(x) + erf_approx(-x)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn erf_converges_to_one() {
        assert!((erf_approx(5.0) - 1.0).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_30 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── perlin_noise_3d ───────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_3d_in_range() {
        let n = perlin_noise_3d(Vec3::new(1.5, 2.3, 0.7));
        assert!(n >= -1.0 && n <= 1.0, "out of range: {n}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_3d_grid_point_near_zero() {
        // At integer lattice points, all gradients cancel → near 0.
        let n = perlin_noise_3d(Vec3::new(1.0, 2.0, 3.0));
        assert!(n.abs() < 1e-5, "lattice: {n}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_3d_varies() {
        let n1 = perlin_noise_3d(Vec3::new(0.3, 0.7, 0.5));
        let n2 = perlin_noise_3d(Vec3::new(1.3, 0.7, 0.5));
        assert!((n1 - n2).abs() > 1e-3, "no variation: {n1} {n2}");
    }

    // ── fbm_3d ────────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fbm_3d_range() {
        let n = fbm_3d(Vec3::new(1.0, 2.0, 0.5), 4, 2.0, 0.5);
        assert!(n >= -3.0 && n <= 3.0, "fbm out of range: {n}");
    }

    // ── lerp_vec ──────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lerp_vec2_midpoint() {
        let v = lerp_vec2(Vec2::ZERO, Vec2::new(2.0, 4.0), 0.5);
        assert!((v.x - 1.0).abs() < 1e-6);
        assert!((v.y - 2.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lerp_vec3_endpoints() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(5.0, 6.0, 7.0);
        let v0 = lerp_vec3(a, b, 0.0);
        let v1 = lerp_vec3(a, b, 1.0);
        assert!((v0.x - 1.0).abs() < 1e-6);
        assert!((v1.z - 7.0).abs() < 1e-6);
    }

    // ── segment_intersect_2d ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn segments_x_cross() {
        assert!(segment_intersect_2d(
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
        ));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn segments_parallel_no_cross() {
        assert!(!segment_intersect_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn segments_t_no_extend() {
        // Segments that would cross if extended but don't overlap.
        assert!(!segment_intersect_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, -1.0),
            Vec2::new(2.0, 1.0),
        ));
    }

    // ── ease_expo ─────────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn expo_easing_endpoints() {
        assert!(ease_expo_in(0.0).abs() < 1e-5);
        assert!((ease_expo_in(1.0) - 1.0).abs() < 1e-5);
        assert!(ease_expo_out(0.0).abs() < 1e-5);
        assert!((ease_expo_out(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_expo_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn expo_in_very_slow_at_midpoint() {
        assert!(ease_expo_in(0.5) < 0.05);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn expo_out_fast_at_start() {
        assert!(ease_expo_out(0.5) > 0.95);
    }
}

#[cfg(test)]
mod tests_pass_31 {
    use super::*;
    use crate::math::Vec3;

    // ── ease_elastic_in ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elastic_in_endpoints() {
        assert!(ease_elastic_in(0.0, 1.0, 0.3).abs() < 1e-5);
        assert!((ease_elastic_in(1.0, 1.0, 0.3) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elastic_in_slow_at_start() {
        // Elastic-in barely moves in the first quarter.
        assert!(ease_elastic_in(0.1, 1.0, 0.3).abs() < 0.1);
    }

    // ── ease_elastic_in_out ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elastic_in_out_endpoints() {
        assert!(ease_elastic_in_out(0.0, 1.0, 0.45).abs() < 1e-5);
        assert!((ease_elastic_in_out(1.0, 1.0, 0.45) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elastic_in_out_midpoint() {
        // Perfect symmetry → f(0.5) = 0.5.
        let v = ease_elastic_in_out(0.5, 1.0, 0.45);
        assert!((v - 0.5).abs() < 1e-4, "midpoint: {v}");
    }

    // ── ease_back_in_out ──────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn back_in_out_endpoints() {
        assert!(ease_back_in_out(0.0, 1.70158).abs() < 1e-5);
        assert!((ease_back_in_out(1.0, 1.70158) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn back_in_out_midpoint_is_half() {
        let v = ease_back_in_out(0.5, 1.70158);
        assert!((v - 0.5).abs() < 1e-4, "midpoint: {v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn back_in_out_overshoots_at_quarter() {
        // Back-in-out should go slightly negative near t=0.25.
        let v = ease_back_in_out(0.2, 1.70158);
        assert!(v < 0.0, "should undershoot at t=0.2: {v}");
    }

    // ── value_noise_3d ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn value_noise_3d_in_range() {
        let n = value_noise_3d(Vec3::new(1.5, 2.3, 0.7));
        assert!(n >= 0.0 && n <= 1.0, "out of range: {n}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn value_noise_3d_varies() {
        let n1 = value_noise_3d(Vec3::new(0.3, 0.7, 0.5));
        let n2 = value_noise_3d(Vec3::new(1.3, 0.7, 0.5));
        assert!((n1 - n2).abs() > 1e-3, "no variation: {n1} {n2}");
    }

    // ── worley_noise_3d ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn worley_noise_3d_non_negative() {
        for (x, y, z) in [(0.5, 0.5, 0.5), (1.2, 3.4, 5.6), (0.0, 0.0, 0.0)] {
            let d = worley_noise_3d(Vec3::new(x, y, z));
            assert!(d >= 0.0, "negative: {d} at ({x},{y},{z})");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn worley_noise_3d_varies() {
        let d1 = worley_noise_3d(Vec3::new(0.1, 0.1, 0.1));
        let d2 = worley_noise_3d(Vec3::new(0.6, 0.6, 0.6));
        assert!((d1 - d2).abs() > 1e-3, "no variation: {d1} {d2}");
    }

    // ── closest_point_on_triangle_3d ──────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_projects_interior() {
        // Point directly above the centroid should project to the centroid.
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 3.0);
        let centroid = Vec3::new(1.0, 0.0, 1.0);
        let p = Vec3::new(1.0, 5.0, 1.0);
        let cp = closest_point_on_triangle_3d(p, a, b, c);
        assert!((cp.x - centroid.x).abs() < 1e-4, "x: {}", cp.x);
        assert!(cp.y.abs() < 1e-4, "y: {}", cp.y);
        assert!((cp.z - centroid.z).abs() < 1e-4, "z: {}", cp.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_snaps_to_vertex() {
        let a = Vec3::ZERO;
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        // Far in the −x direction → nearest vertex is A.
        let cp = closest_point_on_triangle_3d(Vec3::new(-2.0, 0.0, 0.0), a, b, c);
        assert!(cp.x.abs() < 1e-5 && cp.y.abs() < 1e-5 && cp.z.abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_snaps_to_edge() {
        let a = Vec3::ZERO;
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        // Directly above midpoint of AB on +Y.
        let mid_ab = Vec3::new(0.5, 5.0, 0.0);
        let cp = closest_point_on_triangle_3d(mid_ab, a, b, c);
        assert!((cp.x - 0.5).abs() < 1e-4, "x: {}", cp.x);
        assert!(cp.y.abs() < 1e-4, "y: {}", cp.y);
        assert!(cp.z.abs() < 1e-4, "z: {}", cp.z);
    }

    // ── ray_capsule_intersect ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_capsule_direct_hit() {
        // Ray along +Z, capsule along Y-axis at z=5.
        let hit = ray_capsule_intersect(
            Vec3::new(0.0, 0.5, -5.0),
            Vec3::Z,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        assert!(hit.is_some(), "should hit capsule body");
        assert!(hit.unwrap() > 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_capsule_miss() {
        // Ray passes well to the side.
        let hit = ray_capsule_intersect(
            Vec3::new(5.0, 0.5, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        assert!(hit.is_none(), "should miss");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_capsule_end_cap_hit() {
        // Ray aimed directly at the bottom hemisphere.
        let hit = ray_capsule_intersect(
            Vec3::new(0.0, -5.0, 0.0),
            Vec3::Y,
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        assert!(hit.is_some(), "should hit end cap");
    }
}

#[cfg(test)]
mod tests_pass_32 {
    use super::*;
    use crate::math::Vec3;

    // ── quartic easing ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quart_easing_endpoints() {
        assert!(ease_quart_in(0.0).abs() < 1e-6);
        assert!((ease_quart_in(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_quart_out(0.0).abs() < 1e-6);
        assert!((ease_quart_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_quart_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quart_in_very_slow_early() {
        // At t=0.5 quartic-in should still be small (0.5^4 = 0.0625).
        assert!((ease_quart_in(0.5) - 0.0625).abs() < 1e-5);
    }

    // ── quintic easing ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quint_easing_endpoints() {
        assert!(ease_quint_in(0.0).abs() < 1e-6);
        assert!((ease_quint_in(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_quint_out(0.0).abs() < 1e-6);
        assert!((ease_quint_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_quint_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn quint_slower_than_quart_midpoint() {
        // Higher power → slower start.
        assert!(ease_quint_in(0.5) < ease_quart_in(0.5));
    }

    // ── aabb_contains_point_3d ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aabb_contains_origin() {
        assert!(aabb_contains_point_3d(
            Vec3::ZERO,
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0)
        ));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aabb_excludes_outside() {
        assert!(!aabb_contains_point_3d(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0)
        ));
    }

    // ── aabb_intersects_aabb_3d ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aabb_overlap() {
        assert!(aabb_intersects_aabb_3d(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(2.0, 2.0, 2.0),
        ));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aabb_no_overlap() {
        assert!(!aabb_intersects_aabb_3d(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(2.0, 2.0, 2.0),
        ));
    }

    // ── ray_obb_intersect ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_obb_axis_aligned_hit() {
        // Degenerate OBB (identity axes) == AABB; ray from −Z should hit.
        let hit = ray_obb_intersect(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            [Vec3::X, Vec3::Y, Vec3::Z],
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(hit.is_some(), "should hit AABB-mode OBB");
        assert!((hit.unwrap() - 4.0).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_obb_miss() {
        let hit = ray_obb_intersect(
            Vec3::new(5.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            [Vec3::X, Vec3::Y, Vec3::Z],
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(hit.is_none(), "should miss");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_obb_rotated_45_degrees() {
        // OBB rotated 45° around Y; ray along +Z should still hit a unit box at origin.
        use core::f32::consts::FRAC_1_SQRT_2;
        let axes = [
            Vec3::new(FRAC_1_SQRT_2, 0.0, -FRAC_1_SQRT_2), // X rotated 45° CW around Y
            Vec3::Y,
            Vec3::new(FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2), // Z rotated 45° CW around Y
        ];
        let hit = ray_obb_intersect(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            axes,
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(hit.is_some(), "rotated OBB should still be hit");
    }

    // ── CIE XYZ ───────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn xyz_white_point() {
        // Linear white (1,1,1) → Y ≈ 1.0.
        let (_, y, _) = linear_rgb_to_xyz(1.0, 1.0, 1.0);
        assert!((y - 1.0).abs() < 1e-4, "Y white: {y}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn xyz_round_trip() {
        let cases = [(0.8, 0.3, 0.1_f32), (0.0, 0.5, 1.0), (0.2, 0.7, 0.4)];
        for (r, g, b) in cases {
            let (x, y, z) = linear_rgb_to_xyz(r, g, b);
            let (r2, g2, b2) = xyz_to_linear_rgb(x, y, z);
            assert!((r - r2).abs() < 5e-4, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 5e-4, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 5e-4, "b: {b} vs {b2}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn xyz_black_is_black() {
        let (x, y, z) = linear_rgb_to_xyz(0.0, 0.0, 0.0);
        assert!(x.abs() < 1e-10 && y.abs() < 1e-10 && z.abs() < 1e-10);
    }
}

#[cfg(test)]
mod tests_pass_33 {
    use super::*;
    use crate::math::Vec3;

    // ── quat_rotation_between ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rotation_between_x_to_y() {
        let q = quat_rotation_between(Vec3::X, Vec3::Y);
        let r = q.rotate(Vec3::X);
        assert!((r.x).abs() < 1e-5, "x: {}", r.x);
        assert!((r.y - 1.0).abs() < 1e-5, "y: {}", r.y);
        assert!((r.z).abs() < 1e-5, "z: {}", r.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rotation_between_identity_when_parallel() {
        let q = quat_rotation_between(Vec3::Z, Vec3::Z);
        let r = q.rotate(Vec3::X);
        assert!((r.x - 1.0).abs() < 1e-5 && r.y.abs() < 1e-5 && r.z.abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rotation_between_antiparallel_is_180() {
        // Rotation from +X to −X must produce a vector at −X.
        let q = quat_rotation_between(Vec3::X, Vec3::new(-1.0, 0.0, 0.0));
        let r = q.rotate(Vec3::X);
        assert!((r.x + 1.0).abs() < 1e-4, "antiparallel x: {}", r.x);
    }

    // ── blend modes ───────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blend_multiply_black_kills() {
        assert!(blend_multiply(0.8, 0.0).abs() < 1e-7);
        assert!(blend_multiply(0.0, 0.8).abs() < 1e-7);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blend_screen_white_saturates() {
        assert!((blend_screen(1.0, 0.5) - 1.0).abs() < 1e-7);
        assert!((blend_screen(0.5, 1.0) - 1.0).abs() < 1e-7);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blend_overlay_mid_grey_identity() {
        // overlay(0.5, b) == b (conditioning on a=0.5 gives b in both branches).
        let b = 0.3_f32;
        let o = blend_overlay(0.5, b);
        assert!((o - b).abs() < 1e-6, "overlay(0.5,{b}) = {o}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blend_soft_light_neutral_at_half() {
        // soft_light(a, 0.5) == a.
        let a = 0.7_f32;
        assert!((blend_soft_light(a, 0.5) - a).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blend_hard_light_is_overlay_swapped() {
        let a = 0.4_f32;
        let b = 0.7_f32;
        assert!((blend_hard_light(a, b) - blend_overlay(b, a)).abs() < 1e-7);
    }

    // ── bit-width utilities ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn next_power_of_2_cases() {
        assert_eq!(next_power_of_2(0), 1);
        assert_eq!(next_power_of_2(1), 1);
        assert_eq!(next_power_of_2(5), 8);
        assert_eq!(next_power_of_2(8), 8);
        assert_eq!(next_power_of_2(9), 16);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn is_power_of_2_cases() {
        assert!(is_power_of_2(1));
        assert!(is_power_of_2(2));
        assert!(is_power_of_2(1024));
        assert!(!is_power_of_2(0));
        assert!(!is_power_of_2(3));
        assert!(!is_power_of_2(6));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn log2_ceil_cases() {
        assert_eq!(log2_ceil(1), 0);
        assert_eq!(log2_ceil(2), 1);
        assert_eq!(log2_ceil(4), 2);
        assert_eq!(log2_ceil(5), 3);
        assert_eq!(log2_ceil(8), 3);
        assert_eq!(log2_ceil(9), 4);
    }
}

#[cfg(test)]
mod tests_pass_34 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── depth_linearize ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn depth_at_near_returns_near() {
        // NDC 0 (z = −1 in GL convention) corresponds to near.
        // Using the [0,1] convention: z_ndc=0 → z_view = near.
        let z = depth_linearize(0.0, 0.1, 100.0);
        assert!((z - 0.1).abs() < 1e-4, "z at near: {z}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn depth_at_far_returns_far() {
        let z = depth_linearize(1.0, 0.1, 100.0);
        assert!((z - 100.0).abs() < 1e-2, "z at far: {z}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn depth_midpoint_is_nonlinear() {
        // Non-linear: NDC=0.5 should NOT map to (near+far)/2.
        let z = depth_linearize(0.5, 1.0, 100.0);
        let mid = f32::midpoint(1.0_f32, 100.0);
        assert!(
            (z - mid).abs() > 0.5,
            "should be non-linear: {z} vs linear mid {mid}"
        );
    }

    // ── normal_spheremap_encode / decode ──────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spheremap_forward_z_encodes_to_centre() {
        let enc = normal_spheremap_encode(Vec3::Z);
        assert!((enc.x - 0.5).abs() < 1e-5 && (enc.y - 0.5).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spheremap_round_trip() {
        for n in [
            Vec3::new(0.6, 0.0, 0.8).normalize(),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.577_350_26, 0.577_350_26, 0.577_350_26),
        ] {
            let enc = normal_spheremap_encode(n);
            let dec = normal_spheremap_decode(enc);
            assert!(
                (dec.x - n.x).abs() < 1e-4
                    && (dec.y - n.y).abs() < 1e-4
                    && (dec.z - n.z).abs() < 1e-4,
                "round-trip: {n:?} → {dec:?}"
            );
        }
    }

    // ── triangle_normal ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn triangle_normal_xy_plane_points_z() {
        let n = triangle_normal(Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!(n.z > 0.0 && n.x.abs() < 1e-7 && n.y.abs() < 1e-7);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn triangle_normal_ccw_vs_cw_opposite() {
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let n_ccw = triangle_normal(a, b, c);
        let n_cw = triangle_normal(a, c, b);
        assert!(n_ccw.z > 0.0 && n_cw.z < 0.0);
    }
}

#[cfg(test)]
mod tests_pass_35 {
    use super::*;
    use crate::math::Vec2;

    // ── turbulence_2d ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn turbulence_non_negative() {
        let t = turbulence_2d(Vec2::new(1.5, 2.3), 4, 2.0, 0.5);
        assert!(t >= 0.0, "negative turbulence: {t}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn turbulence_varies() {
        let t1 = turbulence_2d(Vec2::new(0.1, 0.1), 4, 2.0, 0.5);
        let t2 = turbulence_2d(Vec2::new(1.3, 0.7), 4, 2.0, 0.5);
        assert!((t1 - t2).abs() > 1e-3, "no variation: {t1} {t2}");
    }

    // ── marble_texture_2d ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn marble_in_range() {
        let m = marble_texture_2d(Vec2::new(1.0, 0.5), 3.0, 2.0);
        assert!(m >= 0.0 && m <= 1.0, "out of [0,1]: {m}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn marble_varies() {
        let m1 = marble_texture_2d(Vec2::new(0.0, 0.0), 3.0, 2.0);
        let m2 = marble_texture_2d(Vec2::new(1.5, 0.0), 3.0, 2.0);
        assert!((m1 - m2).abs() > 1e-3, "no variation: {m1} {m2}");
    }

    // ── checkerboard_2d ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn checkerboard_alternates() {
        let a = checkerboard_2d(Vec2::new(0.5, 0.5), 1.0);
        let b = checkerboard_2d(Vec2::new(1.5, 0.5), 1.0);
        assert!((a - b).abs() > 0.5, "should alternate: {a} {b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn checkerboard_diagonal_same() {
        let a = checkerboard_2d(Vec2::new(0.5, 0.5), 1.0);
        let b = checkerboard_2d(Vec2::new(1.5, 1.5), 1.0);
        assert!((a - b).abs() < 1e-5, "diagonal should match: {a} {b}");
    }

    // ── pack/unpack RGBA8 ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn unorm_4x8_round_trip() {
        let (r, g, b, a) = (0.8, 0.1, 0.5, 1.0_f32);
        let packed = pack_unorm_4x8(r, g, b, a);
        let (r2, g2, b2, a2) = unpack_unorm_4x8(packed);
        assert!((r - r2).abs() < 0.005, "r: {r} vs {r2}");
        assert!((g - g2).abs() < 0.005, "g: {g} vs {g2}");
        assert!((b - b2).abs() < 0.005, "b: {b} vs {b2}");
        assert!((a - a2).abs() < 0.005, "a: {a} vs {a2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn unorm_4x8_clamps() {
        let packed = pack_unorm_4x8(-0.5, 1.5, 0.5, 0.5);
        let (r, _, _, _) = unpack_unorm_4x8(packed);
        assert!(r.abs() < 0.005, "negative clamped to 0: {r}");
    }

    // ── pack/unpack snorm 2×16 ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn snorm_2x16_round_trip() {
        let (x, y) = (0.6_f32, -0.8_f32);
        let packed = pack_snorm_2x16(x, y);
        let (x2, y2) = unpack_snorm_2x16(packed);
        assert!((x - x2).abs() < 1e-4, "x: {x} vs {x2}");
        assert!((y - y2).abs() < 1e-4, "y: {y} vs {y2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn snorm_2x16_extremes() {
        let packed = pack_snorm_2x16(1.0, -1.0);
        let (x, y) = unpack_snorm_2x16(packed);
        assert!((x - 1.0).abs() < 1e-4, "max: {x}");
        assert!((y + 1.0).abs() < 1e-4, "min: {y}");
    }
}

#[cfg(test)]
mod tests_pass_36 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── bilinear_interp ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bilinear_constant() {
        assert!((bilinear_interp(2.0, 2.0, 2.0, 2.0, 0.3, 0.7) - 2.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bilinear_corners() {
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 0.0, 0.0)).abs() < 1e-6); // v00
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 1.0, 0.0) - 1.0).abs() < 1e-6); // v10
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 0.0, 1.0) - 2.0).abs() < 1e-6); // v01
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 1.0, 1.0) - 3.0).abs() < 1e-6); // v11
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bilinear_midpoint() {
        let v = bilinear_interp(0.0, 1.0, 0.0, 1.0, 0.5, 0.5);
        assert!((v - 0.5).abs() < 1e-6, "midpoint: {v}");
    }

    // ── compute_tangent ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn tangent_xy_aligned_is_x() {
        let t = compute_tangent(
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        );
        assert!((t.x - 1.0).abs() < 1e-4 && t.y.abs() < 1e-4 && t.z.abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn tangent_is_unit_length() {
        let t = compute_tangent(
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        );
        assert!((t.length() - 1.0).abs() < 1e-5, "unit: {}", t.length());
    }

    // ── worley_f1_f2_2d ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn worley_f1_f2_ordering() {
        let (f1, f2) = worley_f1_f2_2d(Vec2::new(1.5, 2.3));
        assert!(f1 >= 0.0, "f1 negative: {f1}");
        assert!(f2 >= f1, "f2 < f1: {f1} {f2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn worley_f2_minus_f1_non_negative() {
        for (x, y) in [(0.1, 0.2), (2.5, 1.7), (0.0, 0.0)] {
            let (f1, f2) = worley_f1_f2_2d(Vec2::new(x, y));
            assert!(f2 - f1 >= 0.0, "cell gap negative at ({x},{y})");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn worley_f1_matches_worley_noise_2d() {
        let p = Vec2::new(1.5, 2.3);
        let (f1, _) = worley_f1_f2_2d(p);
        let w = worley_noise_2d(p);
        assert!((f1 - w).abs() < 1e-5, "f1 {f1} vs worley_noise {w}");
    }
}

#[cfg(test)]
mod tests_pass_37 {
    use super::*;
    use crate::math::Vec2;

    // ── linear_rgb_to_cielab / cielab_to_linear_rgb ───────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cielab_white_l_is_100() {
        let (l, _, _) = linear_rgb_to_cielab(1.0, 1.0, 1.0);
        assert!((l - 100.0).abs() < 0.1, "white L*: {l}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cielab_black_l_is_0() {
        let (l, a, b) = linear_rgb_to_cielab(0.0, 0.0, 0.0);
        assert!(l.abs() < 1e-4 && a.abs() < 1e-4 && b.abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cielab_round_trip() {
        let cases = [(0.8, 0.3, 0.1_f32), (0.0, 0.5, 1.0), (0.2, 0.6, 0.4)];
        for (r, g, b) in cases {
            let (l, a, bb) = linear_rgb_to_cielab(r, g, b);
            let (r2, g2, b2) = cielab_to_linear_rgb(l, a, bb);
            assert!((r - r2).abs() < 5e-4, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 5e-4, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 5e-4, "b: {b} vs {b2}");
        }
    }

    // ── uv_rotate ─────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn uv_rotate_zero_is_identity() {
        let uv = Vec2::new(0.3, 0.7);
        let r = uv_rotate(uv, 0.0);
        assert!((r.x - uv.x).abs() < 1e-5 && (r.y - uv.y).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn uv_rotate_90_from_centre_right() {
        use core::f32::consts::FRAC_PI_2;
        // (1.0, 0.5) is to the right of centre (0.5, 0.5).
        // After 90° CCW rotation it should be above centre: (0.5, 1.0).
        let r = uv_rotate(Vec2::new(1.0, 0.5), FRAC_PI_2);
        assert!((r.x - 0.5).abs() < 1e-5, "x: {}", r.x);
        assert!((r.y - 1.0).abs() < 1e-5, "y: {}", r.y);
    }

    // ── sample_ggx_hemisphere ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_sample_unit_vector() {
        let h = sample_ggx_hemisphere(0.5, 0.3, 0.7);
        let len = (h.x * h.x + h.y * h.y + h.z * h.z).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "unit: {len}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_sample_upper_hemisphere() {
        for (u1, u2) in [(0.0, 0.0), (0.5, 0.5), (0.99, 0.99), (0.1, 0.9)] {
            let h = sample_ggx_hemisphere(0.4, u1, u2);
            assert!(h.z >= 0.0, "z negative at ({u1},{u2}): {}", h.z);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_low_roughness_peaks_at_z() {
        // Near-specular (roughness → 0): samples cluster near z=1.
        let h = sample_ggx_hemisphere(0.01, 0.5, 0.5);
        assert!(h.z > 0.999, "near-specular peak: {}", h.z);
    }
}

#[cfg(test)]
mod tests_pass_38 {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    // ── extract_frustum_planes ──────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn frustum_identity_planes_bracket_ndc() {
        // Identity matrix: clip space == NDC.  The "inside" half-space for each
        // plane should accept (0,0,0,1).
        let m = Mat4::identity();
        let planes = extract_frustum_planes(&m);
        // Point at origin: ax+by+cz+d with (x,y,z)=(0,0,0) → just d term.
        for (i, p) in planes.iter().enumerate() {
            // d == p[3].  For identity the row sums are ±1 so d should be ≥ 0.
            assert!(p[3] >= 0.0, "plane {i} d={} should be ≥0 for origin", p[3]);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn frustum_planes_count() {
        let m = Mat4::identity();
        let planes = extract_frustum_planes(&m);
        assert_eq!(planes.len(), 6);
    }

    // ── sphere_vs_frustum ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_inside_identity_frustum() {
        let m = Mat4::identity();
        let planes = extract_frustum_planes(&m);
        // Origin with radius 0 should be inside.
        assert!(sphere_vs_frustum(Vec3::new(0.0, 0.0, 0.0), 0.0, &planes));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_outside_identity_frustum() {
        let m = Mat4::identity();
        let planes = extract_frustum_planes(&m);
        // A sphere far off-axis with tiny radius should be outside.
        assert!(!sphere_vs_frustum(Vec3::new(10.0, 0.0, 0.0), 0.01, &planes));
    }

    // ── aabb_vs_frustum ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aabb_inside_identity_frustum() {
        let m = Mat4::identity();
        let planes = extract_frustum_planes(&m);
        let min = Vec3::new(-0.1, -0.1, -0.1);
        let max = Vec3::new(0.1, 0.1, 0.1);
        assert!(aabb_vs_frustum(min, max, &planes));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn aabb_outside_identity_frustum() {
        let m = Mat4::identity();
        let planes = extract_frustum_planes(&m);
        let min = Vec3::new(5.0, 0.0, 0.0);
        let max = Vec3::new(6.0, 1.0, 1.0);
        assert!(!aabb_vs_frustum(min, max, &planes));
    }

    // ── signed_angle_3d ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn signed_angle_quarter_turn_positive() {
        let from = Vec3::new(1.0, 0.0, 0.0);
        let to = Vec3::new(0.0, 1.0, 0.0);
        let axis = Vec3::new(0.0, 0.0, 1.0); // +Z up
        let a = signed_angle_3d(from, to, axis);
        assert!((a - FRAC_PI_2).abs() < 1e-5, "expected π/2, got {a}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn signed_angle_quarter_turn_negative() {
        let from = Vec3::new(1.0, 0.0, 0.0);
        let to = Vec3::new(0.0, -1.0, 0.0);
        let axis = Vec3::new(0.0, 0.0, 1.0);
        let a = signed_angle_3d(from, to, axis);
        assert!((a + FRAC_PI_2).abs() < 1e-5, "expected -π/2, got {a}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn signed_angle_half_turn() {
        let from = Vec3::new(1.0, 0.0, 0.0);
        let to = Vec3::new(-1.0, 0.0, 0.0);
        let axis = Vec3::new(0.0, 0.0, 1.0);
        let a = signed_angle_3d(from, to, axis);
        assert!((a.abs() - PI).abs() < 1e-5, "expected ±π, got {a}");
    }

    // ── vec3_slerp ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_endpoints() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let s0 = vec3_slerp(a, b, 0.0);
        let s1 = vec3_slerp(a, b, 1.0);
        assert!((s0.x - 1.0).abs() < 1e-5 && s0.y.abs() < 1e-5);
        assert!(s1.x.abs() < 1e-5 && (s1.y - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_midpoint_is_diagonal() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let mid = vec3_slerp(a, b, 0.5);
        let expected = 1.0_f32 / 2.0_f32.sqrt();
        assert!((mid.x - expected).abs() < 1e-5, "x={}", mid.x);
        assert!((mid.y - expected).abs() < 1e-5, "y={}", mid.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_output_is_unit() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 0.0, 1.0);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let v = vec3_slerp(a, b, t);
            let len = v.length();
            assert!((len - 1.0).abs() < 1e-5, "t={t}: len={len}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slerp_parallel_fallback_is_unit() {
        // Nearly identical vectors should not NaN out.
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(1.0 - 1e-8, 1e-8, 0.0).normalize();
        let v = vec3_slerp(a, b, 0.5);
        let len = v.length();
        assert!((len - 1.0).abs() < 1e-4, "len={len}");
    }
}

#[cfg(test)]
mod tests_pass_39 {
    use super::*;
    use std::f32::consts::PI;

    // ── smooth_min_exp ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_min_exp_approaches_min() {
        // With very large k the smooth-min approaches hard min.
        let a = 1.0_f32;
        let b = 3.0_f32;
        let s = smooth_min_exp(a, b, 20.0);
        assert!((s - a.min(b)).abs() < 0.05, "s={s}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_min_exp_symmetric() {
        let s1 = smooth_min_exp(1.0, 2.0, 1.0);
        let s2 = smooth_min_exp(2.0, 1.0, 1.0);
        assert!((s1 - s2).abs() < 1e-5, "not symmetric: {s1} vs {s2}");
    }

    // ── smooth_min_poly / smooth_max_poly ──────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_min_poly_blends_between() {
        let s = smooth_min_poly(1.0, 3.0, 1.0);
        // Result must be ≤ min(a,b) and > min - k.
        assert!(s <= 1.0 + 1e-5, "s={s} should be ≤ 1");
        assert!(s >= 0.0, "s={s}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_max_poly_blends_between() {
        let s = smooth_max_poly(1.0, 3.0, 1.0);
        assert!(s >= 3.0 - 1e-5, "s={s} should be ≥ 3");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_min_max_poly_symmetric() {
        let smin = smooth_min_poly(1.5, 2.5, 0.5);
        let smax = smooth_max_poly(1.5, 2.5, 0.5);
        // Symmetric: smin(a,b) + smax(a,b) ≈ a + b  (this holds for the quartic variant).
        let sum = smin + smax;
        let expected = 1.5 + 2.5;
        assert!(
            (sum - expected).abs() < 1e-4,
            "sum={sum} expected≈{expected}"
        );
    }

    // ── pingpong ───────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pingpong_zero_at_start() {
        assert!((pingpong(0.0, 1.0)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pingpong_peaks_at_length() {
        assert!((pingpong(1.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pingpong_returns_at_double() {
        assert!((pingpong(2.0, 1.0)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pingpong_bounded() {
        for i in 0..100 {
            let t = i as f32 * 0.13;
            let v = pingpong(t, 1.0);
            assert!(v >= 0.0 && v <= 1.0 + 1e-6, "t={t} v={v}");
        }
    }

    // ── wrap_angle ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn wrap_angle_identity_near_zero() {
        assert!((wrap_angle(0.5) - 0.5).abs() < 1e-6);
        assert!((wrap_angle(-0.5) + 0.5).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn wrap_angle_full_circle() {
        let w = wrap_angle(2.0 * PI + 0.3);
        assert!((w - 0.3).abs() < 1e-5, "w={w}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn wrap_angle_negative_full_circle() {
        let w = wrap_angle(-2.0 * PI - 0.3);
        assert!((w + 0.3).abs() < 1e-5, "w={w}");
    }

    // ── pcg_hash / pcg_hash_2d ─────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcg32_output_differs_by_one_input() {
        let a = pcg32_output(0);
        let b = pcg32_output(1);
        assert_ne!(a, b);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pcg32_hash_2d_not_symmetric() {
        let ab = pcg32_hash_2d(1, 2);
        let ba = pcg32_hash_2d(2, 1);
        // Should differ (spatial decorrelation).
        assert_ne!(ab, ba);
    }

    // ── interleaved_gradient_noise ─────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ign_in_range() {
        for x in 0..8u32 {
            for y in 0..8u32 {
                let v = interleaved_gradient_noise(x as f32, y as f32);
                assert!((0.0..1.0).contains(&v), "x={x} y={y} v={v}");
            }
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ign_temporal_shifts_each_frame() {
        let v0 = interleaved_gradient_noise_temporal(3.0, 7.0, 0);
        let v1 = interleaved_gradient_noise_temporal(3.0, 7.0, 1);
        assert_ne!(v0, v1);
    }

    // ── gold_noise ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gold_noise_in_range() {
        for i in 0..16u32 {
            let v = gold_noise(i as f32, (i * 3) as f32, 1.0);
            assert!((0.0..1.0).contains(&v), "i={i} v={v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gold_noise_differs_by_seed() {
        let a = gold_noise(5.0, 7.0, 0.0);
        let b = gold_noise(5.0, 7.0, 1.0);
        assert!(
            (a - b).abs() > 1e-4,
            "seed should change output: {a} vs {b}"
        );
    }
}

#[cfg(test)]
mod tests_pass_40 {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    // ── rodrigues_rotation ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rodrigues_90_degrees_around_z() {
        let v = Vec3::new(1.0, 0.0, 0.0);
        let k = Vec3::new(0.0, 0.0, 1.0);
        let r = rodrigues_rotation(v, k, FRAC_PI_2);
        assert!((r.x).abs() < 1e-5, "x={}", r.x);
        assert!((r.y - 1.0).abs() < 1e-5, "y={}", r.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rodrigues_preserves_length() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let k = Vec3::new(0.0, 1.0, 0.0);
        let r = rodrigues_rotation(v, k, 1.23);
        assert!((r.length() - v.length()).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rodrigues_zero_angle_identity() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let k = Vec3::new(0.0, 0.0, 1.0);
        let r = rodrigues_rotation(v, k, 0.0);
        assert!((r.x - v.x).abs() < 1e-5);
        assert!((r.y - v.y).abs() < 1e-5);
        assert!((r.z - v.z).abs() < 1e-5);
    }

    // ── swing_twist_decompose ──────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn swing_twist_identity_gives_identity_parts() {
        let (swing, twist) = swing_twist_decompose(Quat::identity(), Vec3::new(0.0, 1.0, 0.0));
        assert!((swing.w - 1.0).abs() < 1e-5);
        assert!((twist.w - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn swing_twist_pure_twist_round_trips() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let (_swing, twist) = swing_twist_decompose(q, Vec3::new(0.0, 1.0, 0.0));
        let dot = twist.w * q.w + twist.x * q.x + twist.y * q.y + twist.z * q.z;
        assert!(dot.abs() > 0.999, "dot={dot}");
    }

    // ── refract_vec3 ──────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn refract_normal_incidence_passes_through() {
        let incident = Vec3::new(0.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let r = refract_vec3(incident, normal, 1.0).unwrap();
        assert!((r.x).abs() < 1e-5);
        assert!((r.y + 1.0).abs() < 1e-5, "y={}", r.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn refract_total_internal_reflection() {
        let incident = Vec3::new(0.99, -0.14, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        assert!(refract_vec3(incident, normal, 1.5).is_none());
    }

    // ── beer_lambert ──────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn beer_lambert_zero_distance_is_one() {
        assert!((beer_lambert(1.0, 0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn beer_lambert_known_value() {
        // exp(-0.5) ≈ 0.60653
        let t = beer_lambert(0.5, 1.0);
        assert!((t - (-0.5_f32).exp()).abs() < 1e-5, "t={t}");
    }

    // ── henyey_greenstein ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn henyey_greenstein_isotropic() {
        let expected = 1.0 / (4.0 * PI);
        for &cos_t in &[-1.0_f32, -0.5, 0.0, 0.5, 1.0] {
            let v = henyey_greenstein(cos_t, 0.0);
            assert!((v - expected).abs() < 1e-5, "cos={cos_t} v={v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn henyey_greenstein_forward_scatter_peaks_forward() {
        let forward = henyey_greenstein(1.0, 0.8);
        let back = henyey_greenstein(-1.0, 0.8);
        assert!(forward > back);
    }

    // ── gaussian_kernel_1d ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gaussian_kernel_sums_to_one() {
        let k = gaussian_kernel_1d(3, 1.0);
        let sum: f32 = k.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "sum={sum}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gaussian_kernel_correct_size() {
        assert_eq!(gaussian_kernel_1d(4, 2.0).len(), 9);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gaussian_kernel_symmetric() {
        let k = gaussian_kernel_1d(3, 1.5);
        let n = k.len();
        for i in 0..n / 2 {
            assert!((k[i] - k[n - 1 - i]).abs() < 1e-6);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gaussian_kernel_center_is_max() {
        let k = gaussian_kernel_1d(4, 1.0);
        let center = k[k.len() / 2];
        for &v in &k {
            assert!(v <= center + 1e-6);
        }
    }
}

#[cfg(test)]
mod tests_pass_41 {
    use super::*;

    // ── sobel_filter_3x3 ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sobel_flat_image_zero_gradient() {
        let pixels = [1.0_f32; 9];
        let (gx, gy) = sobel_filter_3x3(&pixels);
        assert!(gx.abs() < 1e-6, "gx={gx}");
        assert!(gy.abs() < 1e-6, "gy={gy}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sobel_vertical_edge_detects_gx() {
        // Left column = 0, right column = 1 → strong gx, weak gy.
        let pixels = [0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];
        let (gx, gy) = sobel_filter_3x3(&pixels);
        assert!(gx > 0.0, "gx should be positive: {gx}");
        assert!(gy.abs() < 1e-5, "gy should be ~0: {gy}");
    }

    // ── height_to_normal ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn height_to_normal_flat_is_up() {
        let pixels = [0.5_f32; 9];
        let n = height_to_normal(&pixels, 1.0, 1.0);
        assert!((n.z - 1.0).abs() < 1e-5, "z={}", n.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn height_to_normal_is_unit() {
        let pixels = [0.0, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 1.0];
        let n = height_to_normal(&pixels, 0.5, 2.0);
        assert!((n.length() - 1.0).abs() < 1e-5);
    }

    // ── trilinear_interp ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn trilinear_corner_values() {
        // t=0,0,0 → v000
        let v = trilinear_interp(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert!((v - 1.0).abs() < 1e-6, "v={v}");
        // t=1,1,1 → v111
        let v = trilinear_interp(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0);
        assert!((v - 1.0).abs() < 1e-6, "v={v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn trilinear_center_averages() {
        let v = trilinear_interp(0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.5, 0.5, 0.5);
        assert!((v - 0.5).abs() < 1e-5, "v={v}");
    }

    // ── mip_level ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mip_level_zero_for_tiny_derivatives() {
        let lod = mip_level(0.001, 0.001, 0.001, 0.001, 10.0);
        assert!(lod >= 0.0, "lod={lod}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn mip_level_increases_with_derivatives() {
        // LOD > 0 requires rho > 1 (i.e. derivative magnitude > 1 texel/pixel).
        let lod_small = mip_level(1.5, 0.0, 1.5, 0.0, 10.0);
        let lod_large = mip_level(4.0, 0.0, 4.0, 0.0, 10.0);
        assert!(lod_large > lod_small, "small={lod_small} large={lod_large}");
        assert!(lod_small > 0.0, "lod_small={lod_small}");
    }

    // ── contrast_adjust ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn contrast_one_is_identity() {
        for x in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            let c = contrast_adjust(x, 1.0);
            assert!((c - x).abs() < 1e-5, "x={x} c={c}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn contrast_zero_gives_midpoint() {
        for x in [0.0_f32, 0.3, 0.7, 1.0] {
            let c = contrast_adjust(x, 0.0);
            assert!((c - 0.5).abs() < 1e-5, "x={x} c={c}");
        }
    }

    // ── saturation_adjust ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn saturation_factor_one_is_identity() {
        let (r, g, b) = saturation_adjust(0.8, 0.4, 0.2, 1.0);
        assert!((r - 0.8).abs() < 1e-5);
        assert!((g - 0.4).abs() < 1e-5);
        assert!((b - 0.2).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn saturation_factor_zero_gives_greyscale() {
        let (r, g, b) = saturation_adjust(0.8, 0.4, 0.2, 0.0);
        let luma = 0.2126 * 0.8 + 0.7152 * 0.4 + 0.0722 * 0.2;
        assert!((r - luma).abs() < 1e-4);
        assert!((g - luma).abs() < 1e-4);
        assert!((b - luma).abs() < 1e-4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn saturation_output_clamped() {
        let (r, g, b) = saturation_adjust(1.0, 0.0, 0.0, 5.0);
        assert!(r <= 1.0 && r >= 0.0);
        assert!(g <= 1.0 && g >= 0.0);
        assert!(b <= 1.0 && b >= 0.0);
    }
}

#[cfg(test)]
mod tests_pass_42 {
    use super::*;

    // ── horner_eval ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn horner_constant_poly() {
        // p(x) = 7 for all x
        assert!((horner_eval(&[7.0], 3.0) - 7.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn horner_linear() {
        // p(x) = 2 + 3x; p(4) = 14
        assert!((horner_eval(&[2.0, 3.0], 4.0) - 14.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn horner_quadratic() {
        // p(x) = 1 + 2x + x²; p(3) = 16
        assert!((horner_eval(&[1.0, 2.0, 1.0], 3.0) - 16.0).abs() < 1e-5);
    }

    // ── newton_raphson ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn newton_sqrt2() {
        // f(x) = x² - 2; root = √2
        let (root, converged) = newton_raphson(|x| x * x - 2.0, |x| 2.0 * x, 1.5, 1e-6, 50);
        assert!(converged, "did not converge");
        assert!((root - 2.0_f32.sqrt()).abs() < 1e-5, "root={root}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn newton_cube_root() {
        // f(x) = x³ - 8; root = 2
        let (root, converged) = newton_raphson(|x| x * x * x - 8.0, |x| 3.0 * x * x, 1.0, 1e-6, 50);
        assert!(converged);
        assert!((root - 2.0).abs() < 1e-4, "root={root}");
    }

    // ── bisect ────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bisect_sqrt2() {
        let (root, converged) = bisect(|x| x * x - 2.0, 1.0, 2.0, 1e-6, 60);
        assert!(converged, "did not converge");
        assert!((root - 2.0_f32.sqrt()).abs() < 1e-5, "root={root}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn bisect_sin_root() {
        // sin(x) = 0 near x = π
        use std::f32::consts::PI;
        let (root, converged) = bisect(f32::sin, 3.0, 3.5, 1e-6, 60);
        assert!(converged);
        assert!((root - PI).abs() < 1e-4, "root={root}");
    }

    // ── rgb_to_yuv_bt601 / yuv_bt601_to_rgb ──────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn yuv_bt601_white_round_trip() {
        let (y, u, v) = rgb_to_yuv_bt601(1.0, 1.0, 1.0);
        assert!((y - 1.0).abs() < 1e-4, "y={y}");
        assert!(u.abs() < 1e-4, "u={u}");
        assert!(v.abs() < 1e-4, "v={v}");
        let (r, g, b) = yuv_bt601_to_rgb(y, u, v);
        assert!((r - 1.0).abs() < 1e-3);
        assert!((g - 1.0).abs() < 1e-3);
        assert!((b - 1.0).abs() < 1e-3);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn yuv_bt601_black_round_trip() {
        let (y, u, v) = rgb_to_yuv_bt601(0.0, 0.0, 0.0);
        assert!(y.abs() < 1e-6);
        let (r, g, b) = yuv_bt601_to_rgb(y, u, v);
        assert!(r.abs() < 1e-6 && g.abs() < 1e-6 && b.abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn yuv_bt601_grey_uv_zero() {
        // Neutral grey → U = V = 0
        let (_, u, v) = rgb_to_yuv_bt601(0.5, 0.5, 0.5);
        assert!(u.abs() < 1e-4, "u={u}");
        assert!(v.abs() < 1e-4, "v={v}");
    }

    // ── delta_e_cie76 ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn delta_e_identical_colours() {
        assert!(delta_e_cie76(50.0, 20.0, -10.0, 50.0, 20.0, -10.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn delta_e_known_value() {
        // ΔE between (50,0,0) and (53,0,0) = 3.0
        let de = delta_e_cie76(50.0, 0.0, 0.0, 53.0, 0.0, 0.0);
        assert!((de - 3.0).abs() < 1e-5, "de={de}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn delta_e_symmetric() {
        let d1 = delta_e_cie76(50.0, 10.0, 5.0, 55.0, 5.0, 10.0);
        let d2 = delta_e_cie76(55.0, 5.0, 10.0, 50.0, 10.0, 5.0);
        assert!((d1 - d2).abs() < 1e-5);
    }
}

#[cfg(test)]
mod tests_pass_44 {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    // ── solve_2x2 ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn solve_2x2_known() {
        // 2x + y = 5, x + 3y = 10 → x=1, y=3
        let x = solve_2x2([[2.0, 1.0], [1.0, 3.0]], [5.0, 10.0], 1e-10).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-5, "x0={}", x[0]);
        assert!((x[1] - 3.0).abs() < 1e-5, "x1={}", x[1]);
    }
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn solve_2x2_singular() {
        assert!(solve_2x2([[1.0, 2.0], [2.0, 4.0]], [1.0, 2.0], 1e-10).is_none());
    }

    // ── solve_3x3 ─────────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn solve_3x3_diagonal() {
        let a = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        let x = solve_3x3(a, [4.0, 9.0, 8.0], 1e-10).unwrap();
        assert!((x[0] - 2.0).abs() < 1e-4);
        assert!((x[1] - 3.0).abs() < 1e-4);
        assert!((x[2] - 2.0).abs() < 1e-4);
    }

    // ── gram_schmidt_3 ────────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gram_schmidt_orthonormal() {
        let (e0, e1, e2) = gram_schmidt_3(
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
        )
        .unwrap();
        assert!((e0.length() - 1.0).abs() < 1e-5);
        assert!((e1.length() - 1.0).abs() < 1e-5);
        assert!((e2.length() - 1.0).abs() < 1e-5);
        assert!(e0.dot(e1).abs() < 1e-5);
        assert!(e0.dot(e2).abs() < 1e-5);
        assert!(e1.dot(e2).abs() < 1e-5);
    }

    // ── givens_rotation ───────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn givens_zeroes_lower() {
        let (c, s) = givens_rotation(3.0, 4.0);
        let zero = -s * 3.0 + c * 4.0;
        assert!(zero.abs() < 1e-5, "zero={zero}");
    }

    // ── linear_regression_2d ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn regression_perfect_line() {
        let pts: Vec<(f32, f32)> = (0..5).map(|i| (i as f32, 2.0 * i as f32 + 1.0)).collect();
        let (m, b) = linear_regression_2d(&pts).unwrap();
        assert!((m - 2.0).abs() < 1e-4 && (b - 1.0).abs() < 1e-4);
    }

    // ── fit_plane_to_points ───────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fit_plane_xy_plane() {
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let (_, n) = fit_plane_to_points(&pts).unwrap();
        assert!(n.z.abs() > 0.99, "n.z={}", n.z);
    }

    // ── winding_number_2d ─────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn winding_number_inside_square() {
        let sq = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        assert!(winding_number_2d(Vec2::new(0.5, 0.5), &sq));
        assert!(!winding_number_2d(Vec2::new(2.0, 0.5), &sq));
    }

    // ── polygon_area_2d ───────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn polygon_area_unit_square() {
        let sq = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let area = polygon_area_2d(&sq).abs();
        assert!((area - 1.0).abs() < 1e-5, "area={area}");
    }

    // ── ray_cylinder_intersect ────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_hits_cylinder_along_x() {
        // Cylinder axis along Y through origin, radius 1. Ray from (0,0,5) toward -Z.
        let t = ray_cylinder_intersect(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, -10.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
            1.0,
        );
        assert!(t.is_some(), "expected hit");
        assert!((t.unwrap() - 4.0).abs() < 1e-4, "t={}", t.unwrap());
    }

    // ── ray_cone_intersect ────────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ray_hits_cone() {
        // Cone apex at origin, axis +Z, 45° half-angle. Ray from (0,0,5) going -Z.
        let t = ray_cone_intersect(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            FRAC_PI_4,
        );
        // Ray hits the cone where z = radius → at z=something on the 45° surface
        assert!(t.is_some(), "expected cone hit");
    }

    // ── closest_point_on_segment_3d ───────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_midpoint() {
        let p = Vec3::new(0.0, 1.0, 0.0);
        let a = Vec3::new(-1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let q = closest_point_on_segment_3d(p, a, b);
        assert!(q.x.abs() < 1e-5 && q.y.abs() < 1e-5 && q.z.abs() < 1e-5);
    }
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_point_clamps_to_endpoint() {
        let p = Vec3::new(5.0, 0.0, 0.0);
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let q = closest_point_on_segment_3d(p, a, b);
        assert!((q.x - 1.0).abs() < 1e-5);
    }

    // ── closest_segment_to_segment ────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn closest_segments_parallel() {
        let (q1, q2) = closest_segment_to_segment(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        );
        let dist = {
            let dx = q2.x - q1.x;
            let dy = q2.y - q1.y;
            let dz = q2.z - q1.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        };
        assert!((dist - 1.0).abs() < 1e-4, "dist={dist}");
    }

    // ── sphere_sphere_overlap ─────────────────────────────────────────────
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spheres_overlapping() {
        let depth =
            sphere_sphere_overlap(Vec3::new(0.0, 0.0, 0.0), 1.0, Vec3::new(1.5, 0.0, 0.0), 1.0);
        assert!(depth.is_some());
        assert!((depth.unwrap() - 0.5).abs() < 1e-5);
    }
    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spheres_not_overlapping() {
        assert!(
            sphere_sphere_overlap(Vec3::new(0.0, 0.0, 0.0), 1.0, Vec3::new(3.0, 0.0, 0.0), 1.0,)
                .is_none()
        );
    }
}

#[cfg(test)]
mod tests_pass_45 {
    use super::*;
    use std::f32::consts::PI;

    // ── exponential_smooth ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exp_smooth_alpha_one_tracks_instantly() {
        assert!((exponential_smooth(0.0, 5.0, 1.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exp_smooth_alpha_zero_no_change() {
        assert!((exponential_smooth(3.0, 10.0, 0.0) - 3.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn exp_smooth_converges() {
        let mut v = 0.0_f32;
        for _ in 0..200 {
            v = exponential_smooth(v, 1.0, 0.05);
        }
        assert!((v - 1.0).abs() < 0.01, "v={v}");
    }

    // ── one_euro_filter_step ──────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn one_euro_static_signal_converges() {
        let (mut f, mut d) = (0.0_f32, 0.0_f32);
        for _ in 0..100 {
            (f, d) = one_euro_filter_step(f, d, 1.0, 0.016, 1.0, 0.007, 1.0);
        }
        assert!((f - 1.0).abs() < 0.05, "f={f}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn one_euro_output_bounded_by_inputs() {
        // Output should stay between initial state and target for a monotone signal.
        let (f, _) = one_euro_filter_step(0.5, 0.0, 0.8, 0.016, 1.0, 0.007, 1.0);
        assert!(f >= 0.0 && f <= 1.0, "f={f}");
    }

    // ── half_life ↔ decay ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn half_life_round_trip() {
        let hl = 2.0_f32;
        let lambda = half_life_to_decay(hl);
        let hl2 = decay_to_half_life(lambda);
        assert!((hl2 - hl).abs() < 1e-5, "hl2={hl2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn half_life_decay_correct() {
        // After one half-life the value should halve.
        let hl = 1.0_f32;
        let lambda = half_life_to_decay(hl);
        let remaining = (-lambda * hl).exp();
        assert!((remaining - 0.5).abs() < 1e-5, "remaining={remaining}");
    }

    // ── smooth_damp ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_damp_converges_to_target() {
        let (mut pos, mut vel) = (0.0_f32, 0.0_f32);
        for _ in 0..300 {
            (pos, vel) = smooth_damp(pos, 10.0, vel, 10.0, 0.016);
        }
        assert!((pos - 10.0).abs() < 0.01, "pos={pos}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_damp_no_overshoot() {
        // Critically damped — should never exceed target.
        let (mut pos, mut vel) = (0.0_f32, 0.0_f32);
        let target = 1.0_f32;
        for _ in 0..500 {
            (pos, vel) = smooth_damp(pos, target, vel, 5.0, 0.016);
            assert!(pos <= target + 1e-4, "overshoot: pos={pos}");
        }
        let _ = vel;
    }

    // ── delta_angle / angular_lerp ────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn delta_angle_short_path() {
        // From 10° to 350° the short path is -20° (going backwards).
        let a = 10.0_f32.to_radians();
        let b = 350.0_f32.to_radians();
        let d = delta_angle(a, b);
        assert!((d.abs() - 20.0_f32.to_radians()).abs() < 1e-4, "d={d}");
        assert!(d < 0.0, "should be negative (backwards)");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn angular_lerp_midpoint() {
        // Lerp halfway from 0 to π/2 should give π/4.
        use std::f32::consts::FRAC_PI_4;
        let mid = angular_lerp(0.0, FRAC_PI_4 * 2.0, 0.5);
        assert!((mid - FRAC_PI_4).abs() < 1e-5, "mid={mid}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn angular_lerp_wraps() {
        // From 170° to -170° the short path is 20°; at t=0.5 should be 180°.
        let a = 170.0_f32.to_radians();
        let b = (-170.0_f32).to_radians();
        let mid = angular_lerp(a, b, 0.5);
        assert!((mid.abs() - PI).abs() < 1e-4, "mid={mid}");
    }

    // ── welford_update ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn welford_mean_correct() {
        let samples = [2.0_f32, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let (mut cnt, mut mean, mut m2) = (0, 0.0_f32, 0.0_f32);
        for &s in &samples {
            (cnt, mean, m2) = welford_update(cnt, mean, m2, s);
        }
        assert!((mean - 5.0).abs() < 1e-5, "mean={mean}");
        let _ = (cnt, m2);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn welford_variance_correct() {
        // Variance of [2,4,4,4,5,5,7,9] = 4.0 (population).
        let samples = [2.0_f32, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let (mut cnt, mut mean, mut m2) = (0, 0.0_f32, 0.0_f32);
        for &s in &samples {
            (cnt, mean, m2) = welford_update(cnt, mean, m2, s);
        }
        let variance = m2 / cnt as f32;
        assert!((variance - 4.0).abs() < 1e-4, "variance={variance}");
        let _ = mean;
    }
}

#[cfg(test)]
mod tests_pass_46 {
    use super::*;

    // ── oct_encode/decode normal ───────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oct_round_trip_poles() {
        for n in [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ] {
            let (u, v) = oct_encode_normal(n);
            let d = oct_decode_normal(u, v);
            let dot = n.x * d.x + n.y * d.y + n.z * d.z;
            assert!(dot > 0.999, "n={n:?} d={d:?} dot={dot}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn oct_decoded_is_unit() {
        let normals = [
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 0.5, 0.3),
            Vec3::new(0.0, -1.0, 0.1),
        ];
        for n_raw in normals {
            let len = n_raw.length();
            let n = Vec3::new(n_raw.x / len, n_raw.y / len, n_raw.z / len);
            let (u, v) = oct_encode_normal(n);
            let d = oct_decode_normal(u, v);
            assert!((d.length() - 1.0).abs() < 1e-5, "len={}", d.length());
        }
    }

    // ── morton_encode/decode_3d ───────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn morton_3d_origin() {
        assert_eq!(morton_encode_3d(0, 0, 0), 0);
        let (x, y, z) = morton_decode_3d(0);
        assert_eq!((x, y, z), (0, 0, 0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn morton_3d_round_trip() {
        for (x, y, z) in [(1, 2, 3), (7, 15, 31), (100, 200, 300)] {
            let code = morton_encode_3d(x, y, z);
            let (dx, dy, dz) = morton_decode_3d(code);
            assert_eq!((dx, dy, dz), (x, y, z), "at ({x},{y},{z})");
        }
    }

    // ── gray_code ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gray_code_round_trip() {
        for n in 0u32..=255 {
            assert_eq!(gray_code_decode(gray_code_encode(n)), n, "n={n}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn gray_code_adjacent_one_bit_diff() {
        for n in 0u32..=254 {
            let diff = gray_code_encode(n) ^ gray_code_encode(n + 1);
            assert!(diff.is_power_of_two(), "n={n} diff={diff:b}");
        }
    }

    // ── fibonacci_hash_u32 ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fibonacci_hash_distributes() {
        // Sequential inputs should produce well-spread outputs.
        let h0 = fibonacci_hash_u32(0);
        let h1 = fibonacci_hash_u32(1);
        let h2 = fibonacci_hash_u32(2);
        // They should all differ.
        assert_ne!(h0, h1);
        assert_ne!(h1, h2);
        assert_ne!(h0, h2);
    }

    // ── reverse_bits_u32 / van_der_corput ────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reverse_bits_involution() {
        // Applying twice gives back the original.
        for n in [0u32, 1, 0xdead_beef, 0x8000_0001, u32::MAX] {
            assert_eq!(reverse_bits_u32(reverse_bits_u32(n)), n);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reverse_bits_produces_distinct_outputs() {
        // reverse_bits_u32 on 0..8 should give 8 distinct values.
        let vals: Vec<u32> = (0..8).map(reverse_bits_u32).collect();
        let unique: std::collections::HashSet<u32> = vals.iter().copied().collect();
        assert_eq!(unique.len(), vals.len());
    }
}

#[cfg(test)]
mod tests_pass_47 {
    use super::*;
    use std::f32::consts::PI;

    // ── euler_step ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn euler_exponential_decay() {
        // y' = -y, y(0) = 1  →  y(1) ≈ e^-1 ≈ 0.368 (Euler with dt=0.01)
        let mut y = 1.0_f32;
        let dt = 0.01;
        for i in 0..100 {
            y = euler_step(i as f32 * dt, y, dt, |_t, y| -y);
        }
        assert!((y - (-1.0_f32).exp()).abs() < 0.01, "y={y}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn euler_linear_ode_exact() {
        // y' = 2t, y(0) = 0  →  y(t) = t². dt small enough to be exact.
        let mut y = 0.0_f32;
        let dt = 0.001;
        for i in 0..1000 {
            y = euler_step(i as f32 * dt, y, dt, |t, _y| 2.0 * t);
        }
        assert!((y - 1.0).abs() < 0.01, "y={y}"); // y(1) = 1² = 1
    }

    // ── runge_kutta_4 ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rk4_more_accurate_than_euler() {
        // y' = -y, y(0) = 1. RK4 should be closer to e^-1 than Euler at same dt.
        let exact = (-1.0_f32).exp();
        let dt = 0.1;
        let mut y_euler = 1.0_f32;
        let mut y_rk4 = 1.0_f32;
        for i in 0..10 {
            let t = i as f32 * dt;
            y_euler = euler_step(t, y_euler, dt, |_t, y| -y);
            y_rk4 = runge_kutta_4(t, y_rk4, dt, |_t, y| -y);
        }
        assert!(
            (y_rk4 - exact).abs() < (y_euler - exact).abs(),
            "rk4_err={} euler_err={}",
            (y_rk4 - exact).abs(),
            (y_euler - exact).abs()
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rk4_known_solution() {
        // y' = y, y(0) = 1  →  y(1) = e. Steps of dt=0.1.
        let mut y = 1.0_f32;
        let dt = 0.1;
        for i in 0..10 {
            y = runge_kutta_4(i as f32 * dt, y, dt, |_t, y| y);
        }
        assert!((y - std::f32::consts::E).abs() < 1e-4, "y={y}");
    }

    // ── verlet_step ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn verlet_free_fall() {
        // x'' = -9.8, x(0) = 0, x'(0) = 0  →  x(t) = -4.9t²
        // Initialise prev_pos for Verlet: prev = pos - v*dt + 0.5*a*dt²
        let g = -9.8_f32;
        let dt = 0.01;
        let mut pos = 0.0_f32;
        let mut prev = pos - 0.0 * dt + 0.5 * g * dt * dt; // v0=0
        for _ in 0..100 {
            (pos, prev) = verlet_step(pos, prev, g, dt);
        }
        let t = 100.0 * dt;
        let expected = 0.5 * g * t * t;
        assert!(
            (pos - expected).abs() < 0.01,
            "pos={pos} expected={expected}"
        );
    }

    // ── rayleigh_phase ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rayleigh_phase_symmetric() {
        let f = rayleigh_phase(0.5);
        let b = rayleigh_phase(-0.5);
        assert!((f - b).abs() < 1e-6, "f={f} b={b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rayleigh_phase_forward_peaks() {
        let forward = rayleigh_phase(1.0);
        let side = rayleigh_phase(0.0);
        assert!(forward > side, "forward={forward} side={side}");
    }

    // ── height_blend ──────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn height_blend_below_threshold() {
        let b = height_blend(0.0, 1.0, 0.5);
        assert!(b < 0.01, "b={b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn height_blend_above_threshold() {
        let b = height_blend(2.0, 1.0, 0.5);
        assert!(b > 0.99, "b={b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn height_blend_at_threshold_is_half() {
        let b = height_blend(1.0, 1.0, 0.5);
        assert!((b - 0.5).abs() < 1e-5, "b={b}");
    }

    // ── curl_noise_2d ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn curl_noise_nonzero() {
        let v = curl_noise_2d(Vec2::new(1.3, 2.7), 0.01);
        // Curl noise of a non-trivial point should be nonzero.
        assert!(v.x.abs() + v.y.abs() > 0.0);
    }

    // ── slope_from_heightmap ──────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slope_flat_is_zero() {
        let pixels = [1.0_f32; 9];
        let s = slope_from_heightmap(&pixels, 1.0);
        assert!(s.abs() < 1e-5, "s={s}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn slope_ramp_nonzero() {
        // Linearly increasing height across x.
        let pixels = [0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];
        let s = slope_from_heightmap(&pixels, 1.0);
        assert!(s > 0.0, "s={s}");
    }
}

#[cfg(test)]
mod tests_pass_48 {
    use super::*;

    // ── box_muller ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn box_muller_known_values() {
        // u1=exp(-0.5), u2=0 → r=1, z0=cos(0)=1, z1=sin(0)=0
        let (z0, z1) = box_muller((-0.5_f32).exp(), 0.0);
        assert!((z0 - 1.0).abs() < 1e-4, "z0={z0}");
        assert!(z1.abs() < 1e-4, "z1={z1}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn box_muller_outputs_finite() {
        for (u1, u2) in [(0.1, 0.3), (0.5, 0.5), (0.9, 0.7), (1e-6, 0.99)] {
            let (z0, z1) = box_muller(u1, u2);
            assert!(z0.is_finite() && z1.is_finite(), "u1={u1} u2={u2}");
        }
    }

    // ── sample_triangle_uniform ───────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sample_triangle_sums_to_one() {
        for (u1, u2) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (0.3, 0.7)] {
            let (w0, w1, w2) = sample_triangle_uniform(u1, u2);
            assert!((w0 + w1 + w2 - 1.0).abs() < 1e-5, "sum={}", w0 + w1 + w2);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sample_triangle_weights_nonneg() {
        for (u1, u2) in [(0.1, 0.9), (0.9, 0.1), (0.5, 0.5)] {
            let (w0, w1, w2) = sample_triangle_uniform(u1, u2);
            assert!(w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0);
        }
    }

    // ── concentric_disk_sample ────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn concentric_disk_inside_unit_disk() {
        for (u, v) in [(-0.8, 0.3), (0.5, -0.6), (0.9, 0.9), (0.0, 0.0)] {
            let (x, y) = concentric_disk_sample(u, v);
            let r2 = x * x + y * y;
            assert!(r2 <= 1.0 + 1e-5, "u={u} v={v} r²={r2}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn concentric_disk_origin_maps_to_origin() {
        let (x, y) = concentric_disk_sample(0.0, 0.0);
        assert!(x.abs() < 1e-6 && y.abs() < 1e-6);
    }

    // ── power_heuristic / balance_heuristic ───────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn power_heuristic_equal_pdfs_half() {
        let w = power_heuristic(1, 1.0, 1, 1.0);
        assert!((w - 0.5).abs() < 1e-5, "w={w}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn power_heuristic_dominant_pdf_near_one() {
        // pdf_a >> pdf_b → weight → 1
        let w = power_heuristic(1, 100.0, 1, 0.001);
        assert!(w > 0.99, "w={w}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn balance_heuristic_sums_to_one() {
        let wa = balance_heuristic(1, 2.0, 1, 3.0);
        let wb = balance_heuristic(1, 3.0, 1, 2.0);
        assert!((wa + wb - 1.0).abs() < 1e-5, "wa+wb={}", wa + wb);
    }

    // ── tent_sample ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn tent_sample_midpoint_is_zero() {
        assert!(tent_sample(0.5).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn tent_sample_endpoints() {
        assert!((tent_sample(0.0) + 1.0).abs() < 1e-5);
        assert!((tent_sample(1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn tent_sample_in_range() {
        for i in 0..=10 {
            let t = tent_sample(i as f32 / 10.0);
            assert!(t >= -1.0 && t <= 1.0, "t={t}");
        }
    }

    // ── stratified_jitter_2d ──────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn stratified_jitter_correct_count() {
        let jitter: Vec<(f32, f32)> = (0..4).map(|_| (0.5, 0.5)).collect();
        let samples = stratified_jitter_2d(2, &jitter);
        assert_eq!(samples.len(), 4);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn stratified_jitter_in_unit_square() {
        let jitter: Vec<(f32, f32)> = (0..9).map(|i| (i as f32 / 9.0, 0.5)).collect();
        for (x, y) in stratified_jitter_2d(3, &jitter) {
            assert!(
                (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
                "x={x} y={y}"
            );
        }
    }
}

#[cfg(test)]
mod tests_pass_49 {
    use super::*;

    // ── perlin_2d ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_2d_integer_coords_near_zero() {
        // Gradient noise at integer lattice points should be zero.
        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (2.0, 3.0)] {
            let v = perlin_2d(Vec2::new(x, y));
            assert!(v.abs() < 1e-5, "({x},{y}) v={v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn perlin_2d_in_range() {
        for i in 0..8 {
            let v = perlin_2d(Vec2::new(i as f32 * 0.37 + 0.1, i as f32 * 0.53 + 0.2));
            assert!(v.abs() <= 1.0, "v={v}");
        }
    }

    // ── ridged_fbm_3d ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ridged_fbm_nonnegative() {
        // Ridged noise should be non-negative (uses 1 - |noise|).
        for i in 0..8 {
            let v = ridged_fbm_3d(Vec3::new(i as f32 * 0.4, i as f32 * 0.3, 0.1), 4, 2.0, 0.5);
            assert!(v >= 0.0, "v={v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ridged_fbm_differs_from_fbm() {
        let p = Vec3::new(1.7, 2.3, 0.5);
        let ridged = ridged_fbm_3d(p, 4, 2.0, 0.5);
        let smooth = fbm_3d(p, 4, 2.0, 0.5);
        assert!((ridged - smooth).abs() > 1e-4, "should differ");
    }

    // ── billow_fbm_3d ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn billow_fbm_nonnegative() {
        for i in 0..8 {
            let v = billow_fbm_3d(Vec3::new(i as f32 * 0.4, i as f32 * 0.3, 0.1), 4, 2.0, 0.5);
            assert!(v >= 0.0, "v={v}");
        }
    }

    // ── curl_noise_3d ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn curl_noise_3d_nonzero() {
        let v = curl_noise_3d(Vec3::new(1.3, 2.7, 0.9), 0.01);
        assert!(v.x.abs() + v.y.abs() + v.z.abs() > 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn curl_noise_3d_finite() {
        let v = curl_noise_3d(Vec3::new(0.5, 1.5, 2.5), 0.001);
        assert!(v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
    }

    // ── domain_warp_2d ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn domain_warp_2d_finite() {
        let v = domain_warp_2d(Vec2::new(1.3, 2.7), 1.0, 4);
        assert!(v.is_finite(), "v={v}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn domain_warp_2d_differs_from_fbm() {
        let p = Vec2::new(1.3, 2.7);
        let warped = domain_warp_2d(p, 2.0, 4);
        let plain = fbm_2d(p, 4, 2.0, 0.5);
        assert!((warped - plain).abs() > 1e-4, "should differ");
    }

    // ── fbm_ridged_2d ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fbm_ridged_2d_nonnegative() {
        for i in 0..8 {
            let v = fbm_ridged_2d(
                Vec2::new(i as f32 * 0.4 + 0.1, i as f32 * 0.3 + 0.2),
                4,
                2.0,
                0.5,
            );
            assert!(v >= 0.0, "v={v}");
        }
    }

    // ── voronoi_smooth_2d ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn voronoi_smooth_2d_finite() {
        for i in 0..8 {
            let v = voronoi_smooth_2d(Vec2::new(i as f32 * 0.7 + 0.1, i as f32 * 0.5 + 0.3), 8.0);
            assert!(v.is_finite(), "v={v}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn voronoi_smooth_varies_with_k() {
        // Different k values should produce different outputs.
        let p = Vec2::new(0.4, 0.6);
        let v2 = voronoi_smooth_2d(p, 2.0);
        let v8 = voronoi_smooth_2d(p, 8.0);
        assert!(v2.is_finite() && v8.is_finite());
        assert!((v2 - v8).abs() > 1e-4, "should differ: v2={v2} v8={v8}");
    }
}

#[cfg(test)]
mod tests_pass_50 {
    use super::*;

    // ── finite_diff_deriv ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn finite_diff_sin_derivative() {
        // d/dx sin(x) = cos(x); f32 precision limits us to ~1e-4
        let x = 1.0_f32;
        let d = finite_diff_deriv(f32::sin, x, 1e-3);
        assert!((d - x.cos()).abs() < 1e-3, "d={d} cos={}", x.cos());
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn finite_diff_polynomial() {
        // d/dx (x³) = 3x²; at x=2 → 12
        let d = finite_diff_deriv(|x| x.powi(3), 2.0, 1e-3);
        assert!((d - 12.0).abs() < 1e-2, "d={d}");
    }

    // ── integrate_trapezoid ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn trapezoid_constant_function() {
        // ∫₀¹ 3 dx = 3
        let vals = vec![3.0_f32; 101];
        let result = integrate_trapezoid(&vals, 0.01);
        assert!((result - 3.0).abs() < 1e-4, "result={result}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn trapezoid_linear_function() {
        // ∫₀¹ x dx = 0.5
        let n = 1001;
        let vals: Vec<f32> = (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
        let result = integrate_trapezoid(&vals, 1.0 / (n - 1) as f32);
        assert!((result - 0.5).abs() < 1e-4, "result={result}");
    }

    // ── integrate_simpson ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn simpson_quadratic() {
        // ∫₀¹ x² dx = 1/3; Simpson is exact for polynomials up to degree 3.
        let n = 101;
        let vals: Vec<f32> = (0..n)
            .map(|i| {
                let x = i as f32 / (n - 1) as f32;
                x * x
            })
            .collect();
        let result = integrate_simpson(&vals, 1.0 / (n - 1) as f32);
        assert!((result - 1.0 / 3.0).abs() < 1e-5, "result={result}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn simpson_more_accurate_than_trapezoid() {
        // ∫₀^π sin(x) dx = 2.0; use a coarse grid to expose differences.
        use std::f32::consts::PI;
        let n = 11;
        let dx = PI / (n - 1) as f32;
        let vals: Vec<f32> = (0..n).map(|i| (i as f32 * dx).sin()).collect();
        let trap = integrate_trapezoid(&vals, dx);
        let simp = integrate_simpson(&vals, dx);
        assert!(
            (simp - 2.0).abs() < (trap - 2.0).abs(),
            "simp_err={} trap_err={}",
            (simp - 2.0).abs(),
            (trap - 2.0).abs()
        );
    }

    // ── convolve_1d ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn convolve_output_length() {
        let sig = [1.0_f32; 5];
        let ker = [1.0_f32; 3];
        let out = convolve_1d(&sig, &ker);
        assert_eq!(out.len(), 7);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn convolve_box_filter_sums() {
        // Signal of all-ones convolved with [1,1,1] → [1,2,3,3,3,2,1]
        let sig = [1.0_f32; 5];
        let ker = [1.0_f32; 3];
        let out = convolve_1d(&sig, &ker);
        let expected = [1.0, 2.0, 3.0, 3.0, 3.0, 2.0, 1.0];
        for (i, (&a, &b)) in out.iter().zip(expected.iter()).enumerate() {
            assert!((a - b).abs() < 1e-5, "i={i} a={a} b={b}");
        }
    }

    // ── covariance / pearson_correlation ──────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn covariance_identical_series() {
        let x = [1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let cov = covariance(&x, &x).unwrap();
        // Var(x) for [1..5] = 2.5
        assert!((cov - 2.5).abs() < 1e-5, "cov={cov}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pearson_perfect_correlation() {
        let x = [1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let y: Vec<f32> = x.iter().map(|&v| 2.0 * v + 1.0).collect();
        let r = pearson_correlation(&x, &y).unwrap();
        assert!((r - 1.0).abs() < 1e-5, "r={r}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pearson_anti_correlation() {
        let x = [1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let y: Vec<f32> = x.iter().map(|&v| -v).collect();
        let r = pearson_correlation(&x, &y).unwrap();
        assert!((r + 1.0).abs() < 1e-5, "r={r}");
    }

    // ── zero_crossings ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn zero_crossings_sine() {
        // Alternating ±1 signal: every adjacent pair is a sign flip → 4 crossings.
        // Avoids f32 sin(π)≈-8.74e-8 precision issues that make sample-based counts fragile.
        let sig = vec![1.0_f32, -1.0, 1.0, -1.0, 1.0];
        let zc = zero_crossings(&sig);
        assert_eq!(zc, 4, "zc={zc}");

        // Sanity check with a real sine-like shape: 3 bumps, 2 crossings.
        let sig2 = vec![-0.5_f32, 0.5, 0.8, 0.5, -0.5, -0.8, -0.3];
        let zc2 = zero_crossings(&sig2);
        assert_eq!(zc2, 2, "zc2={zc2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn zero_crossings_flat() {
        assert_eq!(zero_crossings(&[1.0, 1.0, 1.0, 1.0]), 0);
    }
}

#[cfg(test)]
mod tests_pass_51 {
    use super::*;

    // ── convex_hull_2d ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hull_square() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.5, 0.5), // interior point
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(hull.len(), 4, "square hull len={}", hull.len());
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hull_collinear_excluded() {
        let pts: Vec<Vec2> = (0..5).map(|i| Vec2::new(i as f32, 0.0)).collect();
        let hull = convex_hull_2d(&pts);
        assert!(hull.len() <= 2, "collinear hull len={}", hull.len());
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hull_triangle() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(1.0, 0.5), // interior
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(hull.len(), 3, "triangle hull len={}", hull.len());
    }

    // ── point_in_convex_polygon_2d ────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn point_in_unit_square_hull() {
        // CCW square.
        let hull = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        assert!(point_in_convex_polygon_2d(Vec2::new(0.5, 0.5), &hull));
        assert!(!point_in_convex_polygon_2d(Vec2::new(2.0, 0.5), &hull));
        assert!(!point_in_convex_polygon_2d(Vec2::new(-0.1, 0.5), &hull));
    }

    // ── two_bone_ik ───────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ik_end_reaches_target() {
        let root = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(1.0, 1.0, 0.0);
        let l1 = 1.0_f32;
        let l2 = 2.0_f32.sqrt();
        let hint = Vec3::new(0.0, 0.0, 1.0);
        let joint = two_bone_ik(root, l1, l2, target, hint);
        let d1 = {
            let dx = joint.x - root.x;
            let dy = joint.y - root.y;
            let dz = joint.z - root.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        };
        assert!((d1 - l1).abs() < 1e-4, "d1={d1}");
        let d2 = {
            let dx = target.x - joint.x;
            let dy = target.y - joint.y;
            let dz = target.z - joint.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        };
        assert!((d2 - l2).abs() < 1e-3, "d2={d2} l2={l2}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ik_out_of_reach_extends() {
        let root = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(10.0, 0.0, 0.0);
        let hint = Vec3::new(0.0, 1.0, 0.0);
        let joint = two_bone_ik(root, 1.0, 1.0, target, hint);
        assert!((joint.x - 1.0).abs() < 1e-4, "joint.x={}", joint.x);
        assert!(joint.y.abs() < 1e-4, "joint.y={}", joint.y);
    }

    // ── r2_sequence ───────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn r2_in_unit_square() {
        for i in 0..64 {
            let p = r2_sequence(i);
            assert!(p.x >= 0.0 && p.x < 1.0, "x={}", p.x);
            assert!(p.y >= 0.0 && p.y < 1.0, "y={}", p.y);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn r2_covers_all_quadrants() {
        let (mut q0, mut q1, mut q2, mut q3) = (0u32, 0, 0, 0);
        for i in 0..16 {
            let p = r2_sequence(i);
            match (p.x >= 0.5, p.y >= 0.5) {
                (false, false) => q0 += 1,
                (true, false) => q1 += 1,
                (false, true) => q2 += 1,
                (true, true) => q3 += 1,
            }
        }
        assert!(
            q0 >= 2 && q1 >= 2 && q2 >= 2 && q3 >= 2,
            "uneven coverage: {q0} {q1} {q2} {q3}"
        );
    }

    // ── sobol_2d ──────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sobol_in_unit_square() {
        for i in 0..64 {
            let p = sobol_2d(i);
            assert!(p.x >= 0.0 && p.x < 1.0, "x={}", p.x);
            assert!(p.y >= 0.0 && p.y < 1.0, "y={}", p.y);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sobol_first_sample_origin() {
        let p = sobol_2d(0);
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    // ── critically_damped_spring_step ─────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spring_converges_to_target() {
        let target = 5.0_f32;
        let mut pos = 0.0_f32;
        let mut vel = 0.0_f32;
        for _ in 0..200 {
            pos = critically_damped_spring_step(pos, target, &mut vel, 10.0, 0.016);
        }
        assert!((pos - target).abs() < 0.01, "pos={pos}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn spring_no_overshoot() {
        let target = 1.0_f32;
        let mut pos = 0.0_f32;
        let mut vel = 0.0_f32;
        let mut max_pos = 0.0_f32;
        for _ in 0..500 {
            pos = critically_damped_spring_step(pos, target, &mut vel, 5.0, 0.016);
            if pos > max_pos {
                max_pos = pos;
            }
        }
        assert!(max_pos <= target + 1e-3, "overshoot: max={max_pos}");
    }

    // ── bezier_arc_length_param ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn arc_param_endpoints() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(2.0, 0.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);
        let t0 = bezier_arc_length_param(p0, p1, p2, p3, 0.0, 64);
        let t1 = bezier_arc_length_param(p0, p1, p2, p3, 1.0, 64);
        assert!(t0 < 0.01, "t0={t0}");
        assert!(t1 > 0.99, "t1={t1}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn arc_param_midpoint_straight_line() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(2.0, 0.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);
        let t_mid = bezier_arc_length_param(p0, p1, p2, p3, 0.5, 128);
        assert!((t_mid - 0.5).abs() < 0.01, "t_mid={t_mid}");
    }
}

#[cfg(test)]
mod tests_pass_52 {
    use super::*;

    // ── orient_2d ─────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn orient_ccw_positive() {
        assert!(
            orient_2d(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.5, 1.0)
            ) > 0.0
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn orient_cw_negative() {
        assert!(
            orient_2d(
                Vec2::new(0.0, 0.0),
                Vec2::new(0.5, 1.0),
                Vec2::new(1.0, 0.0)
            ) < 0.0
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn orient_collinear_zero() {
        assert_eq!(
            orient_2d(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0)
            ),
            0.0
        );
    }

    // ── triangle_circumcenter_2d ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn circumcenter_right_triangle() {
        let cc = triangle_circumcenter_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 2.0),
        )
        .unwrap();
        assert!((cc.x - 1.0).abs() < 1e-5, "cc.x={}", cc.x);
        assert!((cc.y - 1.0).abs() < 1e-5, "cc.y={}", cc.y);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn circumcenter_equidistant() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(2.0, 0.0);
        let c = Vec2::new(1.0, 3.0_f32.sqrt());
        let cc = triangle_circumcenter_2d(a, b, c).unwrap();
        let ra = (cc.x - a.x)
            .mul_add(cc.x - a.x, (cc.y - a.y).powi(2))
            .sqrt();
        let rb = (cc.x - b.x)
            .mul_add(cc.x - b.x, (cc.y - b.y).powi(2))
            .sqrt();
        let rc = (cc.x - c.x)
            .mul_add(cc.x - c.x, (cc.y - c.y).powi(2))
            .sqrt();
        assert!((ra - rb).abs() < 1e-4, "ra={ra} rb={rb}");
        assert!((ra - rc).abs() < 1e-4, "ra={ra} rc={rc}");
    }

    // ── in_circumcircle_2d ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn in_circumcircle_inside() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(4.0, 0.0);
        let c = Vec2::new(2.0, 3.0);
        let d = Vec2::new(2.0, 1.0);
        assert!(in_circumcircle_2d(a, b, c, d));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn in_circumcircle_outside() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(4.0, 0.0);
        let c = Vec2::new(2.0, 3.0);
        let d = Vec2::new(2.0, 10.0);
        assert!(!in_circumcircle_2d(a, b, c, d));
    }

    // ── capsule_vs_capsule ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn capsule_overlap_parallel() {
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(2.0, 0.0, 0.0);
        let b0 = Vec3::new(0.0, 1.5, 0.0);
        let b1 = Vec3::new(2.0, 1.5, 0.0);
        // gap = 1.5, sum_r = 2.0 => overlap
        assert!(capsule_vs_capsule(a0, a1, 1.0, b0, b1, 1.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn capsule_no_overlap() {
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(1.0, 0.0, 0.0);
        let b0 = Vec3::new(0.0, 5.0, 0.0);
        let b1 = Vec3::new(1.0, 5.0, 0.0);
        // gap = 5.0, sum_r = 2.0 => no overlap
        assert!(!capsule_vs_capsule(a0, a1, 1.0, b0, b1, 1.0));
    }

    // ── sphere_vs_capsule ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_capsule_overlap() {
        let center = Vec3::new(0.0, 1.4, 0.0);
        let cap_a = Vec3::new(-1.0, 0.0, 0.0);
        let cap_b = Vec3::new(1.0, 0.0, 0.0);
        // closest point on cap to center = (0,0,0), dist=1.4 < sr+cr=1.5
        assert!(sphere_vs_capsule(center, 0.5, cap_a, cap_b, 1.0));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_capsule_no_overlap() {
        let center = Vec3::new(0.0, 3.0, 0.0);
        let cap_a = Vec3::new(-1.0, 0.0, 0.0);
        let cap_b = Vec3::new(1.0, 0.0, 0.0);
        assert!(!sphere_vs_capsule(center, 0.5, cap_a, cap_b, 1.0));
    }

    // ── sat_overlap_polygons_2d ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sat_overlapping_squares() {
        let sq_a = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        let sq_b = vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(3.0, 1.0),
            Vec2::new(3.0, 3.0),
            Vec2::new(1.0, 3.0),
        ];
        assert!(sat_overlap_polygons_2d(&sq_a, &sq_b));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sat_separated_squares() {
        let sq_a = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let sq_b = vec![
            Vec2::new(3.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 1.0),
            Vec2::new(3.0, 1.0),
        ];
        assert!(!sat_overlap_polygons_2d(&sq_a, &sq_b));
    }

    // ── obb_vs_obb_2d ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn obb_axis_aligned_overlap() {
        assert!(obb_vs_obb_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            0.0,
            Vec2::new(1.5, 0.0),
            Vec2::new(1.0, 1.0),
            0.0,
        ));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn obb_axis_aligned_separated() {
        assert!(!obb_vs_obb_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            0.0,
            Vec2::new(5.0, 0.0),
            Vec2::new(1.0, 1.0),
            0.0,
        ));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn obb_rotated_overlap() {
        use std::f32::consts::FRAC_PI_4;
        assert!(obb_vs_obb_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            FRAC_PI_4,
            Vec2::new(1.2, 0.0),
            Vec2::new(1.0, 1.0),
            FRAC_PI_4,
        ));
    }
}

#[cfg(test)]
mod tests_pass_53 {
    use super::*;

    // ── sdf_sphere ────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_surface_zero() {
        let p = Vec3::new(1.0, 0.0, 0.0);
        assert!((sdf_sphere(p, 1.0)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_inside_negative() {
        let p = Vec3::new(0.0, 0.0, 0.0);
        assert!(sdf_sphere(p, 1.0) < 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sphere_outside_positive() {
        let p = Vec3::new(2.0, 0.0, 0.0);
        assert!(sdf_sphere(p, 1.0) > 0.0);
        assert!((sdf_sphere(p, 1.0) - 1.0).abs() < 1e-6);
    }

    // ── sdf_box_3d ────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn box_surface_face() {
        // Point on +X face of unit cube.
        let p = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 1.0);
        assert!(sdf_box_3d(p, b).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn box_inside_negative() {
        let p = Vec3::new(0.5, 0.5, 0.5);
        let b = Vec3::new(1.0, 1.0, 1.0);
        assert!(sdf_box_3d(p, b) < 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn box_outside_positive_corner() {
        let p = Vec3::new(2.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 1.0);
        assert!((sdf_box_3d(p, b) - 1.0).abs() < 1e-5);
    }

    // ── sdf_torus ─────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn torus_on_surface() {
        // Point at (R+r, 0, 0) lies on torus surface.
        let r_maj = 2.0_f32;
        let r_min = 0.5_f32;
        let p = Vec3::new(r_maj + r_min, 0.0, 0.0);
        assert!(sdf_torus(p, r_maj, r_min).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn torus_inside_negative() {
        let p = Vec3::new(2.0, 0.0, 0.0); // in the tube hole, inside torus
        assert!(sdf_torus(p, 2.0, 0.5) < 0.0);
    }

    // ── sdf_capsule_3d ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn capsule_on_side_surface() {
        let a = Vec3::new(0.0, -1.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let p = Vec3::new(0.5, 0.0, 0.0); // beside mid-point
        assert!((sdf_capsule_3d(p, a, b, 0.5)).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn capsule_at_endcap() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 2.0, 0.0);
        let p = Vec3::new(0.0, 2.5, 0.0); // above end-cap
        assert!((sdf_capsule_3d(p, a, b, 0.5)).abs() < 1e-6);
    }

    // ── sdf_cylinder ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cylinder_on_surface() {
        let p = Vec3::new(1.0, 5.0, 0.0); // on surface, any Y
        assert!(sdf_cylinder(p, 1.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cylinder_inside_negative() {
        let p = Vec3::new(0.5, 0.0, 0.0);
        assert!(sdf_cylinder(p, 1.0) < 0.0);
    }

    // ── sdf_cylinder_finite ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cylinder_finite_inside() {
        let a = Vec3::new(0.0, -1.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let p = Vec3::new(0.0, 0.0, 0.0); // dead center
        assert!(sdf_cylinder_finite(p, a, b, 1.0) < 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cylinder_finite_outside_cap() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 2.0, 0.0);
        let p = Vec3::new(0.0, 3.0, 0.0); // above top cap
        assert!(sdf_cylinder_finite(p, a, b, 1.0) > 0.0);
    }

    // ── sdf_op_union / subtract / intersect ───────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn op_union_picks_min() {
        assert_eq!(sdf_op_union(0.5, -0.3), -0.3);
        assert_eq!(sdf_op_union(-1.0, -2.0), -2.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn op_subtract_removes_b() {
        // Point inside b (b < 0) is cut from a.
        let a = -0.5_f32; // inside sphere A
        let b = -0.3_f32; // also inside sphere B
        // After subtracting B from A, result should be positive (outside result).
        assert!(sdf_op_subtract(a, b) > 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn op_intersect_picks_max() {
        assert_eq!(sdf_op_intersect(0.5, 0.3), 0.5);
        assert_eq!(sdf_op_intersect(-1.0, 0.2), 0.2);
    }

    // ── sdf_op_smooth_union ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_union_blends_near_boundary() {
        let a = 0.1_f32;
        let b = -0.1_f32;
        let blended = sdf_op_smooth_union(a, b, 0.5);
        // Should be between sharp min (-0.1) and 0.
        assert!(blended <= 0.0, "blended={blended}");
        assert!(blended >= -0.5, "blended={blended}");
    }

    // ── sdf_op_round / onion ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn op_round_inflates() {
        let d = sdf_box_3d(Vec3::new(1.5, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let rounded = sdf_op_round(d, 0.3);
        // Rounded box is larger, so same point is closer to its surface.
        assert!(rounded < d);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn op_onion_creates_shell() {
        // Inside solid sphere: d = -0.5
        let d = sdf_sphere(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let shell = sdf_op_onion(d, 0.1);
        // Onion should be positive (outside shell) since d=-1.0 and |-1.0|-0.1=0.9 > 0.
        assert!(shell > 0.0);
    }

    // ── sdf_op_repeat_3d ──────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn repeat_maps_to_cell() {
        let c = Vec3::new(2.0, 2.0, 2.0);
        let p = Vec3::new(5.0, 5.0, 5.0);
        let q = sdf_op_repeat_3d(p, c);
        // 5 mod 2 centered: 5 - 2*round(5/2) = 5 - 2*round(2.5) = 5 - 2*3 = -1.
        // So q should be in [-1, 1]^3.
        assert!(q.x.abs() <= 1.0 + 1e-5, "q.x={}", q.x);
        assert!(q.y.abs() <= 1.0 + 1e-5, "q.y={}", q.y);
        assert!(q.z.abs() <= 1.0 + 1e-5, "q.z={}", q.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn repeat_origin_unchanged() {
        let c = Vec3::new(4.0, 4.0, 4.0);
        let p = Vec3::new(0.0, 0.0, 0.0);
        let q = sdf_op_repeat_3d(p, c);
        assert!(q.x.abs() < 1e-6);
        assert!(q.y.abs() < 1e-6);
        assert!(q.z.abs() < 1e-6);
    }
}

#[cfg(test)]
mod tests_pass_54 {
    use super::*;

    // ── lowbias32 ─────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lowbias32_distinct_inputs() {
        assert_ne!(lowbias32(0), lowbias32(1));
        assert_ne!(lowbias32(100), lowbias32(101));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn lowbias32_nonzero_for_one() {
        // 0 is a fixed point (XOR-multiply property); test non-zero inputs.
        assert_ne!(lowbias32(1), 0);
        assert_ne!(lowbias32(1), 1);
    }

    // ── murmur3_fmix32 ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn murmur3_known_value() {
        // 0 is a fixed point of XOR-multiply hashes; test non-zero inputs.
        assert_ne!(murmur3_fmix32(1), 0);
        assert_ne!(murmur3_fmix32(1), 1);
        assert_eq!(murmur3_fmix32(42), murmur3_fmix32(42));
    }

    // ── hash_to_unit_vec3 ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hash_vec3_on_unit_sphere() {
        for seed in 0..16 {
            let v = hash_to_unit_vec3(seed);
            let len_sq = v.x * v.x + v.y * v.y + v.z * v.z;
            assert!((len_sq - 1.0).abs() < 1e-5, "seed={seed} len_sq={len_sq}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hash_vec3_diverse() {
        // Different seeds should produce different vectors.
        let v0 = hash_to_unit_vec3(0);
        let v1 = hash_to_unit_vec3(1);
        let dot = v0.x * v1.x + v0.y * v1.y + v0.z * v1.z;
        assert!((dot - 1.0).abs() > 1e-3, "v0 and v1 too similar, dot={dot}");
    }

    // ── xyz_to_linear_rgb (existing function) ────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn xyz_rgb_round_trip() {
        // Round-trip: linear_rgb_to_xyz (existing) then xyz_to_linear_rgb (existing).
        let (r0, g0, b0) = (0.5_f32, 0.3, 0.8);
        let (x, y, z) = linear_rgb_to_xyz(r0, g0, b0);
        let (r1, g1, b1) = xyz_to_linear_rgb(x, y, z);
        assert!((r0 - r1).abs() < 1e-4, "r: {r0} vs {r1}");
        assert!((g0 - g1).abs() < 1e-4, "g: {g0} vs {g1}");
        assert!((b0 - b1).abs() < 1e-4, "b: {b0} vs {b1}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn xyz_d65_white_is_white() {
        // D65 white point in XYZ: approximately (0.9505, 1.0, 1.089).
        let (r, g, b) = xyz_to_linear_rgb(0.9505, 1.0, 1.089);
        assert!((r - 1.0).abs() < 0.01, "r={r}");
        assert!((g - 1.0).abs() < 0.01, "g={g}");
        assert!((b - 1.0).abs() < 0.01, "b={b}");
    }

    // ── blackbody_linear_rgb ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blackbody_warm_is_reddish() {
        let (r, g, b) = blackbody_linear_rgb(2000.0);
        // Warm temperature: more red than blue.
        assert!(r > b, "r={r} b={b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blackbody_cool_is_bluish() {
        let (r, g, b) = blackbody_linear_rgb(12000.0);
        // Cool temperature: more blue (or equal) than red.
        assert!(b >= r, "r={r} b={b}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blackbody_finite_values() {
        for t in [1700.0, 3000.0, 6500.0, 10000.0, 20000.0] {
            let (r, g, b) = blackbody_linear_rgb(t);
            assert!(r.is_finite() && g.is_finite() && b.is_finite(), "t={t}");
        }
    }

    // ── hilbert_xy_to_d / hilbert_d_to_xy ────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hilbert_round_trip() {
        let n = 4u32; // 16x16 grid
        for d in 0..256 {
            let (x, y) = hilbert_d_to_xy(d, n);
            let d2 = hilbert_xy_to_d(x, y, n);
            assert_eq!(d, d2, "d={d} -> ({x},{y}) -> {d2}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hilbert_origin_is_zero() {
        assert_eq!(hilbert_xy_to_d(0, 0, 4), 0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hilbert_covers_all_cells() {
        let n = 3u32; // 8x8 = 64 cells
        let mut seen = vec![false; 64];
        for d in 0..64 {
            let (x, y) = hilbert_d_to_xy(d, n);
            let idx = (y * 8 + x) as usize;
            assert!(!seen[idx], "duplicate cell ({x},{y}) at d={d}");
            seen[idx] = true;
        }
        assert!(seen.iter().all(|&v| v), "not all cells covered");
    }
}

#[cfg(test)]
mod tests_pass_55 {
    use super::*;

    // ── hann_window ───────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hann_zero_at_endpoints() {
        let n = 16;
        assert!(hann_window(n, 0).abs() < 1e-6, "w[0]={}", hann_window(n, 0));
        // Periodic form: w[n] = w[0] = 0 (not actually evaluated, but w[n-1] != 0).
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hann_peak_at_centre() {
        let n = 16;
        let mid = n / 2;
        let w_mid = hann_window(n, mid);
        for i in 0..n {
            assert!(hann_window(n, i) <= w_mid + 1e-6, "i={i}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hann_in_range() {
        for i in 0..64 {
            let w = hann_window(64, i);
            assert!(w >= 0.0 && w <= 1.0, "i={i} w={w}");
        }
    }

    // ── hamming_window ────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hamming_in_range() {
        for i in 0..64 {
            let w = hamming_window(64, i);
            assert!(w >= 0.0 && w <= 1.0, "i={i} w={w}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hamming_nonzero_at_endpoints() {
        // Hamming never reaches 0 (minimum is ~0.08).
        let w0 = hamming_window(16, 0);
        assert!(w0 > 0.0 && w0 < 0.2, "w[0]={w0}");
    }

    // ── blackman_window ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blackman_in_range() {
        for i in 0..64 {
            let w = blackman_window(64, i);
            assert!(w >= -0.01 && w <= 1.0, "i={i} w={w}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn blackman_larger_sidelobe_suppression() {
        // Blackman at endpoint approaches 0 more than Hamming.
        let bw = blackman_window(64, 0).abs();
        let hw = hamming_window(64, 0).abs();
        assert!(bw < hw, "bw={bw} hw={hw}");
    }

    // ── apply_window ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn apply_window_hann_zeroes_first_sample() {
        let signal = vec![1.0_f32; 16];
        let windowed = apply_window(&signal, hann_window);
        assert!(windowed[0].abs() < 1e-6, "w[0]={}", windowed[0]);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn apply_window_length_preserved() {
        let signal = vec![1.0_f32; 32];
        let windowed = apply_window(&signal, hamming_window);
        assert_eq!(windowed.len(), 32);
    }

    // ── rms ───────────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rms_constant_signal() {
        let signal = vec![3.0_f32; 100];
        assert!((rms(&signal) - 3.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rms_sine_half_amplitude() {
        use std::f32::consts::TAU;
        // RMS of sin is 1/sqrt(2).
        let n = 1024;
        let sig: Vec<f32> = (0..n).map(|i| (TAU * i as f32 / n as f32).sin()).collect();
        let expected = 1.0_f32 / 2.0_f32.sqrt();
        assert!((rms(&sig) - expected).abs() < 0.001, "rms={}", rms(&sig));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rms_empty_is_zero() {
        assert_eq!(rms(&[]), 0.0);
    }

    // ── dct_ii / idct_ii ──────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn dct_round_trip() {
        let original = vec![1.0_f32, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0, 0.0];
        let coeffs = dct_ii(&original);
        let recovered = idct_ii(&coeffs);
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-4, "orig={a} rec={b}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn dct_constant_signal_dc_only() {
        // Constant signal: only DC coefficient (k=0) is non-zero.
        let n = 8;
        let sig = vec![2.0_f32; n];
        let coeffs = dct_ii(&sig);
        // DC = sum of all samples.
        assert!(
            (coeffs[0] - 2.0 * n as f32).abs() < 1e-4,
            "dc={}",
            coeffs[0]
        );
        for k in 1..n {
            assert!(coeffs[k].abs() < 1e-4, "k={k} coeff={}", coeffs[k]);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn dct_empty_returns_empty() {
        assert!(dct_ii(&[]).is_empty());
        assert!(idct_ii(&[]).is_empty());
    }

    // ── sdf_normal_3d ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sdf_normal_sphere_points_outward() {
        // On the surface of a unit sphere at (1,0,0), normal should point +X.
        let p = Vec3::new(1.0, 0.0, 0.0);
        let n = sdf_normal_3d(p, |q| sdf_sphere(q, 1.0), 1e-3);
        assert!((n.x - 1.0).abs() < 0.01, "n.x={}", n.x);
        assert!(n.y.abs() < 0.01, "n.y={}", n.y);
        assert!(n.z.abs() < 0.01, "n.z={}", n.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sdf_normal_is_unit_length() {
        let p = Vec3::new(0.7, 0.5, 0.2);
        let n = sdf_normal_3d(p, |q| sdf_sphere(q, 1.0), 1e-3);
        let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
        assert!((len - 1.0).abs() < 1e-4, "len={len}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn sdf_normal_box_face() {
        // On +Y face of a box, normal should point +Y.
        let p = Vec3::new(0.0, 1.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 1.0);
        let n = sdf_normal_3d(p, |q| sdf_box_3d(q, b), 1e-3);
        assert!((n.y - 1.0).abs() < 0.01, "n.y={}", n.y);
    }
}

#[cfg(test)]
mod tests_pass_56 {
    use super::*;

    // ── perspective_reverse_z ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reverse_z_near_maps_to_one() {
        use std::f32::consts::FRAC_PI_2;
        let proj = perspective_reverse_z(FRAC_PI_2, 1.0, 0.1);
        // A point at z = -near (in view space) should produce NDC z = 1.
        let near = 0.1_f32;
        let p = [0.0_f32, 0.0, -near, 1.0];
        let mut clip = [0.0f32; 4];
        for row in 0..4 {
            clip[row] = (0..4).map(|col| proj.m[col][row] * p[col]).sum();
        }
        let ndc_z = clip[2] / clip[3];
        assert!((ndc_z - 1.0).abs() < 1e-5, "ndc_z={ndc_z}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reverse_z_far_approaches_zero() {
        use std::f32::consts::FRAC_PI_2;
        let proj = perspective_reverse_z(FRAC_PI_2, 1.0, 0.1);
        // Very far point should approach z = 0.
        let p = [0.0_f32, 0.0, -1_000_000.0, 1.0];
        let mut clip = [0.0f32; 4];
        for row in 0..4 {
            clip[row] = (0..4).map(|col| proj.m[col][row] * p[col]).sum();
        }
        let ndc_z = clip[2] / clip[3];
        assert!(ndc_z.abs() < 0.01, "ndc_z={ndc_z}");
    }

    // ── taa_halton_jitter ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn taa_jitter_in_half_pixel_range() {
        for frame in 0..16 {
            let j = taa_halton_jitter(frame, 1920, 1080);
            assert!(j.x.abs() <= 0.5 / 1920.0 + 1e-6, "frame={frame} jx={}", j.x);
            assert!(j.y.abs() <= 0.5 / 1080.0 + 1e-6, "frame={frame} jy={}", j.y);
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn taa_jitter_varies_per_frame() {
        let j0 = taa_halton_jitter(0, 1920, 1080);
        let j1 = taa_halton_jitter(1, 1920, 1080);
        assert!(j0.x != j1.x || j0.y != j1.y);
    }

    // ── f32_to_f16 / f16_to_f32 ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn half_float_round_trip_common_values() {
        for &v in &[0.0_f32, 1.0, -1.0, 0.5, 2.0, 100.0, -0.25] {
            let h = f32_to_f16(v);
            let back = f16_to_f32(h);
            // Half-float has ~3 decimal digits of precision.
            let rel_err = if v == 0.0 {
                back.abs()
            } else {
                ((back - v) / v).abs()
            };
            assert!(rel_err < 1e-3, "v={v} h={h:#06x} back={back}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn half_float_zero_roundtrip() {
        assert_eq!(f32_to_f16(0.0), 0x0000);
        assert_eq!(f16_to_f32(0x0000), 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn half_float_inf_preserved() {
        let h = f32_to_f16(f32::INFINITY);
        assert_eq!(f16_to_f32(h), f32::INFINITY);
        let hn = f32_to_f16(f32::NEG_INFINITY);
        assert_eq!(f16_to_f32(hn), f32::NEG_INFINITY);
    }

    // ── cascade_shadow_splits ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cascade_splits_endpoints() {
        let splits = cascade_shadow_splits(0.1, 100.0, 4, 0.5);
        assert_eq!(splits.len(), 5);
        assert!((splits[0] - 0.1).abs() < 1e-6);
        assert!((splits[4] - 100.0).abs() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cascade_splits_monotone() {
        let splits = cascade_shadow_splits(0.1, 200.0, 4, 0.7);
        for w in splits.windows(2) {
            assert!(w[1] > w[0], "non-monotone: {:.4} >= {:.4}", w[0], w[1]);
        }
    }

    // ── reconstruct_normal_z ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reconstruct_upward_normal() {
        // Normal pointing straight up: xy = (0, 0), z = 1.
        let n = reconstruct_normal_z(Vec2::new(0.0, 0.0));
        assert!((n.z - 1.0).abs() < 1e-6, "n.z={}", n.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reconstruct_normal_is_unit() {
        let n = reconstruct_normal_z(Vec2::new(0.5, 0.3));
        let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "len={len}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn reconstruct_normal_clamps_oob() {
        // XY magnitude > 1 is out of range — z should be clamped to 0.
        let n = reconstruct_normal_z(Vec2::new(0.9, 0.9));
        assert!(n.z >= 0.0, "z negative: {}", n.z);
    }
}

#[cfg(test)]
mod tests_pass_57 {
    use super::*;

    // ── ev100 / ev100_to_exposure ─────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ev100_standard_daylight() {
        // Sunny 16 rule: f/16, 1/100s, ISO 100 => EV100 ~ 15.
        let ev = ev100(16.0, 1.0 / 100.0, 100.0);
        assert!((ev - 14.0).abs() < 1.5, "ev={ev}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ev100_higher_iso_lowers_ev() {
        let ev_100 = ev100(2.8, 1.0 / 60.0, 100.0);
        let ev_800 = ev100(2.8, 1.0 / 60.0, 800.0);
        assert!(ev_800 < ev_100, "ev_100={ev_100} ev_800={ev_800}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ev100_to_exposure_positive() {
        let exp = ev100_to_exposure(12.0);
        assert!(exp > 0.0 && exp < 1.0, "exp={exp}");
    }

    // ── log_average_luminance ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn log_average_constant_field() {
        let lums = vec![1.0_f32; 100];
        let avg = log_average_luminance(&lums);
        assert!((avg - 1.0).abs() < 0.01, "avg={avg}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn log_average_empty_is_zero() {
        assert_eq!(log_average_luminance(&[]), 0.0);
    }

    // ── cardinal_spline ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cardinal_tension_zero_matches_catmull_rom() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(2.0, 1.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);
        let card = cardinal_spline(p0, p1, p2, p3, 0.5, 0.0);
        let cr = catmull_rom(p0, p1, p2, p3, 0.5);
        assert!(
            (card.x - cr.x).abs() < 1e-5,
            "x: card={} cr={}",
            card.x,
            cr.x
        );
        assert!(
            (card.y - cr.y).abs() < 1e-5,
            "y: card={} cr={}",
            card.y,
            cr.y
        );
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn cardinal_endpoints_pass_through_p1_p2() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 2.0, 0.0);
        let p2 = Vec3::new(3.0, 1.0, 0.0);
        let p3 = Vec3::new(4.0, 0.0, 0.0);
        let at_0 = cardinal_spline(p0, p1, p2, p3, 0.0, 0.5);
        let at_1 = cardinal_spline(p0, p1, p2, p3, 1.0, 0.5);
        assert!((at_0.x - p1.x).abs() < 1e-5);
        assert!((at_1.x - p2.x).abs() < 1e-5);
    }

    // ── quat_look_at ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn look_at_forward_z_is_identity() {
        let q = quat_look_at(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0));
        // Rotating +Z by identity should still give +Z.
        let z = Vec3::new(0.0, 0.0, 1.0);
        let rotated = q.rotate(z);
        assert!((rotated.x).abs() < 0.01, "x={}", rotated.x);
        assert!((rotated.y).abs() < 0.01, "y={}", rotated.y);
        assert!((rotated.z - 1.0).abs() < 0.01, "z={}", rotated.z);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn look_at_returns_unit_quat() {
        let q = quat_look_at(Vec3::new(1.0, 0.5, 0.3), Vec3::new(0.0, 1.0, 0.0));
        let len = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "len={len}");
    }

    // ── quat_nlerp_weighted ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn nlerp_single_quat_returns_self() {
        let q = Quat::identity();
        let result = quat_nlerp_weighted(&[q], &[1.0]);
        assert!((result.w - q.w).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn nlerp_equal_weights_midpoint() {
        let q0 = Quat::identity();
        // 90-degree rotation around Y.
        let half = (std::f32::consts::FRAC_PI_4).sin();
        let q1 = Quat {
            w: half,
            x: 0.0,
            y: half,
            z: 0.0,
        };
        let blended = quat_nlerp_weighted(&[q0, q1], &[1.0, 1.0]);
        let len = (blended.x * blended.x
            + blended.y * blended.y
            + blended.z * blended.z
            + blended.w * blended.w)
            .sqrt();
        assert!((len - 1.0).abs() < 1e-5, "len={len}");
    }

    // ── damped_oscillator_state ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn underdamped_oscillates() {
        // Underdamped: position should cross zero at some point.
        let mut crossed = false;
        let mut prev_x = 1.0_f32;
        for i in 1..100 {
            let t = i as f32 * 0.05;
            let (x, _) = damped_oscillator_state(5.0, 0.1, 1.0, 0.0, t);
            if prev_x * x < 0.0 {
                crossed = true;
                break;
            }
            prev_x = x;
        }
        assert!(crossed, "underdamped should oscillate through zero");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn critically_damped_no_overshoot() {
        // Critically damped from x0=1, v0=0: should decay monotonically.
        let mut max_x = 1.0_f32;
        let mut prev_x = 1.0_f32;
        for i in 1..200 {
            let t = i as f32 * 0.01;
            let (x, _) = damped_oscillator_state(5.0, 1.0, 1.0, 0.0, t);
            if x > max_x {
                max_x = x;
            }
            prev_x = x;
        }
        let _ = prev_x;
        assert!(max_x <= 1.0 + 1e-4, "overdamped overshoot: max_x={max_x}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn overdamped_decays_to_zero() {
        let (x, _) = damped_oscillator_state(5.0, 2.0, 1.0, 0.0, 5.0);
        assert!(x.abs() < 0.01, "overdamped should decay: x={x}");
    }
}

#[cfg(test)]
mod tests_pass_58 {
    use super::*;

    // ── sdf_ellipsoid ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ellipsoid_origin_is_negative() {
        let d = sdf_ellipsoid(Vec3::ZERO, Vec3::new(1.0, 0.5, 2.0));
        assert!(d < 0.0, "d={d}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ellipsoid_far_point_is_positive() {
        let d = sdf_ellipsoid(Vec3::new(10.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(d > 0.0, "d={d}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ellipsoid_on_surface_near_zero() {
        // Unit-sphere special case — point on surface should be ~0.
        let p = Vec3::new(1.0, 0.0, 0.0);
        let r = Vec3::new(1.0, 1.0, 1.0);
        let d = sdf_ellipsoid(p, r);
        assert!(d.abs() < 1e-4, "d={d}");
    }

    // ── sdf_hex_prism ─────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hex_prism_centre_inside() {
        let d = sdf_hex_prism(Vec3::ZERO, Vec2::new(1.0, 1.0));
        assert!(d < 0.0, "d={d}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn hex_prism_far_above_outside() {
        let d = sdf_hex_prism(Vec3::new(0.0, 5.0, 0.0), Vec2::new(1.0, 1.0));
        assert!(d > 0.0, "d={d}");
    }

    // ── sdf_pyramid ───────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pyramid_centre_inside() {
        // Pyramid tip at y=+h=1.0, base at y=0.  Mid-height centre is inside.
        let d = sdf_pyramid(Vec3::new(0.0, 0.5, 0.0), 1.0);
        assert!(d < 0.0, "d={d}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn pyramid_far_outside() {
        let d = sdf_pyramid(Vec3::new(0.0, 10.0, 0.0), 1.0);
        assert!(d > 0.0, "d={d}");
    }

    // ── sdf_op_smooth_subtract ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_subtract_recovers_hard_subtract_at_k0() {
        let a = 0.3_f32;
        let b = -0.5_f32;
        let soft = sdf_op_smooth_subtract(a, b, 1e-6);
        let hard = a.max(-b);
        assert!((soft - hard).abs() < 1e-3, "soft={soft} hard={hard}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_subtract_blends_near_boundary() {
        // Near the boundary the smooth version adds a blend offset (>= hard).
        // Far from the blend zone (|a+b| >> k) it converges to hard subtract.
        let a = 0.5_f32;
        let b = 0.8_f32; // deep inside B — well outside blend radius k=0.1
        let soft = sdf_op_smooth_subtract(a, b, 0.1);
        let hard = a.max(-b);
        assert!((soft - hard).abs() < 1e-4, "soft={soft} hard={hard}");
    }

    // ── sdf_op_smooth_intersect ───────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_intersect_recovers_hard_intersect_at_k0() {
        let a = 0.3_f32;
        let b = 0.5_f32;
        let soft = sdf_op_smooth_intersect(a, b, 1e-6);
        let hard = a.max(b);
        assert!((soft - hard).abs() < 1e-3, "soft={soft} hard={hard}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smooth_intersect_smaller_than_hard_near_boundary() {
        let a = 0.05_f32;
        let b = 0.05_f32;
        let soft = sdf_op_smooth_intersect(a, b, 0.5);
        let hard = a.max(b);
        assert!(soft <= hard + 1e-5, "soft={soft} hard={hard}");
    }

    // ── sdf_op_elongate ───────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elongate_identity_when_h_zero() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        let pe = sdf_op_elongate(p, Vec3::ZERO);
        assert!((pe - p).length() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn elongate_clamps_inside_h() {
        // p.x=0.3 inside h.x=0.5 → result x=0, sphere sees origin on that axis.
        let p = Vec3::new(0.3, 0.0, 0.0);
        let pe = sdf_op_elongate(p, Vec3::new(0.5, 0.0, 0.0));
        assert!(pe.x.abs() < 1e-6, "pe.x={}", pe.x);
    }

    // ── sdf_op_twist ──────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn twist_zero_k_is_identity() {
        let p = Vec3::new(1.0, 2.0, 0.5);
        let pt = sdf_op_twist(p, 0.0);
        assert!((pt - p).length() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn twist_preserves_y_and_radius() {
        let p = Vec3::new(1.0, 1.0, 0.0);
        let pt = sdf_op_twist(p, std::f32::consts::FRAC_PI_2);
        assert!((pt.y - p.y).abs() < 1e-5);
        let r_in = p.x.mul_add(p.x, p.z * p.z).sqrt();
        let r_out = pt.x.mul_add(pt.x, pt.z * pt.z).sqrt();
        assert!((r_in - r_out).abs() < 1e-5);
    }

    // ── barrel_distortion ─────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn barrel_no_distortion_at_origin() {
        let uv = Vec2::ZERO;
        let out = barrel_distortion(uv, 0.3);
        assert!((out - uv).length() < 1e-6);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn barrel_positive_k_expands() {
        let uv = Vec2::new(0.5, 0.5);
        let out = barrel_distortion(uv, 0.3);
        assert!(out.length() > uv.length(), "out={out:?}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn barrel_negative_k_contracts() {
        let uv = Vec2::new(0.5, 0.5);
        let out = barrel_distortion(uv, -0.3);
        assert!(out.length() < uv.length(), "out={out:?}");
    }
}

#[cfg(test)]
mod tests_pass_59 {
    use super::*;

    // ── fog ───────────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fog_linear_clear_at_start() {
        assert!((fog_factor_linear(0.0, 0.0, 100.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fog_linear_full_at_end() {
        assert!(fog_factor_linear(100.0, 0.0, 100.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fog_linear_clamped_beyond_end() {
        assert!(fog_factor_linear(200.0, 0.0, 100.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fog_exp_one_at_zero_dist() {
        assert!((fog_factor_exp(0.0, 0.05) - 1.0).abs() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fog_exp_decays_with_distance() {
        assert!(fog_factor_exp(100.0, 0.05) < fog_factor_exp(50.0, 0.05));
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn fog_exp2_one_at_zero_dist() {
        assert!((fog_factor_exp2(0.0, 0.05) - 1.0).abs() < 1e-5);
    }

    // ── solid_angle_sphere ────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn solid_angle_hemisphere_at_surface() {
        // Observer exactly on sphere surface: solid angle = 2π.
        let sa = solid_angle_sphere(1.0, 1.0);
        assert!((sa - 2.0 * std::f32::consts::PI).abs() < 1e-4, "sa={sa}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn solid_angle_decreases_with_distance() {
        let sa_near = solid_angle_sphere(1.0, 2.0);
        let sa_far = solid_angle_sphere(1.0, 10.0);
        assert!(sa_near > sa_far, "near={sa_near} far={sa_far}");
    }

    // ── projectile_position ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn projectile_at_t0_is_pos0() {
        let pos = projectile_position(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(5.0, 0.0, 0.0),
            0.0,
            Vec3::new(0.0, -9.81, 0.0),
        );
        assert!((pos - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-5);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn projectile_falls_under_gravity() {
        let pos1 = projectile_position(Vec3::ZERO, Vec3::ZERO, 1.0, Vec3::new(0.0, -9.81, 0.0));
        let pos2 = projectile_position(Vec3::ZERO, Vec3::ZERO, 2.0, Vec3::new(0.0, -9.81, 0.0));
        assert!(pos2.y < pos1.y, "pos1.y={} pos2.y={}", pos1.y, pos2.y);
    }

    // ── normal_map_blend_rnm ──────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rnm_blend_flat_with_flat_is_flat() {
        let flat = Vec3::new(0.0, 0.0, 1.0);
        let result = normal_map_blend_rnm(flat, flat);
        assert!((result - flat).length() < 1e-4, "result={result:?}");
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn rnm_blend_result_is_unit_length() {
        let n1 = Vec3::new(0.1, 0.2, 0.974).normalize();
        let n2 = Vec3::new(-0.1, 0.3, 0.948).normalize();
        let r = normal_map_blend_rnm(n1, n2);
        assert!((r.length() - 1.0).abs() < 1e-4, "len={}", r.length());
    }

    // ── ggx_d ─────────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_d_positive() {
        // NDF is always positive.
        assert!(ggx_d(1.0, 0.5) > 0.0);
        assert!(ggx_d(0.7, 0.3) > 0.0);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn ggx_d_higher_at_normal_incidence() {
        // D peaks at n_dot_h = 1.0 (specular lobe centre).
        let d_peak = ggx_d(1.0, 0.3);
        let d_off = ggx_d(0.5, 0.3);
        assert!(d_peak > d_off, "peak={d_peak} off={d_off}");
    }

    // ── smith_g_schlick_ggx ───────────────────────────────────────────────────

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smith_g_range_zero_to_one() {
        for &ndotv in &[0.01_f32, 0.1, 0.5, 0.9, 1.0] {
            let g = smith_g_schlick_ggx(ndotv, 0.5);
            assert!(g >= 0.0 && g <= 1.0, "g={g} at n_dot_v={ndotv}");
        }
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn smith_g_one_at_normal_incidence_low_roughness() {
        // At n_dot_v = 1.0 and roughness → 0, G → 1.
        let g = smith_g_schlick_ggx(1.0, 0.01);
        assert!((g - 1.0).abs() < 0.01, "g={g}");
    }
}

#[cfg(test)]
mod tests_nlerp_fix {
    use super::*;

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn test_nlerp_sum_zero() {
        let quats = vec![Quat::identity(), Quat::identity()];
        let weights = vec![0.0, 0.0];
        let q = quat_nlerp_weighted(&quats, &weights);
        assert_eq!(q.w, 1.0);
    }
}

#[cfg(test)]
mod tests_sentry {
    use super::*;
    use std::f32::NAN;

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn convex_hull_2d_nan_no_panic() {
        let points = vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, NAN),
            Vec2::new(2.0, 0.0),
        ];
        let hull = convex_hull_2d(&points);
        assert!(hull.len() <= points.len());
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    #[allow(clippy::float_cmp)]
    fn convex_hull_2d_empty_no_panic() {
        let points: Vec<Vec2> = vec![];
        let hull = convex_hull_2d(&points);
        assert!(hull.is_empty());
    }
}
