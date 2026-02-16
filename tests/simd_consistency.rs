#[cfg(all(target_arch = "x86_64"))]
use abrash::math::{Vec3, project_to_screen_optimized, project_triangle_to_screen};

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
    let (simd_res, _, _) = project_triangle_to_screen(v, w, v, w, v, w, half_width, half_height);

    println!("Scalar result: {:?}", scalar_res);
    println!("SIMD result:   {:?}", simd_res);

    // Scalar: saturates to i32::MAX.
    // SIMD: clamped to ~2147483520 (float precision limit below i32::MAX).

    // Both should be positive and very large.
    assert!(scalar_res.x > 2_000_000_000, "Scalar should saturate high");
    assert!(simd_res.x > 2_000_000_000, "SIMD should saturate high");
    assert_eq!(
        scalar_res.x.signum(),
        simd_res.x.signum(),
        "Signs should match"
    );

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
    let (simd_res, _, _) = project_triangle_to_screen(v, w, v, w, v, w, half_width, half_height);

    println!("Scalar result: {:?}", scalar_res);
    println!("SIMD result:   {:?}", simd_res);

    // Both should saturate to i32::MIN (or i32::MIN + 1).
    // Scalar: i32::MIN + 1.
    // SIMD: i32::MIN + 1.

    assert_eq!(
        scalar_res.x, simd_res.x,
        "Scalar and SIMD x coordinates mismatch on negative overflow!"
    );
}
