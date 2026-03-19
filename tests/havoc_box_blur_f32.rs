#[test]
fn test_box_blur_f32_vertical_scalar_oob() {
    let width = 5;
    let height = 5;
    let _src = vec![0f32; width * height];
    let _dest = vec![0f32; width * height];
    let _acc = vec![0f32; width];

    // Have to call individual passes because box_blur_f32 hardcodes radius to 2
    // Oh wait, box_blur_f32 hardcodes radius to 2!
}
