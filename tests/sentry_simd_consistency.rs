use abrash::math::{
    Mat4, ScreenPoint, Vec3, project_to_screen_optimized, project_triangle_to_screen,
};
use proptest::prelude::*;
use std::mem::MaybeUninit;

// Pure scalar implementation for reference
fn transform_point_scalar(m: &Mat4, v: Vec3) -> (Vec3, f32) {
    let x = m.m[0][0] * v.x + m.m[1][0] * v.y + m.m[2][0] * v.z + m.m[3][0];
    let y = m.m[0][1] * v.x + m.m[1][1] * v.y + m.m[2][1] * v.z + m.m[3][1];
    let z = m.m[0][2] * v.x + m.m[1][2] * v.y + m.m[2][2] * v.z + m.m[3][2];
    let w = m.m[0][3] * v.x + m.m[1][3] * v.y + m.m[2][3] * v.z + m.m[3][3];
    (Vec3::new(x, y, z), w)
}

proptest! {
    #[test]
    fn fuzz_project_triangle_to_screen(
        vx0 in -1000.0f32..1000.0, vy0 in -1000.0f32..1000.0, vz0 in -1000.0f32..1000.0, w0 in 0.1f32..100.0,
        vx1 in -1000.0f32..1000.0, vy1 in -1000.0f32..1000.0, vz1 in -1000.0f32..1000.0, w1 in 0.1f32..100.0,
        vx2 in -1000.0f32..1000.0, vy2 in -1000.0f32..1000.0, vz2 in -1000.0f32..1000.0, w2 in 0.1f32..100.0,
        hw in 100.0f32..2000.0, hh in 100.0f32..2000.0
    ) {
        let v0 = Vec3::new(vx0, vy0, vz0);
        let v1 = Vec3::new(vx1, vy1, vz1);
        let v2 = Vec3::new(vx2, vy2, vz2);

        // Run SIMD (if available)
        let (s0, s1, s2) = project_triangle_to_screen(v0, w0, v1, w1, v2, w2, hw, hh);

        // Run Scalar (project_to_screen_optimized is scalar)
        let p0 = project_to_screen_optimized(v0, w0, hw, hh);
        let p1 = project_to_screen_optimized(v1, w1, hw, hh);
        let p2 = project_to_screen_optimized(v2, w2, hw, hh);

        // Compare
        // We use a custom comparison because the SIMD implementation uses approximate reciprocal (rcp)
        // which can result in slightly different float values for inv_w, and thus slight differences in x/y.
        let compare_screen_point = |s: ScreenPoint, p: ScreenPoint, name: &str| {
             // Allow +/- 1 pixel difference due to float precision differences
            assert!((s.x - p.x).abs() <= 1, "{}: x mismatch: {} vs {}", name, s.x, p.x);
            assert!((s.y - p.y).abs() <= 1, "{}: y mismatch: {} vs {}", name, s.y, p.y);

            // Allow epsilon for float fields (approx reciprocal error is around 1.5*2^-12 ~ 0.0003 without Newton-Raphson)
            // With NR it's much better but still not exact.
            let eps = 1e-3;
            assert!((s.z - p.z).abs() < eps, "{}: z mismatch: {} vs {}", name, s.z, p.z);

            // For inv_w, the error can be relative to the magnitude.
            // If inv_w is large (small w), error can be larger.
            if s.inv_w.abs() > 1.0 {
                 let rel_diff = (s.inv_w - p.inv_w).abs() / s.inv_w.abs().max(p.inv_w.abs());
                 assert!(rel_diff < eps, "{}: inv_w relative mismatch: {} vs {} (diff {})", name, s.inv_w, p.inv_w, rel_diff);
            } else {
                 assert!((s.inv_w - p.inv_w).abs() < eps, "{}: inv_w mismatch: {} vs {}", name, s.inv_w, p.inv_w);
            }
        };

        compare_screen_point(s0, p0, "Point 0");
        compare_screen_point(s1, p1, "Point 1");
        compare_screen_point(s2, p2, "Point 2");
    }

    #[test]
    fn fuzz_transform_points(
        m00 in any::<f32>(), m01 in any::<f32>(), m02 in any::<f32>(), m03 in any::<f32>(),
        m10 in any::<f32>(), m11 in any::<f32>(), m12 in any::<f32>(), m13 in any::<f32>(),
        m20 in any::<f32>(), m21 in any::<f32>(), m22 in any::<f32>(), m23 in any::<f32>(),
        m30 in any::<f32>(), m31 in any::<f32>(), m32 in any::<f32>(), m33 in any::<f32>(),
        // Generate a batch of points
        ref points_vec in prop::collection::vec(
            (any::<f32>(), any::<f32>(), any::<f32>()),
            1..32 // Small batch size to test tail handling and alignment
        )
    ) {
        let m = Mat4 {
            m: [
                [m00, m01, m02, m03],
                [m10, m11, m12, m13],
                [m20, m21, m22, m23],
                [m30, m31, m32, m33],
            ]
        };

        let points: Vec<Vec3> = points_vec.iter().map(|&(x, y, z)| Vec3::new(x, y, z)).collect();
        let mut output_simd: Vec<MaybeUninit<(Vec3, f32)>> = Vec::with_capacity(points.len());
        unsafe { output_simd.set_len(points.len()); }

        // Run SIMD (AVX2 if available)
        m.transform_points_uninit(&points, &mut output_simd);

        // Verify against Scalar
        for (i, p) in points.iter().enumerate() {
            let (res_simd, w_simd) = unsafe { output_simd[i].assume_init() };
            let (res_scalar, w_scalar) = transform_point_scalar(&m, *p);

            // Using loose epsilon because SIMD vs Scalar float reordering can cause differences
            // especially with large numbers or near zero.
            // Also if inputs are NaN/Inf, results should match (both NaN or both Inf).
            // But strict equality on NaN fails.

            let eps = 1e-3;

            // Helper to check closeness
            let check = |a: f32, b: f32, name: &str| {
                if a.is_nan() {
                    assert!(b.is_nan(), "{name} mismatch: SIMD=NaN, Scalar={b}");
                } else if a.is_infinite() {
                    assert_eq!(a, b, "{name} mismatch: SIMD={a}, Scalar={b}");
                } else {
                    let diff = (a - b).abs();
                    // Relative error for large numbers
                    let max_abs = a.abs().max(b.abs());
                    if max_abs > 1.0 {
                         assert!(diff / max_abs < eps, "{} mismatch: {} vs {} (rel diff {})", name, a, b, diff/max_abs);
                    } else {
                         assert!(diff < eps, "{name} mismatch: {a} vs {b}");
                    }
                }
            };

            check(res_simd.x, res_scalar.x, "X");
            check(res_simd.y, res_scalar.y, "Y");
            check(res_simd.z, res_scalar.z, "Z");
            check(w_simd, w_scalar, "W");
        }
    }
}
