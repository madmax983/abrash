use abrash::post_process::box_blur_vertical;

#[test]
fn test_box_blur_vertical_avx2_int_overflow() {
    let width = 5;
    let height = 5;
    // Max u32 value for pixels
    let src = vec![0xFFFFFFFFu32; width * height];
    let mut dest = vec![0u32; width * height];
    let mut acc = vec![0i32; width * 3];
    let radius = 10_000_000u32;

    // Call it, enabling AVX2 via unsafe if supported
    // But box_blur_vertical handles that for us automatically now!
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_vertical(&src, &mut dest, &mut acc, width, height, radius);
    }));

    // Expect it NOT to panic now
    assert!(result.is_ok());
}
