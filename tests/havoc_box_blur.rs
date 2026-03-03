#[test]
fn test_box_blur_horizontal_large_radius_dos() {
    let width = 10;
    let height = 1;
    let radius = 1000000000u32;
    let src = vec![0u32; width * height];
    let mut dest = vec![0u32; width * height];

    // This will likely take extremely long because of the pre-fill loop
    // for _ in 0..=radius { r_acc += r_first; ... }
    abrash::post_process::blur::box_blur_horizontal(&src, &mut dest, width, height, radius);
}
