use abrash::post_process::blur::{box_blur_horizontal, box_blur_vertical};

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

#[test]
#[should_panic]
fn test_box_blur_horizontal_out_of_bounds() {
    // Width and height > 0, but source array is smaller than width * height
    let src = vec![0; 5]; // Only 5 elements
    let mut dest = vec![0; 100];
    let width = 10;
    let height = 10;
    let radius = 1u32;

    // Should panic when trying to read out of bounds from `src`
    box_blur_horizontal(&src, &mut dest, width, height, radius);
}

#[test]
#[should_panic]
fn test_box_blur_vertical_out_of_bounds() {
    // Width and height > 0, but dest array is smaller than width * height
    let src = vec![0; 100];
    let mut dest = vec![0; 5]; // Only 5 elements
    let mut acc = vec![0; 300]; // 3 * width
    let width = 10;
    let height = 10;
    let radius = 1u32;

    // Should panic when trying to write out of bounds to `dest`
    box_blur_vertical(&src, &mut dest, &mut acc, width, height, radius);
}
