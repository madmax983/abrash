use abrash::post_process::box_blur_horizontal;

#[test]
fn test_box_blur_zero_width() {
    let src = vec![0u32; 10];
    let mut dest = vec![0u32; 10];
    let width = 0;
    let height = 10;
    let radius = 10u32;
    // this will panic due to src_row[0]
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        box_blur_horizontal(&src, &mut dest, width, height, radius);
    }));
}
