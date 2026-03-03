use abrash::post_process::blur::box_blur_horizontal;

#[test]
fn test_box_blur_int_overflow() {
    let width = 5;
    let height = 5;
    // Max u32 value for pixels
    let src = vec![0xFFFFFFFFu32; width * height];
    let mut dest = vec![0u32; width * height];
    // radius controls how many items are added to accumulator
    let radius = 1000u32;

    // Call it
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_horizontal(&src, &mut dest, width, height, radius);
    }));

    // Expect it to pass without panicking
    assert!(result.is_ok());
}
