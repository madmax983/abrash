#[cfg(all(target_arch = "x86_64"))]
use abrash::math::{project_to_screen_optimized, project_triangle_to_screen, Vec3};
#[cfg(all(target_arch = "x86_64"))]
use proptest::prelude::*;

#[cfg(all(target_arch = "x86_64"))]
#[test]
fn test_simd_vs_scalar_overflow() {
    // Scenario: A point very far to the right, causing screen_x to overflow i32 positive range.
    // NDC x = 1e10 (huge).
    // Half width = 400.
    // Screen x = (1e10 + 1) * 400 = ~4e12.
    // i32::MAX is ~2e9.
    // So this overflows.

    let v = Vec3::new(1e10, 0.0, 10.0);
    let w = 1.0;
    let half_width = 400.0;
    let half_height = 300.0;

    // Scalar
    let scalar_res = project_to_screen_optimized(v, w, half_width, half_height);

    // SIMD
    // We pass 3 identical vertices to make it simple
    let (simd_res, _, _) = project_triangle_to_screen(
        v, w,
        v, w,
        v, w,
        half_width, half_height
    );

    println!("Scalar result: {:?}", scalar_res);
    println!("SIMD result:   {:?}", simd_res);

    // Scalar: saturates to i32::MAX.
    // SIMD: clamped to ~2147483520 (float precision limit below i32::MAX).

    // Both should be positive and very large.
    assert!(scalar_res.x > 2_000_000_000, "Scalar should saturate high");
    assert!(simd_res.x > 2_000_000_000, "SIMD should saturate high");
    assert_eq!(scalar_res.x.signum(), simd_res.x.signum(), "Signs should match");

    // Y should be consistent too (it might not overflow here since y is 0.0 -> screen_y = 300)
    assert_eq!(scalar_res.y, simd_res.y, "Y coordinates should match");
}

#[cfg(all(target_arch = "x86_64"))]
#[test]
fn test_simd_vs_scalar_underflow() {
    // Scenario: A point very far to the left, causing screen_x to overflow i32 negative range.
    // NDC x = -1e10.
    // Screen x = (-1e10 + 1) * 400 = ~-4e12.
    // i32::MIN is ~-2e9.

    let v = Vec3::new(-1e10, 0.0, 10.0);
    let w = 1.0;
    let half_width = 400.0;
    let half_height = 300.0;

    // Scalar
    let scalar_res = project_to_screen_optimized(v, w, half_width, half_height);

    // SIMD
    let (simd_res, _, _) = project_triangle_to_screen(
        v, w,
        v, w,
        v, w,
        half_width, half_height
    );

    println!("Scalar result: {:?}", scalar_res);
    println!("SIMD result:   {:?}", simd_res);

    // Both should saturate to i32::MIN (or i32::MIN + 1).
    // Scalar: i32::MIN + 1.
    // SIMD: i32::MIN + 1.

    assert_eq!(scalar_res.x, simd_res.x, "Scalar and SIMD x coordinates mismatch on negative overflow!");
}

#[cfg(all(target_arch = "x86_64"))]
proptest! {
    #[test]
    fn test_project_triangle_consistency(
        v0_coords in proptest::array::uniform3(-1000.0f32..1000.0),
        w0 in prop_oneof![ -10.0f32..-0.1, 0.1f32..10.0 ],
        v1_coords in proptest::array::uniform3(-1000.0f32..1000.0),
        w1 in prop_oneof![ -10.0f32..-0.1, 0.1f32..10.0 ],
        v2_coords in proptest::array::uniform3(-1000.0f32..1000.0),
        w2 in prop_oneof![ -10.0f32..-0.1, 0.1f32..10.0 ],
    ) {
        let v0 = Vec3::new(v0_coords[0], v0_coords[1], v0_coords[2]);
        let v1 = Vec3::new(v1_coords[0], v1_coords[1], v1_coords[2]);
        let v2 = Vec3::new(v2_coords[0], v2_coords[1], v2_coords[2]);

        let half_width = 400.0;
        let half_height = 300.0;

        let (simd_p0, simd_p1, simd_p2) = project_triangle_to_screen(
            v0, w0,
            v1, w1,
            v2, w2,
            half_width, half_height
        );

        let scalar_p0 = project_to_screen_optimized(v0, w0, half_width, half_height);
        let scalar_p1 = project_to_screen_optimized(v1, w1, half_width, half_height);
        let scalar_p2 = project_to_screen_optimized(v2, w2, half_width, half_height);

        // Define tolerances
        // Integer coordinates might differ by 1 due to rounding direction of rcp vs div
        let int_tolerance = 1;

        // Use relative tolerance for floats because 1/w can be large
        let rel_tolerance = 0.005; // 0.5% error allowed due to rcp approximation

        let check_float = |a: f32, b: f32, name: &str| {
            let diff = (a - b).abs();
            let max_abs = a.abs().max(b.abs()).max(1.0);
            prop_assert!(diff <= rel_tolerance * max_abs, "{}: SIMD {} vs Scalar {} (diff: {}, limit: {})", name, a, b, diff, rel_tolerance * max_abs);
            Ok(())
        };

        // Check P0
        prop_assert!((simd_p0.x - scalar_p0.x).abs() <= int_tolerance, "P0.x mismatch: SIMD {} vs Scalar {}", simd_p0.x, scalar_p0.x);
        prop_assert!((simd_p0.y - scalar_p0.y).abs() <= int_tolerance, "P0.y mismatch: SIMD {} vs Scalar {}", simd_p0.y, scalar_p0.y);
        check_float(simd_p0.z, scalar_p0.z, "P0.z")?;
        check_float(simd_p0.inv_w, scalar_p0.inv_w, "P0.inv_w")?;

        // Check P1
        prop_assert!((simd_p1.x - scalar_p1.x).abs() <= int_tolerance, "P1.x mismatch");
        prop_assert!((simd_p1.y - scalar_p1.y).abs() <= int_tolerance, "P1.y mismatch");
        check_float(simd_p1.z, scalar_p1.z, "P1.z")?;
        check_float(simd_p1.inv_w, scalar_p1.inv_w, "P1.inv_w")?;

        // Check P2
        prop_assert!((simd_p2.x - scalar_p2.x).abs() <= int_tolerance, "P2.x mismatch");
        prop_assert!((simd_p2.y - scalar_p2.y).abs() <= int_tolerance, "P2.y mismatch");
        check_float(simd_p2.z, scalar_p2.z, "P2.z")?;
        check_float(simd_p2.inv_w, scalar_p2.inv_w, "P2.inv_w")?;
    }
}
