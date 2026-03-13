use abrash::post_process::box_blur_horizontal;

#[test]
fn test_box_blur_api_validation() {
    // Tests API surface where it might panic due to mismatched array lengths
    let width = 100usize;
    let height = 100usize;
    let radius = 10u32;
    let src = vec![0u32; width * height - 1]; // Incorrect length
    let mut dest = vec![0u32; width * height];

    // We expect this to panic. If it does, we succeeded.
    // If we wanted to "fix" it, we'd add bounds checking. But Havoc just breaks it.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_horizontal(&src, &mut dest, width, height, radius);
    }));
    assert!(
        result.is_err(),
        "Expected panic due to OOB buffer size mismatch"
    );
}
