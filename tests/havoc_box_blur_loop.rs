use abrash::post_process::blur::box_blur_horizontal;

#[test]
fn test_box_blur_horizontal_large_radius() {
    let width = 5;
    let height = 5;
    let src = vec![0u32; width * height];
    let mut dest = vec![0u32; width * height];
    let radius = 20u32;

    // Call it
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_horizontal(&src, &mut dest, width, height, radius);
    }));

    // Expect it to pass without panicking
    assert!(result.is_ok());
}

#[test]
fn test_box_blur_vertical_large_radius() {
    let width = 5;
    let height = 5;
    let src = vec![0u32; width * height];
    let mut dest = vec![0u32; width * height];
    let mut acc = vec![0i32; width * 3];
    let radius = 20u32;

    // Call it
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        abrash::post_process::blur::box_blur_vertical(
            &src, &mut dest, &mut acc, width, height, radius,
        );
    }));

    // Expect it to pass without panicking
    assert!(result.is_ok());
}
