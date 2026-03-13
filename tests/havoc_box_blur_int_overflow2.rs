use abrash::post_process::box_blur_horizontal;

#[test]
fn test_box_blur_int_overflow2() {
    let width = 5;
    let height = 5;
    // Max u32 value for pixels
    let src = vec![0xFFFFFFFFu32; width * height];
    let mut dest = vec![0u32; width * height];

    // radius controls how many items are added to accumulator
    let radius = 10_000_000u32; // adding 10 million 255s = 2.5 billion, overflows i32!

    // Call it
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_horizontal(&src, &mut dest, width, height, radius);
    }));

    // Expect it NOT to panic now because of capping
    assert!(result.is_ok());
}
