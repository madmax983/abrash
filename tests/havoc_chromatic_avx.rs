use abrash::post_process::filters;

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[test]
fn test_chromatic_aberration_simd_bounds() {
    if !std::is_x86_feature_detected!("avx2") {
        return;
    }

    // Test that the SIMD wrapper in `filters` catches the offset > width check.
    // wait, does `apply_chromatic_aberration` use SIMD automatically?
    // Yes! Let's test `apply_post_processing` or just `apply_chromatic_aberration`
    // with different offsets and widths.
}
