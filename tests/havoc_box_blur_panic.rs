use abrash::post_process::box_blur_horizontal;

#[test]
fn test_box_blur_zero_width_height() {
    // Zero width/height with large radius
    let src = vec![];
    let mut dest = vec![];
    let width = 0;
    let height = 0;
    let radius = 10u32;
    box_blur_horizontal(&src, &mut dest, width, height, radius);
}
