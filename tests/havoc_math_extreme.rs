use abrash::math::{Mat4, Vec3, project_to_screen_optimized, project_triangle_to_screen};
use proptest::prelude::*;

// Define strategies for Vec3 and Mat4 using ANY float (NaN, Inf, Subnormal)
prop_compose! {
    fn arb_vec3()(x in any::<f32>(), y in any::<f32>(), z in any::<f32>()) -> Vec3 {
        Vec3::new(x, y, z)
    }
}

prop_compose! {
    fn arb_mat4()(
        m00 in any::<f32>(), m01 in any::<f32>(), m02 in any::<f32>(), m03 in any::<f32>(),
        m10 in any::<f32>(), m11 in any::<f32>(), m12 in any::<f32>(), m13 in any::<f32>(),
        m20 in any::<f32>(), m21 in any::<f32>(), m22 in any::<f32>(), m23 in any::<f32>(),
        m30 in any::<f32>(), m31 in any::<f32>(), m32 in any::<f32>(), m33 in any::<f32>()
    ) -> Mat4 {
        Mat4 {
            m: [
                [m00, m01, m02, m03],
                [m10, m11, m12, m13],
                [m20, m21, m22, m23],
                [m30, m31, m32, m33],
            ]
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_transform_point_extreme(m in arb_mat4(), v in arb_vec3()) {
        // Just verify it doesn't panic/segfault
        let _ = m.transform_point(v);
    }

    #[test]
    fn test_transform_points_batch_extreme(
        m in arb_mat4(),
        points in proptest::collection::vec(arb_vec3(), 0..100)
    ) {
        let mut output = vec![(Vec3::default(), 0.0); points.len()];
        // Should use SIMD path if available and likely panic if unsafe blocks are fragile
        m.transform_points(&points, &mut output);
    }

    #[test]
    fn test_project_to_screen_extreme(
        v in arb_vec3(),
        w in any::<f32>(),
        hw in any::<f32>(),
        hh in any::<f32>()
    ) {
        // project_to_screen_optimized uses i32 conversion which can panic on overflow if not careful
        // or UB if casting non-finite float to int (though Rust defines it as saturating usually, or specifically UB for `as` in old versions?
        // In Rust 1.45+ `as` is saturating. But let's see.)
        let _ = project_to_screen_optimized(v, w, hw, hh);
    }

    #[test]
    fn test_project_triangle_to_screen_extreme(
        v0 in arb_vec3(), w0 in any::<f32>(),
        v1 in arb_vec3(), w1 in any::<f32>(),
        v2 in arb_vec3(), w2 in any::<f32>(),
        hw in any::<f32>(), hh in any::<f32>()
    ) {
        // This hits the SIMD path project_triangle_to_screen
        let _ = project_triangle_to_screen(v0, w0, v1, w1, v2, w2, hw, hh);
    }
}
