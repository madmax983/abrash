use abrash::post_process::blur::box_blur_horizontal;

#[test]
fn test_box_blur_oom() {
    let mut src = vec![0u32; 1];
    let mut dest = vec![0u32; 1];

    // Pass enormous width/height -> multiplication might overflow or loop might take forever?
    // Wait, width is just an argument, the function iterates over height, but bounds checks might fail.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_horizontal(&src, &mut dest, 1_000_000, 1_000_000, 10);
    }));
    // Expect panic since 1000000 > 1
    assert!(result.is_err());
}
