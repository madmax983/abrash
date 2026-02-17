use abrash::obj_loader::load_obj;
use abrash::math::{Vec3, project_triangle_to_screen, project_to_screen_optimized};
use proptest::prelude::*;

proptest! {
    // Fuzz Target 1: Throw random strings at the OBJ loader to find panics
    #[test]
    fn crash_test_obj_loader(s in "\\PC*") {
        // We don't care about the result, only that it doesn't panic
        let _ = load_obj(&s);
    }

    // Fuzz Target 2: Verify consistency between Scalar and SIMD projection logic
    // This hunts for differences in NaN/Inf handling and rounding modes
    #[test]
    fn test_projection_consistency(
        v0_x in any::<f32>(), v0_y in any::<f32>(), v0_z in any::<f32>(), w0 in any::<f32>(),
        v1_x in any::<f32>(), v1_y in any::<f32>(), v1_z in any::<f32>(), w1 in any::<f32>(),
        v2_x in any::<f32>(), v2_y in any::<f32>(), v2_z in any::<f32>(), w2 in any::<f32>(),
    ) {
        let half_width = 400.0;
        let half_height = 300.0;

        let v0 = Vec3::new(v0_x, v0_y, v0_z);
        let v1 = Vec3::new(v1_x, v1_y, v1_z);
        let v2 = Vec3::new(v2_x, v2_y, v2_z);

        // SIMD path (on x86_64) - projects 3 vertices at once
        let (s0, s1, s2) = project_triangle_to_screen(v0, w0, v1, w1, v2, w2, half_width, half_height);

        // Scalar path - projects vertices individually
        // This acts as our "oracle" (though strictly speaking, scalar isn't always right either, but they MUST match)
        let c0 = project_to_screen_optimized(v0, w0, half_width, half_height);
        let c1 = project_to_screen_optimized(v1, w1, half_width, half_height);
        let c2 = project_to_screen_optimized(v2, w2, half_width, half_height);

        // Define a relaxed comparison for float-derived integers
        // SIMD and Scalar might differ by 1 due to rounding modes or order of operations
        // But for NaN/Inf, they might differ wildly (e.g. 0 vs MIN_INT).
        let check = |s_val: i32, c_val: i32, component: &str, v_idx: usize| {
            // Exact match required? Or allow +/- 1?
            // Let's require exact match first to see what breaks.
            // If legitimate float rounding diffs occur, we relax.
             prop_assert_eq!(s_val, c_val, "{} mismatch for v{}: SIMD={} Scalar={}", component, v_idx, s_val, c_val);
             Ok(())
        };

        check(s0.x, c0.x, "X", 0)?;
        check(s0.y, c0.y, "Y", 0)?;

        check(s1.x, c1.x, "X", 1)?;
        check(s1.y, c1.y, "Y", 1)?;

        check(s2.x, c2.x, "X", 2)?;
        check(s2.y, c2.y, "Y", 2)?;
    }
}
